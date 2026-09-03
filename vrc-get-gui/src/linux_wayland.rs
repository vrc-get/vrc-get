use std::io;
use std::path::Path;

use crate::os::BringUnityToFrontResult;

pub(super) fn activate(project_path: &Path, pids: &[u32]) -> io::Result<BringUnityToFrontResult> {
    match wlr::activate(project_path, pids) {
        Ok(BringUnityToFrontResult::BroughtToFront) => {
            return Ok(BringUnityToFrontResult::BroughtToFront);
        }
        Ok(_) => {}
        Err(e) => log::debug!("wlr_foreign_toplevel activation failed: {e}"),
    }

    match xdg::activate(project_path, pids) {
        Ok(BringUnityToFrontResult::BroughtToFront) => {
            return Ok(BringUnityToFrontResult::BroughtToFront);
        }
        Ok(_) => {}
        Err(e) => log::debug!("xdg_activation activation failed: {e}"),
    }

    Ok(BringUnityToFrontResult::WindowNotFound)
}

mod wlr {
    use std::io;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use wayland_client::protocol::{wl_registry, wl_seat};
    use wayland_client::{
        Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    };
    use wayland_protocols_wlr::foreign_toplevel::v1::client::{
        zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1,
    };

    use crate::os::BringUnityToFrontResult;

    const TOPLEVEL_TIMEOUT: Duration = Duration::from_secs(1);

    pub(super) fn activate(
        project_path: &Path,
        _pids: &[u32],
    ) -> io::Result<BringUnityToFrontResult> {
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let conn = Connection::connect_to_env()
            .map_err(|e| io::Error::other(format!("Wayland connect: {e}")))?;

        let display = conn.display();
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();

        let mut state = State::default();
        display.get_registry(&qh, ());

        roundtrip(&mut queue, &mut state)?;

        if state.manager.is_none() {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        }

        let deadline = Instant::now() + TOPLEVEL_TIMEOUT;
        loop {
            roundtrip(&mut queue, &mut state)?;
            if state.all_toplevels_done() || Instant::now() >= deadline {
                break;
            }
        }

        let target = state.toplevels.iter().find(|t| {
            let data = t.data::<Mutex<ToplevelData>>().unwrap();
            let data = data.lock().unwrap();
            data.done
                && data.app_id.eq_ignore_ascii_case("Unity")
                && data.title.contains(project_name)
        });

        let Some(target) = target else {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        };

        let Some(seat) = &state.seat else {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        };

        target.activate(seat);
        roundtrip(&mut queue, &mut state)?;

        log::info!("Activated with: wlr_foreign_toplevel");
        Ok(BringUnityToFrontResult::BroughtToFront)
    }

    fn roundtrip(
        queue: &mut wayland_client::EventQueue<State>,
        state: &mut State,
    ) -> io::Result<()> {
        queue
            .roundtrip(state)
            .map_err(|e| io::Error::other(format!("Wayland roundtrip: {e}")))?;
        Ok(())
    }

    #[derive(Default)]
    struct State {
        manager: Option<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1>,
        seat: Option<wl_seat::WlSeat>,
        toplevels: Vec<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1>,
        done_count: usize,
    }

    impl State {
        fn all_toplevels_done(&self) -> bool {
            !self.toplevels.is_empty() && self.done_count >= self.toplevels.len()
        }
    }

    #[derive(Default)]
    struct ToplevelData {
        title: String,
        app_id: String,
        done: bool,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(
            state: &mut Self,
            registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            {
                match interface.as_str() {
                    "zwlr_foreign_toplevel_manager_v1" => {
                        state.manager = Some(registry.bind(name, version.min(3), qh, ()));
                    }
                    "wl_seat" if state.seat.is_none() => {
                        state.seat = Some(registry.bind(name, version.min(1), qh, ()));
                    }
                    _ => {}
                }
            }
        }
    }

    impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
            event: zwlr_foreign_toplevel_manager_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
                state.toplevels.push(toplevel);
            }
        }

        event_created_child!(State, zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, [
            zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE =>
                (zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, Mutex::new(ToplevelData::default())),
        ]);
    }

    impl Dispatch<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, Mutex<ToplevelData>>
        for State
    {
        fn event(
            state: &mut Self,
            _: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
            event: zwlr_foreign_toplevel_handle_v1::Event,
            data: &Mutex<ToplevelData>,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let mut data = data.lock().unwrap();
            match event {
                zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                    data.title = title;
                }
                zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                    data.app_id = app_id;
                }
                zwlr_foreign_toplevel_handle_v1::Event::Done if !data.done => {
                    data.done = true;
                    state.done_count += 1;
                }
                _ => {}
            }
        }
    }

    delegate_noop!(State: ignore wl_seat::WlSeat);
}

// xdg_activation_v1 (standard protocol, requires target app cooperation).
// Currently always returns WindowNotFound because Unity does not handle
// activation tokens. Kept for future use when Unity supports native Wayland.
mod xdg {
    use std::io;
    use std::path::Path;
    use std::sync::Mutex;

    use wayland_client::protocol::wl_registry;
    use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
    use wayland_protocols::xdg::activation::v1::client::{
        xdg_activation_token_v1, xdg_activation_v1,
    };

    use crate::os::BringUnityToFrontResult;

    pub(super) fn activate(
        _project_path: &Path,
        _pids: &[u32],
    ) -> io::Result<BringUnityToFrontResult> {
        let conn = Connection::connect_to_env()
            .map_err(|e| io::Error::other(format!("Wayland connect: {e}")))?;

        let display = conn.display();
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();

        let mut state = State::default();
        display.get_registry(&qh, ());

        roundtrip(&mut queue, &mut state)?;

        let Some(activation) = &state.activation else {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        };

        let token_obj = activation.get_activation_token(&qh, ());
        token_obj.set_app_id("ALCOM".to_string());
        token_obj.commit();

        roundtrip(&mut queue, &mut state)?;

        let Some(_token) = state.token.lock().unwrap().take() else {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        };

        // TODO: pass the token to Unity via IPC and have Unity call
        // xdg_activation_v1.activate(token, unity_surface).
        Ok(BringUnityToFrontResult::WindowNotFound)
    }

    fn roundtrip(
        queue: &mut wayland_client::EventQueue<State>,
        state: &mut State,
    ) -> io::Result<()> {
        queue
            .roundtrip(state)
            .map_err(|e| io::Error::other(format!("Wayland roundtrip: {e}")))?;
        Ok(())
    }

    #[derive(Default)]
    struct State {
        activation: Option<xdg_activation_v1::XdgActivationV1>,
        token: Mutex<Option<String>>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(
            state: &mut Self,
            registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
                && interface == "xdg_activation_v1"
            {
                state.activation = Some(registry.bind(name, version.min(1), qh, ()));
            }
        }
    }

    impl Dispatch<xdg_activation_v1::XdgActivationV1, ()> for State {
        fn event(
            _: &mut Self,
            _: &xdg_activation_v1::XdgActivationV1,
            _: xdg_activation_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &xdg_activation_token_v1::XdgActivationTokenV1,
            event: xdg_activation_token_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let xdg_activation_token_v1::Event::Done { token } = event {
                *state.token.lock().unwrap() = Some(token);
            }
        }
    }

    delegate_noop!(State: ignore wayland_client::protocol::wl_callback::WlCallback);
}
