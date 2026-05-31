use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

enum Message {
    Task(BoxFuture),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeClosed;

impl std::fmt::Display for BridgeClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tokio bridge is closed")
    }
}

impl std::error::Error for BridgeClosed {}

#[derive(Clone)]
pub struct TokioBridge {
    sender: mpsc::UnboundedSender<Message>,
    closed: Arc<AtomicBool>,
}

impl TokioBridge {
    pub fn new(thread_name: &'static str) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let closed = Arc::new(AtomicBool::new(false));
        let closed_for_thread = closed.clone();

        thread::Builder::new()
            .name(thread_name.to_owned())
            .spawn(move || {
                let runtime = Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .expect("building tokio bridge runtime");

                runtime.block_on(async move {
                    while let Some(message) = receiver.recv().await {
                        match message {
                            Message::Task(task) => {
                                tokio::spawn(task);
                            }
                            Message::Shutdown => break,
                        }
                    }
                    closed_for_thread.store(true, Ordering::Release);
                });
            })
            .expect("spawning tokio bridge thread");

        Self { sender, closed }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn spawn<Fut>(&self, task: Fut) -> Result<(), BridgeClosed>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.sender
            .send(Message::Task(Box::pin(task)))
            .map_err(|_| BridgeClosed)
    }

    pub fn call<Fut, T>(&self, task: Fut) -> Result<oneshot::Receiver<T>, BridgeClosed>
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        self.spawn(async move {
            let _ = sender.send(task.await);
        })?;
        Ok(receiver)
    }

    pub fn shutdown(&self) -> Result<(), BridgeClosed> {
        self.sender
            .send(Message::Shutdown)
            .map_err(|_| BridgeClosed)
    }
}

#[cfg(test)]
mod tests {
    use super::TokioBridge;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test(flavor = "current_thread")]
    async fn call_returns_result() {
        let bridge = TokioBridge::new("test-runtime");

        let receiver = bridge.call(async { 40 + 2 }).unwrap();
        assert_eq!(receiver.await.unwrap(), 42);

        bridge.shutdown().unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn spawn_runs_in_background() {
        let bridge = TokioBridge::new("test-runtime-bg");

        let receiver = bridge
            .call(async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                "done"
            })
            .unwrap();

        assert_eq!(
            timeout(Duration::from_secs(1), receiver)
                .await
                .unwrap()
                .unwrap(),
            "done"
        );

        bridge.shutdown().unwrap();
    }
}
