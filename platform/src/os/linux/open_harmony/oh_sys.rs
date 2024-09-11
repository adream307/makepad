#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::*;

#[repr(C)]
pub struct OH_NativeVSync {
    _unused: [u8; 0],
}

#[link(name = "ace_napi.z")]
#[link(name = "ace_ndk.z")]
#[link(name = "hilog_ndk.z")]
#[link(name = "native_window")]
#[link(name = "native_vsync")]
extern "C" {
    pub fn OH_NativeVSync_Create(name: *const c_char, length: c_uint) -> *mut OH_NativeVSync;
    pub fn OH_NativeVSync_Destroy(nativeVsync: *mut OH_NativeVSync) -> c_void;
    pub fn OH_NativeVSync_RequestFrame(
        nativeVsync: *mut OH_NativeVSync,
        callback: extern "C" fn(timestamp: c_longlong, data: *mut c_void),
        data: *mut c_void,
    ) -> c_int;
}
