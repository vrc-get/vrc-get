// This file is based on the following file from the dotnet runtime
// https://github.com/dotnet/runtime/blob/v8.0.1/src/coreclr/nativeaot/Bootstrap/main.cpp
// Original license:
// Licensed to the .NET Foundation under one or more agreements.
// The .NET Foundation licenses this file to you under the MIT license.

//
// This is the mechanism whereby multiple linked modules contribute their global data for initialization at
// startup of the application.
//
// ILC creates sections in the output obj file to mark the beginning and end of merged global data.
// It defines sentinel symbols that are used to get the addresses of the start and end of global data
// at runtime. The section names are platform-specific to match platform-specific linker conventions.
//

use std::ffi::c_int;

extern "C" {
    fn RhInitialize(isDll: bool) -> bool;
    fn RhSetRuntimeInitializationCallback(fPtr: unsafe extern "C" fn() -> c_int);
    fn RhRegisterOSModule(
        p_module: *mut u8,
        pv_managed_code_start_range: *const u8,
        cb_managed_code_range: u32,
        pv_unboxing_stubs_start_range: *const u8,
        cb_unboxing_stubs_range: u32,
        p_classlib_functions: *const ClasslibFunction,
        n_classlib_functions: u32,
    ) -> bool;
    fn PalGetModuleHandleFromPointer(pointer: *mut u8) -> *mut u8;

    // region classlibFunctions
    fn GetRuntimeException();
    fn FailFast();
    fn AppendExceptionStackFrame();
    fn GetSystemArrayEEType();
    fn OnFirstChanceException();
    fn OnUnhandledException();
    fn IDynamicCastableIsInterfaceImplemented();
    fn IDynamicCastableGetInterfaceImplementation();
    #[cfg(target_vendor = "apple")]
    fn ObjectiveCMarshalTryGetTaggedMemory();
    #[cfg(target_vendor = "apple")]
    fn ObjectiveCMarshalGetIsTrackedReferenceCallback();
    #[cfg(target_vendor = "apple")]
    fn ObjectiveCMarshalGetOnEnteredFinalizerQueueCallback();
    #[cfg(target_vendor = "apple")]
    fn ObjectiveCMarshalGetUnhandledExceptionPropagationHandler();
    // endregion

    fn InitializeModules(
        os_module: *mut u8,
        modules: *const usize,
        count: c_int,
        p_classlib_functions: *const ClasslibFunction,
        n_classlib_functions: c_int,
    );

    fn __managed__Startup();
}

#[cfg(target_vendor = "apple")]
macro_rules! apple_fn_or_none {
    ($expr: expr) => {
        Some($expr)
    };
}

#[cfg(not(target_vendor = "apple"))]
macro_rules! apple_fn_or_none {
    ($expr: expr) => {
        None
    };
}

type ClasslibFunction = Option<unsafe extern "C" fn()>;

#[test]
fn test_classlib_function_size() {
    assert_eq!(
        std::mem::size_of::<ClasslibFunction>(),
        std::mem::size_of::<usize>()
    );
    assert_eq!(
        std::mem::align_of::<ClasslibFunction>(),
        std::mem::align_of::<usize>()
    );
}

static C_CLASSLIB_FUNCTIONS: [ClasslibFunction; 14] = [
    Some(GetRuntimeException),
    Some(FailFast),
    None, // UnhandledExceptionHandler
    Some(AppendExceptionStackFrame),
    None, // CheckStaticClassConstruction
    Some(GetSystemArrayEEType),
    Some(OnFirstChanceException),
    Some(OnUnhandledException),
    Some(IDynamicCastableIsInterfaceImplemented),
    Some(IDynamicCastableGetInterfaceImplementation),
    apple_fn_or_none!(ObjectiveCMarshalTryGetTaggedMemory),
    apple_fn_or_none!(ObjectiveCMarshalGetIsTrackedReferenceCallback),
    apple_fn_or_none!(ObjectiveCMarshalGetOnEnteredFinalizerQueueCallback),
    apple_fn_or_none!(ObjectiveCMarshalGetUnhandledExceptionPropagationHandler),
];

static INITIALIZE_ONCE: std::sync::Once = std::sync::Once::new();

pub(crate) fn initialize() {
    INITIALIZE_ONCE.call_once(|| unsafe {
        RhSetRuntimeInitializationCallback(initialize_runtime);
    });
}

extern "C" fn initialize_runtime() -> c_int {
    unsafe {
        if !RhInitialize(true) {
            return -1;
        }

        let os_module = PalGetModuleHandleFromPointer(__managed__Startup as *mut u8);

        let managedcode = managedcode();
        let unbox = unbox();

        if !RhRegisterOSModule(
            os_module,
            managedcode.as_ptr(),
            managedcode.len() as u32,
            unbox.as_ptr(),
            unbox.len() as u32,
            C_CLASSLIB_FUNCTIONS.as_ptr(),
            C_CLASSLIB_FUNCTIONS.len() as u32,
        ) {
            return -1;
        }

        let modules = modules();

        InitializeModules(
            os_module,
            modules.as_ptr(),
            modules.len() as c_int,
            C_CLASSLIB_FUNCTIONS.as_ptr(),
            C_CLASSLIB_FUNCTIONS.len() as c_int,
        );

        // Run startup method immediately for a native library
        __managed__Startup();

        0
    }
}

use os::*;

unsafe fn slice_from_start_stop<T>(start: &'static T, stop: &'static T) -> &'static [T] {
    unsafe {
        std::slice::from_raw_parts(
            start,
            (stop as *const T).offset_from(start as *const _) as usize,
        )
    }
}

#[cfg(target_vendor = "apple")]
mod os {
    // for apple platform (mach-o platform), we use section$start/section$end to get the section
    use crate::bootstrapper::slice_from_start_stop;

    extern "C" {
        #[link_name = "\u{1}section$start$__DATA$__modules"]
        static modules_start_ptr: usize;
        #[link_name = "\u{1}section$end$__DATA$__modules"]
        static modules_end_ptr: usize;
        #[link_name = "\u{1}section$start$__TEXT$__managedcode"]
        static managedcode_start_ptr: u8;
        #[link_name = "\u{1}section$end$__TEXT$__managedcode"]
        static managedcode_end_ptr: u8;
        #[link_name = "\u{1}section$start$__TEXT$__unbox"]
        static unbox_start_ptr: u8;
        #[link_name = "\u{1}section$end$__TEXT$__unbox"]
        static unbox_end_ptr: u8;
    }

    pub(super) fn managedcode() -> &'static [u8] {
        unsafe { slice_from_start_stop(&managedcode_start_ptr, &managedcode_end_ptr) }
    }

    pub(super) fn unbox() -> &'static [u8] {
        unsafe { slice_from_start_stop(&unbox_start_ptr, &unbox_end_ptr) }
    }

    pub(super) fn modules() -> &'static [usize] {
        unsafe { slice_from_start_stop(&modules_start_ptr, &modules_end_ptr) }
    }
}

#[cfg(not(any(target_vendor = "apple", target_env = "msvc")))]
mod os {
    // for other platforms, (likey GNU platform), we use __start/__stop symbol to get section
    use crate::bootstrapper::slice_from_start_stop;

    extern "C" {
        static __start___modules: usize;
        static __stop___modules: usize;
        static __start___managedcode: u8;
        static __stop___managedcode: u8;
        static __start___unbox: u8;
        static __stop___unbox: u8;
    }

    pub(super) fn managedcode() -> &'static [u8] {
        unsafe { slice_from_start_stop(&__start___managedcode, &__stop___managedcode) }
    }

    pub(super) fn unbox() -> &'static [u8] {
        unsafe { slice_from_start_stop(&__start___unbox, &__stop___unbox) }
    }

    pub(super) fn modules() -> &'static [usize] {
        unsafe { slice_from_start_stop(&__start___modules, &__stop___modules) }
    }
}

#[cfg(target_env = "msvc")]
mod os {
    // There is nothing like #pragma comment(linker, "") in rust
    // so user needs to manually add the following linker option to the binary crate.
    // "/merge:.modules=.rdata" "/merge:.unbox=.text"

    // In MSVC, there is nothing like start and stop symbol,
    // so we put our code to .<section>$A and .<section>$Z and
    // use pointer to those code to get the start and stop of the section.

    use crate::bootstrapper::slice_from_start_stop;

    #[link_section = ".modules$A"]
    static MODULES_START: usize = 0;
    #[link_section = ".modules$Z"]
    static MODULES_END: usize = 0;

    static mut BOOKEND_A: u8 = 0;
    static mut BOOKEND_Z: u8 = 0;

    #[link_section = ".managedcode$A"]
    fn managedcode_start() -> *mut u8 {
        unsafe { &mut BOOKEND_A }
    }

    #[link_section = ".managedcode$Z"]
    fn managedcode_end() -> *mut u8 {
        unsafe { &mut BOOKEND_Z }
    }

    #[link_section = ".unbox$A"]
    fn unbox_start() -> *mut u8 {
        unsafe { &mut BOOKEND_A }
    }

    #[link_section = ".unbox$Z"]
    fn unbox_end() -> *mut u8 {
        unsafe { &mut BOOKEND_Z }
    }

    pub(super) fn managedcode() -> &'static [u8] {
        unsafe {
            slice_from_start_stop(
                &*(managedcode_start as *const _),
                &*(managedcode_end as *const _),
            )
        }
    }

    pub(super) fn unbox() -> &'static [u8] {
        unsafe { slice_from_start_stop(&*(unbox_start as *const _), &*(unbox_end as *const _)) }
    }

    pub(super) fn modules() -> &'static [usize] {
        unsafe { slice_from_start_stop(&MODULES_START, &MODULES_END) }
    }
}
