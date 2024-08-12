#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[repr(C)]
pub struct OH_NativeVSync {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct NativeResourceManager {
    _unused: [u8; 0],
}

#[link(name = "ace_napi.z")]
#[link(name = "ace_ndk.z")]
#[link(name = "hilog_ndk.z")]
#[link(name = "rawfile.z")]
#[link(name = "native_window")]
#[link(name = "native_vsync")]
extern "C" {
    pub fn OH_NativeVSync_Create(
        name: *const ::core::ffi::c_char,
        length: ::core::ffi::c_uint,
    ) -> *mut OH_NativeVSync;
    pub fn OH_NativeVSync_Destroy(nativeVsync: *mut OH_NativeVSync) -> ::core::ffi::c_void;
    pub fn OH_NativeVSync_RequestFrame(
        nativeVsync: *mut OH_NativeVSync,
        callback: extern "C" fn(timestamp: ::core::ffi::c_longlong, data: *mut ::core::ffi::c_void),
        data: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    pub fn OH_NativeVSync_GetPeriod(
        nativeVsync: *mut OH_NativeVSync,
        period: *mut ::core::ffi::c_longlong,
    ) -> ::core::ffi::c_int;

    pub fn OH_ResourceManager_InitNativeResourceManager(env: napi_ohos::sys::napi_env, jsResMgr: napi_ohos::sys::napi_value) -> *mut NativeResourceManager;
}
