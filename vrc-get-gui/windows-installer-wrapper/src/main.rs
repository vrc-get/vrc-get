#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
#![cfg(windows)]

extern crate windows_sys;

use core::ptr::NonNull;
use core::{
    mem,
    ptr::{null, null_mut},
};
use windows_sys::w;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, WriteConsoleW};
use windows_sys::Win32::System::Environment::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

static INSTALLER: &[u8] = include_bytes!(env!("INSTALLER_EXE"));

macro_rules! wc {
    ($value: literal) => {{
        #[allow(unused_unsafe)]
        unsafe {
            WCstr::from_cstr(w!($value))
        }
    }};
}

// MSVC Target uses mainCRTStartup as the entrypoint
// see https://github.com/rust-lang/rust/blob/212ef7770dfad656782207fda799bdae28fc5b7b/compiler/rustc_codegen_ssa/src/back/linker.rs#L1168-L1184
// see https://rust-lang.github.io/rfcs/1665-windows-subsystem.html#additional-linker-argument
#[cfg(target_env = "msvc")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> ! {
    main()
}

// Rustc doesn't override entrypoint for gnu linkers
// see https://github.com/rust-lang/rust/blob/212ef7770dfad656782207fda799bdae28fc5b7b/compiler/rustc_codegen_ssa/src/back/linker.rs#L889-L891
// see https://github.com/rust-lang/rust/blob/212ef7770dfad656782207fda799bdae28fc5b7b/compiler/rustc_codegen_ssa/src/back/linker.rs#L1583-L1586
#[cfg(target_env = "gnu")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn WinMainCRTStartup() -> ! {
    main()
}

fn main() -> ! {
    unsafe {
        let path = match create_temp_file() {
            Some(p) => p,
            None => {
                error_out(wc!("Failed to create temporary file\n"));
                ExitProcess(1)
            }
        };
        let path = path.as_wcstr();

        if write_installer(path).is_err() {
            error_out(wc!("writing to temporary file failed\n"));
            ExitProcess(2);
        }

        let command_line = WCstr::from_cstr(GetCommandLineW());
        let command_line = command_line.remove_until(b' ' as u16).unwrap_or(wc!(""));

        let replace = command_line.contains(wc!("/UPDATE")) || command_line.contains(wc!("/P"));

        let mut cmdline = match build_cmdline(path, replace) {
            Some(c) => c,
            None => ExitProcess(3),
        };

        let code = run_and_wait(path, cmdline.as_mut());

        DeleteFileW(path.as_ptr());

        ExitProcess(code);
    }
}

unsafe fn create_temp_file() -> Option<StackPath> {
    unsafe {
        let mut temp = StackPath::new();
        if GetTempPathW(temp.capacity(), temp.as_mut_ptr()) == 0 {
            return None;
        }

        let mut name = StackPath::new();

        if GetTempFileNameW(temp.as_ptr(), w!("upd"), 0, name.as_mut_ptr()) == 0 {
            return None;
        }

        Some(name)
    }
}

unsafe fn write_installer(path: &WCstr) -> Result<(), ()> {
    unsafe {
        let file = Handle::new(CreateFileW(
            path.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_TEMPORARY,
            null_mut(),
        ));

        if file.raw == INVALID_HANDLE_VALUE {
            return Err(());
        }

        let mut written = 0;

        if WriteFile(
            file.raw,
            INSTALLER.as_ptr() as _,
            INSTALLER.len() as u32,
            &mut written,
            null_mut(),
        ) == 0
        {
            return Err(());
        }

        Ok(())
    }
}

unsafe fn run_and_wait(path: &WCstr, cmdline: &mut WCstr) -> u32 {
    unsafe {
        let mut si: STARTUPINFOW = mem::zeroed();
        si.cb = size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESHOWWINDOW;
        si.wShowWindow = SW_HIDE as u16;

        let mut pi: PROCESS_INFORMATION = mem::zeroed();

        if CreateProcessW(
            path.as_ptr(),
            cmdline.as_mut_ptr(),
            null_mut(),
            null_mut(),
            0,
            0,
            null_mut(),
            null(),
            &mut si,
            &mut pi,
        ) == 0
        {
            return 1;
        }

        WaitForSingleObject(pi.hProcess, INFINITE);

        let mut code = 0;
        GetExitCodeProcess(pi.hProcess, &mut code);

        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);

        code
    }
}

unsafe fn build_cmdline(path: &WCstr, replace: bool) -> Option<WCString> {
    let original = unsafe { WCstr::from_cstr(GetCommandLineW()) };
    let original = original.remove_until(b' ' as u16).unwrap_or(wc!(""));

    let mut len = path.len() + 1;
    // Specify /CURRENTUSER since our NSIS installer doesn't support machine/alluser install
    let update_params = wc!(" /SP- /SILENT /NOICONS /CURRENTUSER");

    if replace {
        error_out(wc!("nsis update mode detected. replacing command line\n"));
        len += update_params.len();
    } else {
        len += original.len();
    }

    let mut mem = WCString::with_capacity(len)?;

    mem.append(path);

    if replace {
        mem.append(update_params);
    } else {
        mem.append(wc!(" "));
        mem.append(original);
    }

    Some(mem)
}

struct WCString {
    ptr: NonNull<u16>,
    len: usize,
    cap: usize,
}

impl WCString {
    fn with_capacity(cap: usize) -> Option<Self> {
        unsafe {
            let heap = GetProcessHeap();
            let mem = HeapAlloc(heap, HEAP_ZERO_MEMORY, (cap + 1) * 2) as *mut u16;
            if mem.is_null() {
                return None;
            }
            core::slice::from_raw_parts_mut(mem, cap + 1).fill(0);
            Some(Self {
                ptr: NonNull::new(mem).unwrap(),
                len: 0,
                cap,
            })
        }
    }

    fn append(&mut self, s: &WCstr) {
        assert!(self.len.saturating_add(s.len()) <= self.cap);
        unsafe {
            core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.cap)[self.len..][..s.len()]
                .copy_from_slice(s.contents());
            self.len += s.len();
        }
    }

    #[allow(unused)]
    fn as_str(&self) -> &WCstr {
        WCstr::from_slice_with_nl(unsafe {
            core::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
        })
    }

    fn as_mut(&mut self) -> &mut WCstr {
        WCstr::from_slice_with_nl_mut(unsafe {
            core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
        })
    }
}

const unsafe fn wcslen(mut s: *const u16) -> usize {
    unsafe {
        let mut len = 0;

        while *s != 0 {
            len += 1;
            s = s.add(1);
        }

        len
    }
}

struct StackPath {
    buf: [u16; MAX_PATH as usize],
}

impl StackPath {
    fn new() -> StackPath {
        Self {
            buf: [0; MAX_PATH as usize],
        }
    }

    fn capacity(&self) -> u32 {
        MAX_PATH as usize as u32
    }

    fn as_mut_ptr(&mut self) -> *mut u16 {
        &mut self.buf as *mut _
    }

    fn as_ptr(&self) -> *const u16 {
        self.buf.as_ptr()
    }

    fn as_wcstr(&self) -> &WCstr {
        unsafe { WCstr::from_cstr(self.as_ptr()) }
    }
}

struct Handle {
    raw: HANDLE,
}

impl Handle {
    fn new(raw: HANDLE) -> Self {
        Self { raw }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() && self.raw != INVALID_HANDLE_VALUE {
                CloseHandle(self.raw);
            }
        }
    }
}

struct WCstr([u16]);

impl WCstr {
    const unsafe fn from_cstr<'a>(s: *const u16) -> &'a WCstr {
        unsafe {
            let len = wcslen(s) + 1;
            &*(core::slice::from_raw_parts::<'a>(s, len) as *const [u16] as *const WCstr)
        }
    }

    #[allow(unused)]
    unsafe fn from_cstr_mut<'a>(s: *mut u16) -> &'a mut WCstr {
        unsafe {
            let len = wcslen(s) + 1;
            &mut *(core::slice::from_raw_parts_mut::<'a>(s, len) as *mut [u16] as *mut WCstr)
        }
    }

    fn from_slice_with_nl(slice: &[u16]) -> &WCstr {
        unsafe { &*(slice as *const [u16] as *const WCstr) }
    }

    fn from_slice_with_nl_mut(slice: &mut [u16]) -> &mut WCstr {
        unsafe { &mut *(slice as *mut [u16] as *mut WCstr) }
    }

    fn remove_until(&self, c: u16) -> Option<&WCstr> {
        Some(Self::from_slice_with_nl(
            &self.0[self.0.iter().position(|&x| x == c)?..],
        ))
    }

    fn len(&self) -> usize {
        self.0.len() - 1
    }

    fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> *mut u16 {
        self.0.as_mut_ptr()
    }

    fn contains(&self, other: &WCstr) -> bool {
        unsafe {
            let mut h1 = self.as_ptr();

            while *h1 != 0 {
                let mut h2 = h1;
                let mut n1 = other.as_ptr();

                while *h2 != 0 && *n1 != 0 && *h2 == *n1 {
                    h2 = h2.add(1);
                    n1 = n1.add(1);
                }

                if *n1 == 0 {
                    return true;
                }

                h1 = h1.add(1);
            }

            false
        }
    }

    fn contents(&self) -> &[u16] {
        &self.0[0..self.len()]
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    error_out(wc!("panic\n"));
    unsafe { ExitProcess(1) }
}

#[allow(unused)]
fn error_out(out: &WCstr) {
    unsafe {
        let stderr = GetStdHandle(STD_ERROR_HANDLE);
        let mut written = 0;
        WriteConsoleW(stderr, out.as_ptr(), out.len() as u32, &mut written, null());
    }
}

// prevent crt from linking

#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *dest.add(i) = *src.add(i) };
    }
    dest
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(dest: *mut u8, c: core::ffi::c_int, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *dest.add(i) = c as u8 };
    }
    dest
}
