/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::utils;
use crate::utils::command::{CommandExt, create_command};
use anyhow::{Context, Result, bail};
use std::io::{IsTerminal, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// The default port of vite dev server. Must be same as `devUrl` in Tauri.toml
const DEFAULT_DEV_SERVER_PORT: u16 = 3030;
/// The number of ports we try when the default port is in use.
const DEV_SERVER_PORT_CANDIDATES: u16 = 20;
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const DEV_SERVER_TIMEOUT: Duration = Duration::from_secs(60);
const PROMPT: &str = "enter 'r' to restart ALCOM, 'q' to quit";

/// Starts the vite dev server and runs the dev build of ALCOM.
///
/// While running, you can enter 'r' to rebuild and restart ALCOM, or 'q' to quit.
#[derive(clap::Parser)]
pub(super) struct Command {
    /// Skips running `npm install` before starting dev server.
    #[arg(long)]
    skip_npm_install: bool,

    /// Enable verbose cargo output.
    #[arg(long)]
    verbose: bool,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let workspace_root = utils::cargo::workspace_root();
        let gui_dir = workspace_root.join("vrc-get-gui");

        if !self.skip_npm_install {
            create_command("npm")
                .arg("install")
                .current_dir(&gui_dir)
                .run_checked("npm install for vrc-get-gui")?;
        }

        let port = find_dev_server_port()?;

        let mut children = Children {
            vite: spawn_vite(&gui_dir, port)?,
            alcom: None,
            build: None,
        };

        // build ALCOM while the dev server is starting
        build_command(workspace_root, self.verbose, port)
            .run_checked("building vrc-get-gui for dev")?;
        wait_for_dev_server(&mut children.vite, port)?;

        children.alcom = Some(spawn_alcom(&gui_dir)?);
        // declared after `children` so the terminal is restored before killing children
        let mut status_line = StatusLine::new();
        status_line.set("running", "started ALCOM");

        let input = spawn_stdin_reader();
        let mut interactive = true;
        // true if restart is requested while building. we'll build again after the current build.
        let mut rebuild_requested = false;

        loop {
            match input.recv_timeout(POLL_INTERVAL) {
                Ok(Input::Byte(byte)) => match status_line.key(byte).as_deref().map(str::trim) {
                    Some("r" | "R") => {
                        if children.build.is_some() {
                            rebuild_requested = true;
                            status_line.log("will rebuild again after the current build");
                        } else {
                            children.kill_alcom();
                            children.build = Some(spawn_build(workspace_root, self.verbose, port)?);
                            status_line.set("building", "rebuilding ALCOM");
                        }
                    }
                    Some("q" | "Q") => return Ok(0),
                    Some("") | None => {}
                    Some(command) => status_line.log(&format!("unknown command '{command}'")),
                },
                Ok(Input::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    interactive = false;
                    // nothing can restart ALCOM anymore
                    if children.alcom.is_none() && children.build.is_none() {
                        return Ok(1);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }

            if terminal::interrupted() {
                // Ctrl-C or similar. Children will be killed and the terminal will be restored on drop.
                return Ok(130);
            }

            status_line.poll_resize();

            if let Some(status) = children.vite.try_wait().context("waiting vite")? {
                bail!("vite dev server exited unexpectedly with {status}");
            }

            // The build runs in background to keep processing inputs and resizes while building.
            if let Some(build) = &mut children.build
                && let Some(status) = build.try_wait().context("waiting cargo build")?
            {
                children.build = None;
                if std::mem::take(&mut rebuild_requested) {
                    children.build = Some(spawn_build(workspace_root, self.verbose, port)?);
                    status_line.log("rebuilding ALCOM again");
                } else if status.success() {
                    children.alcom = Some(spawn_alcom(&gui_dir)?);
                    status_line.set("running", "restarted ALCOM");
                } else if interactive {
                    status_line.set("build failed", &format!("build failed with {status}"));
                } else {
                    return Ok(status.code().unwrap_or(1));
                }
            }

            if let Some(alcom) = &mut children.alcom
                && let Some(status) = alcom.try_wait().context("waiting ALCOM")?
            {
                children.alcom = None;
                if !interactive {
                    return Ok(status.code().unwrap_or(1));
                }
                status_line.set("exited", &format!("ALCOM exited with {status}"));
            }
        }
    }
}

/// The child processes of dev-alcom. They will be killed on drop.
struct Children {
    vite: Child,
    alcom: Option<Child>,
    /// The `cargo build` for restarting ALCOM
    build: Option<Child>,
}

impl Children {
    fn kill_alcom(&mut self) {
        if let Some(mut alcom) = self.alcom.take() {
            alcom.kill().ok();
            alcom.wait().ok();
        }
    }
}

impl Drop for Children {
    fn drop(&mut self) {
        if let Some(mut build) = self.build.take() {
            build.kill().ok();
            build.wait().ok();
        }
        self.kill_alcom();
        self.vite.kill().ok();
        self.vite.wait().ok();
    }
}

/// Finds the port for the dev server.
///
/// We use the default port if possible, but if it's in use, e.g. by dev server of another
/// working copy, we use another port to avoid conflict.
///
/// The URL of the dev server is embedded to ALCOM so changing port needs rebuilding ALCOM.
/// Therefore, we try the following ports in order instead of asking a free port to the OS,
/// which is likely to be different every time, to reduce the number of rebuilds.
fn find_dev_server_port() -> Result<u16> {
    let candidates =
        DEFAULT_DEV_SERVER_PORT..DEFAULT_DEV_SERVER_PORT.saturating_add(DEV_SERVER_PORT_CANDIDATES);
    for port in candidates.clone() {
        if is_port_available(port) {
            if port != DEFAULT_DEV_SERVER_PORT {
                eprintln!(
                    "dev-alcom: port {DEFAULT_DEV_SERVER_PORT} is in use. using port {port} for dev server"
                );
            }
            return Ok(port);
        }
    }
    bail!(
        "no ports are available for dev server in {}..={}",
        candidates.start,
        candidates.end - 1
    );
}

fn is_port_available(port: u16) -> bool {
    // vite listens on localhost, which can be either IPv4 or IPv6
    // If another server is listening, vite fails to start, and wait_for_dev_server would wrongly
    // treat that server as ours.
    if TcpStream::connect(("localhost", port)).is_ok() {
        return false;
    }
    // This detects the ports bound but not (yet) accepting connections
    let can_bind = |addr: IpAddr| match TcpListener::bind((addr, port)) {
        Ok(_) => true,
        // e.g. IPv6 is not available on the machine. It's not because the port is in use.
        Err(e) => e.kind() != std::io::ErrorKind::AddrInUse,
    };
    can_bind(Ipv4Addr::LOCALHOST.into()) && can_bind(Ipv6Addr::LOCALHOST.into())
}

fn spawn_vite(gui_dir: &Path, port: u16) -> Result<Child> {
    // We launch vite directly instead of `npm run dev` to make sure we can kill vite;
    // killing npm does not kill the vite launched by the npm.
    create_command("node")
        .arg("node_modules/vite/bin/vite.js")
        // ALCOM connects to the port so vite must not choose another port.
        // This is ensured by `strictPort` in vite.config.ts.
        .arg("--port")
        .arg(port.to_string())
        .current_dir(gui_dir)
        // vite exits on EOF of the stdin so we have to keep the stdin open.
        // The pipe is kept open since it's owned by the `Child`.
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("starting vite dev server")
}

fn wait_for_dev_server(vite: &mut Child, port: u16) -> Result<()> {
    let start = Instant::now();
    loop {
        if TcpStream::connect(("localhost", port)).is_ok() {
            return Ok(());
        }
        if let Some(status) = vite.try_wait().context("waiting vite")? {
            bail!("vite dev server exited unexpectedly with {status}");
        }
        if start.elapsed() > DEV_SERVER_TIMEOUT {
            bail!("vite dev server did not start listening on localhost:{port}");
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Creates `cargo build -p vrc-get-gui` command for dev build.
///
/// Unlike build-alcom, we don't enable `custom-protocol` feature, so ALCOM will use the dev server.
///
/// The URL of the dev server is embedded to the binary at build time, so when we use non-default
/// port, we override `devUrl` with `TAURI_CONFIG`, which is merged to Tauri.toml by tauri.
/// We don't set it for the default port to share the build cache with normal `cargo build`.
fn build_command(workspace_root: &Path, verbose: bool, port: u16) -> std::process::Command {
    let mut cmd = utils::cargo::command();
    cmd.current_dir(workspace_root)
        .arg("build")
        .arg("-p")
        .arg("vrc-get-gui");

    if verbose {
        cmd.arg("--verbose");
    }

    if port != DEFAULT_DEV_SERVER_PORT {
        let config =
            serde_json::json!({ "build": { "devUrl": format!("http://localhost:{port}") } });
        cmd.env("TAURI_CONFIG", config.to_string());
    }

    cmd
}

fn spawn_build(workspace_root: &Path, verbose: bool, port: u16) -> Result<Child> {
    build_command(workspace_root, verbose, port)
        // cargo must not read the keys for us
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("starting cargo build")
}

fn spawn_alcom(gui_dir: &Path) -> Result<Child> {
    let binary = utils::build_dir(None::<&str>, "dev")
        .join(format!("ALCOM{}", std::env::consts::EXE_SUFFIX));

    std::process::Command::new(&binary)
        // dev build of ALCOM generates lib/bindings.ts relative to the working directory
        .current_dir(gui_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("starting {}", binary.display()))
}

enum Input {
    Byte(u8),
    Eof,
}

/// Reads stdin byte by byte. The bytes are assembled to command by [`StatusLine::key`].
fn spawn_stdin_reader() -> mpsc::Receiver<Input> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for byte in std::io::stdin().lock().bytes() {
            let Ok(byte) = byte else { break };
            if sender.send(Input::Byte(byte)).is_err() {
                return;
            }
        }
        sender.send(Input::Eof).ok();
    });
    receiver
}

/// The line showing the state of ALCOM and the prompt.
///
/// If possible, the line is pinned to the bottom of the terminal by limiting the scrolling region
/// of the terminal to the lines above it, so the outputs of vite / ALCOM / cargo, which are
/// directly connected to the terminal, will scroll without overwriting the status line.
///
/// While pinned, echo of the terminal is disabled and the typing command is shown on the status
/// line instead, since the echo will be mixed in the outputs of child processes.
///
/// When stderr is not a terminal, the prompt is just printed as a normal log line.
struct StatusLine {
    state: &'static str,
    /// The command currently typing
    input: String,
    /// The (rows, columns) of the terminal if the status line is pinned.
    pinned: Option<(u16, u16)>,
}

impl StatusLine {
    fn new() -> Self {
        let mut result = Self {
            state: "",
            input: String::new(),
            pinned: None,
        };

        // We have to restore the scrolling region on exit, so we need to handle Ctrl-C.
        if std::io::stderr().is_terminal()
            && let Some(size) = terminal::size()
            && terminal::prepare()
        {
            terminal::disable_echo();
            result.pinned = Some(size);
            result.setup_scroll_region();
        }

        result
    }

    /// Updates the state shown on the status line with printing log message
    fn set(&mut self, state: &'static str, message: &str) {
        self.state = state;
        self.log(message);
        self.draw();
    }

    fn log(&self, message: &str) {
        if self.pinned.is_some() {
            eprintln!("dev-alcom: {message}");
        } else {
            eprintln!("dev-alcom: {message}. {PROMPT}");
        }
    }

    /// Processes one byte from stdin. Returns the command if a command is entered.
    fn key(&mut self, byte: u8) -> Option<String> {
        const MAX_INPUT: usize = 16;
        let result = match byte {
            b'\n' | b'\r' => Some(std::mem::take(&mut self.input)),
            // backspace / delete
            0x08 | 0x7f => {
                self.input.pop();
                None
            }
            // Ctrl-U
            0x15 => {
                self.input.clear();
                None
            }
            b' '..=b'~' if self.input.len() < MAX_INPUT => {
                self.input.push(byte as char);
                None
            }
            _ => return None,
        };
        self.draw();
        result
    }

    /// Follows the resize of the terminal.
    fn poll_resize(&mut self) {
        let Some(old_size) = self.pinned else {
            return;
        };
        let Some(size) = terminal::size() else {
            return;
        };
        if size != old_size {
            // Clear the status line at the old position:
            //   ESC 7          save cursor position (DECSC)
            //   ESC [ {row};1H move cursor to column 1 of the old status line (CUP)
            //   ESC [ 2K       erase the entire line (EL)
            //   ESC 8          restore cursor position (DECRC)
            write_terminal(&format!("\x1b7\x1b[{};1H\x1b[2K\x1b8", old_size.0));
            self.pinned = Some(size);
            self.setup_scroll_region();
        }
    }

    fn setup_scroll_region(&self) {
        let Some((rows, _)) = self.pinned else {
            return;
        };
        // Limit the scrolling region to the lines above the status line:
        //   \n                 with the next one, ensures the cursor is not on the last line
        //                      (scrolls the contents up by one line if it was on the last line)
        //   ESC [ 1A           move cursor up by one line (CUU)
        //   ESC 7              save cursor position (DECSC)
        //   ESC [ 1;{bottom}r  set scrolling region to line 1..=bottom (DECSTBM).
        //                      This moves the cursor to the top-left corner.
        //   ESC 8              restore cursor position (DECRC)
        write_terminal(&format!("\n\x1b[1A\x1b7\x1b[1;{}r\x1b8", rows - 1));
        self.draw();
    }

    fn draw(&self) {
        let Some((rows, columns)) = self.pinned else {
            return;
        };
        let text = format!(" dev-alcom [{}] {PROMPT} > {}_ ", self.state, self.input);
        // the text is ascii-only so we can cut at any index
        let text = &text[..text.len().min(columns as usize)];
        // Draw the status line without moving the cursor from the scrolling region:
        //   ESC 7           save cursor position (DECSC)
        //   ESC [ {rows};1H move cursor to column 1 of the last line (CUP)
        //   ESC [ 2K        erase the entire line (EL)
        //   ESC [ 7m        start reverse video (SGR)
        //   {text}
        //   ESC [ 0m        reset text attributes (SGR)
        //   ESC 8           restore cursor position (DECRC)
        write_terminal(&format!(
            "\x1b7\x1b[{rows};1H\x1b[2K\x1b[7m{text}\x1b[0m\x1b8"
        ));
    }
}

impl Drop for StatusLine {
    fn drop(&mut self) {
        if let Some((rows, _)) = self.pinned {
            terminal::restore_echo();
            // Reset the scrolling region and clear the status line:
            //   ESC 7           save cursor position (DECSC)
            //   ESC [ r         reset scrolling region to the entire screen (DECSTBM).
            //                   This moves the cursor to the top-left corner.
            //   ESC 8           restore cursor position (DECRC)
            //   ESC 7           save cursor position again (DECSC)
            //   ESC [ {rows};1H move cursor to column 1 of the status line (CUP)
            //   ESC [ 2K        erase the entire line (EL)
            //   ESC 8           restore cursor position (DECRC)
            write_terminal(&format!("\x1b7\x1b[r\x1b8\x1b7\x1b[{rows};1H\x1b[2K\x1b8"));
        }
    }
}

/// Writes to the terminal with single write to avoid mixing with outputs of child processes
fn write_terminal(data: &str) {
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(data.as_bytes()).ok();
    stderr.flush().ok();
}

/// Platform dependent part of the terminal handling
#[cfg(unix)]
mod terminal {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    static INTERRUPTED: AtomicBool = AtomicBool::new(false);

    /// Returns true if this process is requested to exit with Ctrl-C or similar.
    pub fn interrupted() -> bool {
        INTERRUPTED.load(Ordering::Relaxed)
    }

    /// Gets the (rows, columns) of the terminal
    pub fn size() -> Option<(u16, u16)> {
        // SAFETY: winsize is plain old data and TIOCGWINSZ just fills the passed winsize.
        let size = unsafe {
            let mut size: libc::winsize = std::mem::zeroed();
            if libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ as _, &mut size) != 0 {
                return None;
            }
            size
        };
        super::usable_size(size.ws_row, size.ws_col)
    }

    static ORIGINAL_TERMIOS: Mutex<Option<libc::termios>> = Mutex::new(None);

    /// Disables echo and line buffering of the terminal, if stdin is a terminal.
    ///
    /// Signals like Ctrl-C are kept enabled.
    pub fn disable_echo() {
        // SAFETY: termios is plain old data and those functions just read / write it.
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) != 0 {
                return;
            }
            let original = termios;
            termios.c_lflag &= !(libc::ICANON | libc::ECHO);
            termios.c_cc[libc::VMIN] = 1;
            termios.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios) == 0 {
                *ORIGINAL_TERMIOS.lock().unwrap() = Some(original);
            }
        }
    }

    pub fn restore_echo() {
        if let Some(termios) = ORIGINAL_TERMIOS.lock().unwrap().take() {
            // SAFETY: termios is the valid one we got from tcgetattr
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);
            }
        }
    }

    extern "C" fn on_signal(_: libc::c_int) {
        // only async-signal-safe operation is allowed here
        INTERRUPTED.store(true, Ordering::Relaxed);
    }

    /// Prepares for pinning status line: makes [`interrupted`] work instead of dying by signals.
    ///
    /// The handlers are reset to default in the child processes on exec so they are not affected.
    pub fn prepare() -> bool {
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            // SAFETY: the handler only stores to an atomic variable, which is async-signal-safe.
            let result = unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = on_signal as extern "C" fn(libc::c_int) as usize;
                action.sa_flags = libc::SA_RESTART;
                libc::sigemptyset(&mut action.sa_mask);
                libc::sigaction(signal, &action, std::ptr::null_mut())
            };
            if result != 0 {
                return false;
            }
        }
        true
    }
}

#[cfg(windows)]
mod terminal {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows_sys::Win32::Foundation::{FALSE, HANDLE, TRUE};
    use windows_sys::Win32::System::Console::{
        CONSOLE_SCREEN_BUFFER_INFO, CTRL_BREAK_EVENT, CTRL_C_EVENT, ENABLE_ECHO_INPUT,
        ENABLE_LINE_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode,
        GetConsoleScreenBufferInfo, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
        SetConsoleCtrlHandler, SetConsoleMode,
    };
    use windows_sys::core::BOOL;

    static INTERRUPTED: AtomicBool = AtomicBool::new(false);

    /// Returns true if this process is requested to exit with Ctrl-C or similar.
    pub fn interrupted() -> bool {
        INTERRUPTED.load(Ordering::Relaxed)
    }

    fn stderr_handle() -> HANDLE {
        // SAFETY: GetStdHandle has no preconditions
        unsafe { GetStdHandle(STD_ERROR_HANDLE) }
    }

    /// Gets the (rows, columns) of the terminal
    pub fn size() -> Option<(u16, u16)> {
        // SAFETY: CONSOLE_SCREEN_BUFFER_INFO is plain old data and the function just fills it.
        let info = unsafe {
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(stderr_handle(), &mut info) == 0 {
                return None;
            }
            info
        };
        // escape sequences work in the coordinate of the window, not of the buffer.
        let rows = info.srWindow.Bottom - info.srWindow.Top + 1;
        let columns = info.srWindow.Right - info.srWindow.Left + 1;
        super::usable_size(rows.try_into().ok()?, columns.try_into().ok()?)
    }

    static ORIGINAL_INPUT_MODE: Mutex<Option<u32>> = Mutex::new(None);

    /// Disables echo and line buffering of the console, if stdin is a console.
    ///
    /// Ctrl-C is kept enabled.
    pub fn disable_echo() {
        // SAFETY: mode is valid pointer to write
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            let mut mode = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return;
            }
            if SetConsoleMode(handle, mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT)) != 0 {
                *ORIGINAL_INPUT_MODE.lock().unwrap() = Some(mode);
            }
        }
    }

    pub fn restore_echo() {
        if let Some(mode) = ORIGINAL_INPUT_MODE.lock().unwrap().take() {
            // SAFETY: SetConsoleMode has no memory safety preconditions
            unsafe {
                SetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), mode);
            }
        }
    }

    unsafe extern "system" fn on_ctrl(ctrl_type: u32) -> BOOL {
        match ctrl_type {
            CTRL_C_EVENT | CTRL_BREAK_EVENT => {
                INTERRUPTED.store(true, Ordering::Relaxed);
                // handled; the main loop will exit the process
                TRUE
            }
            _ => FALSE,
        }
    }

    /// Prepares for pinning status line: enables escape sequences and
    /// makes [`interrupted`] work instead of dying by Ctrl-C.
    ///
    /// The child processes have their own handlers so they are not affected.
    pub fn prepare() -> bool {
        // SAFETY: mode is valid pointer to write and the handler only stores to an atomic variable.
        unsafe {
            let handle = stderr_handle();
            let mut mode = 0;
            GetConsoleMode(handle, &mut mode) != 0
                && SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
                && SetConsoleCtrlHandler(Some(on_ctrl), TRUE) != 0
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod terminal {
    pub fn interrupted() -> bool {
        false
    }

    pub fn size() -> Option<(u16, u16)> {
        None
    }

    pub fn prepare() -> bool {
        false
    }

    pub fn disable_echo() {}

    pub fn restore_echo() {}
}

/// Returns the size if the terminal is large enough to pin the status line
#[cfg(any(unix, windows))]
fn usable_size(rows: u16, columns: u16) -> Option<(u16, u16)> {
    (rows >= 5 && columns >= 1).then_some((rows, columns))
}
