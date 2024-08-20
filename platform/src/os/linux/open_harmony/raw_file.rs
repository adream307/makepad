#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use napi_ohos::sys::{
    napi_call_function, napi_env, napi_get_global, napi_get_named_property, napi_get_undefined,
    napi_typeof, napi_value, napi_valuetype, Status, ValueType,
};
use napi_ohos::Env;
use std::io::{Error, ErrorKind, Result};

#[repr(C)]
struct RawFile {
    _unused: [u8; 0],
}

#[repr(C)]
pub struct NativeResourceManager {
    _unused: [u8; 0],
}

#[link(name = "rawfile.z")]
extern "C" {
    fn OH_ResourceManager_InitNativeResourceManager(
        env: napi_env,
        jsResMgr: napi_value,
    ) -> *mut NativeResourceManager;
    fn OH_ResourceManager_ReleaseNativeResourceManager(
        resMgr: *mut NativeResourceManager,
    ) -> ::core::ffi::c_void;
    fn OH_ResourceManager_OpenRawFile(
        mgr: *const NativeResourceManager,
        fileName: *const ::core::ffi::c_char,
    ) -> *mut RawFile;
    fn OH_ResourceManager_GetRawFileSize(rawFile: *mut RawFile) -> ::core::ffi::c_long;
    fn OH_ResourceManager_CloseRawFile(rawFile: *mut RawFile) -> ::core::ffi::c_void;
    fn OH_ResourceManager_ReadRawFile(
        rawFile: *const RawFile,
        buf: *mut ::core::ffi::c_void,
        length: ::core::ffi::c_ulong,
    ) -> ::core::ffi::c_int;
}

pub struct RawFileMgr {
    native_resource_manager: *mut NativeResourceManager,
}

impl RawFileMgr {
    pub fn new(raw_env: napi_env, res_mgr: napi_value) -> RawFileMgr {
        let native_resource_manager =
            unsafe { OH_ResourceManager_InitNativeResourceManager(raw_env, res_mgr) };
        if native_resource_manager.is_null() {
            crate::log!("call OH_ResourceManager_InitNativeResourceManager failed");
        }
        Self {
            native_resource_manager,
        }
    }

    fn to_string(val_type: &napi_valuetype) -> String {
        match *val_type {
            ValueType::napi_undefined => "undefined".to_string(),
            ValueType::napi_null => "null".to_string(),
            ValueType::napi_boolean => "boolean".to_string(),
            ValueType::napi_number => "number".to_string(),
            ValueType::napi_string => "string".to_string(),
            ValueType::napi_symbol => "symbol".to_string(),
            ValueType::napi_object => "object".to_string(),
            ValueType::napi_function => "function".to_string(),
            ValueType::napi_external => "external".to_string(),
            _ => "undefined".to_string(),
        }
    }

    pub fn read_to_end<S: AsRef<str>>(&mut self, path: S, buf: &mut Vec<u8>) -> Result<usize> {
        if self.native_resource_manager.is_null() {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "OH_ResourceManager_InitNativeResourceManager failed",
            ));
        }
        let path_cstring = std::ffi::CString::new(path.as_ref())?;
        let raw_file = unsafe {
            OH_ResourceManager_OpenRawFile(self.native_resource_manager, path_cstring.as_ptr())
        };
        if raw_file.is_null() {
            let msg = format!("open file {} failed", path.as_ref());
            return Err(Error::new(ErrorKind::NotConnected, msg));
        }
        let file_length = unsafe { OH_ResourceManager_GetRawFileSize(raw_file) };
        if file_length <= 0 {
            let _ = unsafe { OH_ResourceManager_CloseRawFile(raw_file) };
            buf.clear();
            return Ok(0);
        }
        buf.resize(file_length.try_into().unwrap(), 0 as u8);
        let read_length = unsafe {
            OH_ResourceManager_ReadRawFile(
                raw_file,
                buf.as_ptr() as *mut ::core::ffi::c_void,
                file_length.try_into().unwrap(),
            )
        };
        if i64::from(read_length) < file_length {
            buf.resize(read_length.try_into().unwrap(), 0 as u8);
        }
        let _ = unsafe { OH_ResourceManager_CloseRawFile(raw_file) };
        return Ok(read_length.try_into().unwrap());
    }
}

impl Drop for RawFileMgr {
    fn drop(&mut self) {
        unsafe {
            OH_ResourceManager_ReleaseNativeResourceManager(self.native_resource_manager);
        }
    }
}
