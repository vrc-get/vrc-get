use crate::commands::RustError;
use log::error;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use tauri::{EventHandler, Window};

#[derive(Serialize, specta::Type)]
#[serde(tag = "type")]
pub(crate) enum AsyncCallResult<P, R> {
    Result {
        value: R,
    },
    Started {},
    #[allow(unused)]
    UnusedProgress {
        progress: P,
    },
}

pub(crate) enum ImplResult<R, AsyncFn> {
    Immediate(R),
    Async(AsyncFn),
}

#[derive(Clone)]
pub(crate) struct AsyncCommandContext<P> {
    channel: String,
    window: Window,
    _progress: PhantomData<P>,
}

impl<P: Serialize + Clone> AsyncCommandContext<P> {
    pub(crate) fn emit(&self, value: P) -> Result<(), tauri::Error> {
        match self.window.emit(&self.channel, value) {
            Err(tauri::Error::WebviewNotFound) => Ok(()),
            Err(e) => Err(e),
            Ok(()) => Ok(()),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "value")]
enum FinishedMessage<R> {
    Success(R),
    Failed(RustError),
}

pub(crate) async fn async_command<P, R, AsyncFn, AsyncFnFut>(
    channel: String,
    window: Window,
    body: impl Future<Output = Result<ImplResult<R, AsyncFn>, RustError>>,
) -> Result<AsyncCallResult<P, R>, RustError>
where
    AsyncFn: FnOnce(AsyncCommandContext<P>) -> AsyncFnFut + Send + 'static,
    AsyncFnFut: Future<Output = Result<R, RustError>> + Send,
    P: Serialize + Clone,
    R: Serialize + Clone,
{
    let async_fn = match body.await? {
        ImplResult::Immediate(value) => return Ok(AsyncCallResult::Result { value }),
        ImplResult::Async(async_fn) => async_fn,
    };

    let event_handler_slot = Arc::new(Mutex::<Option<EventHandler>>::new(None));

    let window_1 = window.clone();
    let window_2 = window.clone();
    let channel_1 = channel.clone();

    let handle = tokio::spawn(async move {
        let context = AsyncCommandContext {
            channel: format!("{}:progress", channel),
            window: window.clone(),
            _progress: PhantomData,
        };
        let message = match async_fn(context).await {
            Ok(value) => FinishedMessage::Success(value),
            Err(value) => FinishedMessage::Failed(value),
        };

        if let Err(e) = window.emit(&format!("{}:finished", channel), message) {
            match e {
                tauri::Error::WebviewNotFound => {}
                _ => error!("error sending stdout: {e}"),
            }
        }
    });

    *event_handler_slot.lock().unwrap() =
        Some(window_2.listen(format!("{channel_1}:cancel"), move |_| {
            window_1.emit(&format!("{channel_1}:cancelled"), ()).ok();
            handle.abort();
        }));

    Ok(AsyncCallResult::Started {})
}

#[allow(dead_code)]
pub(crate) fn immediate<R, AsyncFn>(value: R) -> Result<ImplResult<R, AsyncFn>, RustError> {
    Ok(ImplResult::Immediate(value))
}

pub(crate) struct With<P>(PhantomData<P>);

impl<P> With<P> {
    pub(crate) fn continue_async<R, AsyncFn, AsyncFnResult>(
        f: AsyncFn,
    ) -> Result<ImplResult<R, AsyncFn>, RustError>
    where
        AsyncFn: FnOnce(AsyncCommandContext<P>) -> AsyncFnResult,
        P: Serialize + Clone,
        R: Serialize + Clone,
    {
        Ok(ImplResult::Async(f))
    }
}
