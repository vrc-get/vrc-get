use std::io;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _,
    CreateWindowAux, EventMask, InputFocus, PropMode, StackMode, Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use crate::os::BringUnityToFrontResult;

pub(super) fn activate(pids: &[u32]) -> io::Result<BringUnityToFrontResult> {
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| io::Error::other(format!("X11 connect: {e}")))?;

    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let net_client_list = intern_atom(&conn, b"_NET_CLIENT_LIST")?;
    let net_wm_pid = intern_atom(&conn, b"_NET_WM_PID")?;
    let net_active_window = intern_atom(&conn, b"_NET_ACTIVE_WINDOW")?;

    let client_list = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, u32::MAX)
        .map_err(|e| io::Error::other(format!("get _NET_CLIENT_LIST: {e}")))?
        .reply()
        .map_err(|e| io::Error::other(format!("get _NET_CLIENT_LIST reply: {e}")))?;

    let Some(windows) = client_list.value32() else {
        return Ok(BringUnityToFrontResult::WindowNotFound);
    };
    let windows: Vec<Window> = windows.collect();

    let mut target = None;
    for &window in &windows {
        let pid_prop = conn
            .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .map_err(|e| io::Error::other(format!("get _NET_WM_PID: {e}")))?
            .reply()
            .map_err(|e| io::Error::other(format!("get _NET_WM_PID reply: {e}")))?;

        let Some(pid) = pid_prop.value32().and_then(|mut v| v.next()) else {
            continue;
        };

        if pids.contains(&pid) {
            target = Some(window);
            break;
        }
    }

    let Some(target) = target else {
        return Ok(BringUnityToFrontResult::WindowNotFound);
    };

    let timestamp = get_server_timestamp(&conn, screen)?;

    // Method 1: _NET_ACTIVE_WINDOW (EWMH)
    let data = ClientMessageData::from([2u32, timestamp, 0, 0, 0]);
    let event = ClientMessageEvent::new(32, target, net_active_window, data);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
        event,
    )
    .map_err(|e| io::Error::other(format!("send _NET_ACTIVE_WINDOW: {e}")))?;

    conn.flush()
        .map_err(|e| io::Error::other(format!("X11 flush: {e}")))?;
    conn.sync()
        .map_err(|e| io::Error::other(format!("X11 sync: {e}")))?;

    if has_focus(&conn, target) {
        log::info!("activated with: X11 _NET_ACTIVE_WINDOW (EWMH)");
        return Ok(BringUnityToFrontResult::BroughtToFront);
    }

    // Method 2: Core X11 raise + focus (fallback for XWayland compositors
    // that ignore _NET_ACTIVE_WINDOW)
    conn.configure_window(
        target,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )
    .map_err(|e| io::Error::other(format!("configure_window: {e}")))?;
    conn.set_input_focus(InputFocus::PARENT, target, timestamp)
        .map_err(|e| io::Error::other(format!("set_input_focus: {e}")))?;

    conn.flush()
        .map_err(|e| io::Error::other(format!("X11 flush: {e}")))?;
    conn.sync()
        .map_err(|e| io::Error::other(format!("X11 sync: {e}")))?;

    log::info!("activated with: X11 raise+focus (core fallback)");
    Ok(BringUnityToFrontResult::BroughtToFront)
}

fn has_focus(conn: &impl Connection, target: Window) -> bool {
    let Ok(cookie) = conn.get_input_focus() else {
        return false;
    };
    let Ok(reply) = cookie.reply() else {
        return false;
    };
    reply.focus == target
}

fn get_server_timestamp(
    conn: &impl Connection,
    screen: &x11rb::protocol::xproto::Screen,
) -> io::Result<u32> {
    let atom = intern_atom(conn, b"_ALCOM_TIMESTAMP_PING")?;

    let wid = conn
        .generate_id()
        .map_err(|e| io::Error::other(format!("generate_id: {e}")))?;
    conn.create_window(
        0,
        wid,
        screen.root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_ONLY,
        0,
        &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )
    .map_err(|e| io::Error::other(format!("create_window: {e}")))?;

    conn.change_property8(PropMode::APPEND, wid, atom, AtomEnum::STRING, &[])
        .map_err(|e| io::Error::other(format!("change_property: {e}")))?;
    conn.flush()
        .map_err(|e| io::Error::other(format!("flush: {e}")))?;

    let timestamp = loop {
        let event = conn
            .wait_for_event()
            .map_err(|e| io::Error::other(format!("wait_for_event: {e}")))?;
        if let Event::PropertyNotify(e) = event {
            break e.time;
        }
    };

    conn.destroy_window(wid)
        .map_err(|e| io::Error::other(format!("destroy_window: {e}")))?;
    conn.flush()
        .map_err(|e| io::Error::other(format!("flush: {e}")))?;

    Ok(timestamp)
}

fn intern_atom(conn: &impl Connection, name: &[u8]) -> io::Result<x11rb::protocol::xproto::Atom> {
    conn.intern_atom(false, name)
        .map_err(|e| io::Error::other(format!("intern_atom: {e}")))?
        .reply()
        .map(|r| r.atom)
        .map_err(|e| io::Error::other(format!("intern_atom reply: {e}")))
}
