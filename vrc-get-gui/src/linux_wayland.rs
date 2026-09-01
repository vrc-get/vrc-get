// Wayland window activation via xdg_activation_v1 (standard protocol).
//
// xdg_activation_v1 is a token-based protocol: the requesting app (ALCOM)
// obtains a token from the compositor, then the target app (Unity) must call
// activate(token, its_own_surface) to raise itself. This requires cooperation
// from the target app.
//
// Currently disabled (USE_WAYLAND_ACTIVATION = false) because Unity Editor
// does not support Wayland natively and therefore cannot handle activation
// tokens. When Unity gains native Wayland support, this path can be enabled
// and an IPC mechanism added to pass the token to Unity.

use std::io;
use std::path::Path;
use std::sync::Mutex;

use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::xdg::activation::v1::client::{
    xdg_activation_token_v1, xdg_activation_v1,
};

use crate::os::BringUnityToFrontResult;

pub(super) fn activate(_project_path: &Path, _pids: &[u32]) -> io::Result<BringUnityToFrontResult> {
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

    // Request a token. ALCOM has focus when the user clicks the button,
    // so the compositor should grant a valid token.
    let token_obj = activation.get_activation_token(&qh, ());
    token_obj.set_app_id("ALCOM".to_string());
    token_obj.commit();

    roundtrip(&mut queue, &mut state)?;

    let Some(_token) = state.token.lock().unwrap().take() else {
        return Ok(BringUnityToFrontResult::WindowNotFound);
    };

    // TODO: pass the token to Unity via IPC and have Unity call
    // xdg_activation_v1.activate(token, unity_surface).
    // Until Unity supports this, we cannot complete the activation.
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
