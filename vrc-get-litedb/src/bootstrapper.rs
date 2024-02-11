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
        pv_managed_code_start_range: *mut u8,
        cb_managed_code_range: u32,
        pv_unboxing_stubs_start_range: *mut u8,
        cb_unboxing_stubs_range: u32,
        p_classlib_functions: *mut ClasslibFunction,
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
        modules: *mut *mut u8,
        count: c_int,
        p_classlib_functions: *mut *mut u8,
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

static mut C_CLASSLIB_FUNCTIONS: [ClasslibFunction; 14] = [
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
            managedcode.as_mut_ptr(),
            managedcode.len() as u32,
            unbox.as_mut_ptr(),
            unbox.len() as u32,
            C_CLASSLIB_FUNCTIONS.as_mut_ptr(),
            C_CLASSLIB_FUNCTIONS.len() as u32,
        ) {
            return -1;
        }

        let modules = modules();

        InitializeModules(
            os_module,
            modules.as_mut_ptr(),
            modules.len() as c_int,
            C_CLASSLIB_FUNCTIONS.as_mut_ptr() as *mut *mut u8,
            C_CLASSLIB_FUNCTIONS.len() as c_int,
        );

        // Run startup method immediately for a native library
        __managed__Startup();

        0
    }
}

use os::*;

unsafe fn slice_from_start_stop<T>(start: &mut T, stop: &mut T) -> &'static mut [T] {
    unsafe {
        std::slice::from_raw_parts_mut(
            start,
            (stop as *mut T).offset_from(start as *mut _) as usize,
        )
    }
}

#[cfg(target_vendor = "apple")]
mod os {
    // for apple platform (mach-o platform), we use section$start/section$end to get the section
    use crate::bootstrapper::slice_from_start_stop;

    extern "C" {
        #[link_name = "\u{1}section$start$__DATA$__modules"]
        static mut modules_start_ptr: *mut u8;
        #[link_name = "\u{1}section$end$__DATA$__modules"]
        static mut modules_end_ptr: *mut u8;
        #[link_name = "\u{1}section$start$__TEXT$__managedcode"]
        static mut managedcode_start_ptr: u8;
        #[link_name = "\u{1}section$end$__TEXT$__managedcode"]
        static mut managedcode_end_ptr: u8;
        #[link_name = "\u{1}section$start$__TEXT$__unbox"]
        static mut unbox_start_ptr: u8;
        #[link_name = "\u{1}section$end$__TEXT$__unbox"]
        static mut unbox_end_ptr: u8;
    }

    pub(super) fn managedcode() -> &'static mut [u8] {
        unsafe { slice_from_start_stop(&mut managedcode_start_ptr, &mut managedcode_end_ptr) }
    }

    pub(super) fn unbox() -> &'static mut [u8] {
        unsafe { slice_from_start_stop(&mut unbox_start_ptr, &mut unbox_end_ptr) }
    }

    pub(super) fn modules() -> &'static mut [*mut u8] {
        unsafe { slice_from_start_stop(&mut modules_start_ptr, &mut modules_end_ptr) }
    }
}

#[cfg(not(any(target_vendor = "apple", target_env = "msvc")))]
mod os {
    // for other platforms, (likey GNU platform), we use __start/__stop symbol to get section
    use crate::bootstrapper::slice_from_start_stop;

    extern "C" {
        static mut __start___modules: *mut u8;
        static mut __stop___modules: *mut u8;
        static mut __start___managedcode: u8;
        static mut __stop___managedcode: u8;
        static mut __start___unbox: u8;
        static mut __stop___unbox: u8;
    }

    pub(super) fn managedcode() -> &'static mut [u8] {
        unsafe { slice_from_start_stop(&mut __start___managedcode, &mut __stop___managedcode) }
    }

    pub(super) fn unbox() -> &'static mut [u8] {
        unsafe { slice_from_start_stop(&mut __start___unbox, &mut __stop___unbox) }
    }

    pub(super) fn modules() -> &'static mut [*mut u8] {
        unsafe { slice_from_start_stop(&mut __start___modules, &mut __stop___modules) }
    }
}

// TODO: port MSVC

/*

#if defined(_MSC_VER)

#pragma section(".modules$A", read)
#pragma section(".modules$Z", read)
extern "C" __declspec(allocate(".modules$A")) void * __modules_a[];
extern "C" __declspec(allocate(".modules$Z")) void * __modules_z[];

__declspec(allocate(".modules$A")) void * __modules_a[] = { nullptr };
__declspec(allocate(".modules$Z")) void * __modules_z[] = { nullptr };

//
// Each obj file compiled from managed code has a .modules$I section containing a pointer to its ReadyToRun
// data (which points at eager class constructors, frozen strings, etc).
//
// The #pragma ... /merge directive folds the book-end sections and all .modules$I sections from all input
// obj files into .rdata in alphabetical order.
//
#pragma comment(linker, "/merge:.modules=.rdata")

//
// Unboxing stubs need to be merged, folded and sorted. They are delimited by two special sections (.unbox$A
// and .unbox$Z). All unboxing stubs are in .unbox$M sections.
//
#pragma comment(linker, "/merge:.unbox=.text")

char _bookend_a;
char _bookend_z;

//
// Generate bookends for the managed code section.
// We give them unique bodies to prevent folding.
//

#pragma code_seg(".managedcode$A")
void* __managedcode_a() { return &_bookend_a; }
#pragma code_seg(".managedcode$Z")
void* __managedcode_z() { return &_bookend_z; }
#pragma code_seg()

//
// Generate bookends for the unboxing stub section.
// We give them unique bodies to prevent folding.
//

#pragma code_seg(".unbox$A")
void* __unbox_a() { return &_bookend_a; }
#pragma code_seg(".unbox$Z")
void* __unbox_z() { return &_bookend_z; }
#pragma code_seg()

#endif // _MSC_VER

 */
