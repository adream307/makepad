#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use napi_ohos::sys::{
    ValueType,
    Status,
    napi_env,
    napi_value,
    napi_valuetype,
    napi_typeof,
    napi_get_global,
    napi_get_named_property,
    napi_call_function,
    napi_get_undefined
};
use std::io::{Error, Result, ErrorKind};
use napi_ohos::Env;

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
    fn OH_ResourceManager_InitNativeResourceManager(env: napi_env, jsResMgr: napi_value) -> *mut NativeResourceManager;
    fn OH_ResourceManager_ReleaseNativeResourceManager(resMgr: * mut NativeResourceManager) -> ::core::ffi::c_void;
    fn OH_ResourceManager_OpenRawFile (mgr :* const NativeResourceManager, fileName: * const ::core::ffi::c_char) -> * mut RawFile;
    fn OH_ResourceManager_GetRawFileSize (rawFile: * mut RawFile) -> ::core::ffi::c_long;
    fn OH_ResourceManager_CloseRawFile (rawFile: * mut RawFile) -> ::core::ffi::c_void;
    fn OH_ResourceManager_ReadRawFile (rawFile: * const RawFile, buf: * mut ::core::ffi::c_void, length: ::core::ffi::c_ulong) -> ::core::ffi::c_int;
}

pub struct RawFileMgr {
    native_resource_manager: *mut NativeResourceManager,
}

impl RawFileMgr {
    pub fn new(raw_env:napi_env, res_mgr:napi_value)->RawFileMgr {
        let native_resource_manager = unsafe { OH_ResourceManager_InitNativeResourceManager(raw_env,res_mgr) };
        if native_resource_manager.is_null() {
            crate::log!("call OH_ResourceManager_InitNativeResourceManager failed");
        }
        Self{
            native_resource_manager
        }
    }

    fn to_string(val_type : &napi_valuetype) -> String {
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
            _ => "undefined".to_string()
        }
    }
    
    pub fn get_resource_manager(env: &Env) -> Option<(napi_env, napi_value)> {
        let raw_env = env.raw();
        let mut global_obj = std::ptr::null_mut();
        let napi_status =  unsafe { napi_get_global(raw_env, & mut global_obj)};
        if napi_status != Status::napi_ok {
            crate::log!("get global from env failed, error code = {}",napi_status);
            return None;
        }
        crate::log!("get global from env success");
    
        let mut global_this = std::ptr::null_mut();
        let napi_status = unsafe { napi_get_named_property(raw_env, global_obj, c"globalThis".as_ptr(), & mut global_this )};
        if napi_status != Status::napi_ok {
            crate::log!("get globalThis from global failed, error code = {}",napi_status);
            return None;
        }
        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(raw_env,global_this,& mut napi_type) };
        if napi_type != ValueType::napi_object {
            crate::log!("globalThis expect to be object, current data type = {}",Self::to_string(&napi_type));
            return None;
        }
        crate::log!("get globalThis from global success");
    
        let mut get_context_fn = std::ptr::null_mut();
        let napi_status = unsafe { napi_get_named_property(raw_env, global_this, c"getContext".as_ptr(), & mut get_context_fn)};
        if napi_status != Status::napi_ok {
            crate::log!("get getContext from globalThis failed, error code = {}",napi_status);
            return None;
        }
        napi_type = 0;
        let _ = unsafe { napi_typeof(raw_env,get_context_fn,& mut napi_type) };
        if napi_type != ValueType::napi_function {
            crate::log!("getContext expect to be function, current data type = {}",Self::to_string(&napi_type));
            return None;
        }
        crate::log!("get getContext function success");
    
        let mut ctx_recv = std::ptr::null_mut();
        unsafe { let _ = napi_get_undefined(raw_env, &mut ctx_recv); }
        let mut get_context_result = std::ptr::null_mut();
        let napi_status = unsafe {napi_call_function(raw_env, ctx_recv, get_context_fn, 0, std::ptr::null(), & mut get_context_result)};
        if napi_status != Status::napi_ok {
            crate::log!("call getContext() failed, error code = {}",napi_status);
            return None;
        }
        napi_type = 0;
        let _ = unsafe { napi_typeof(raw_env,get_context_result,& mut napi_type) };
        if napi_type != ValueType::napi_object {
            crate::log!("getContext() result expect to be object, current data type = {}",Self::to_string(&napi_type));
            return None;
        }
        crate::log!("call getContext() succcess");
    
        let mut res_mgr = std::ptr::null_mut();
        let napi_status = unsafe { napi_get_named_property(raw_env, get_context_result, c"resourceManager".as_ptr(), & mut res_mgr)};
        if napi_status != Status::napi_ok {
            crate::log!("get  resourceManager failed, error code = {}", napi_status);
            return None;
        }
        let _ = unsafe { napi_typeof(raw_env,res_mgr,& mut napi_type) };
        if napi_type == ValueType::napi_undefined {
            crate::log!("resourceManager could not be undefined, error code");
            return None;
        }
        crate::log!("get resourceManager success");
        return Some((raw_env, res_mgr));
    }

    pub fn read_to_end<S: AsRef<str>>(&mut self,path: S, buf: &mut Vec<u8>) -> Result<usize> {
        if self.native_resource_manager.is_null() {
            return Err(Error::new(ErrorKind::NotConnected,"OH_ResourceManager_InitNativeResourceManager failed"));
        }
        let raw_file = unsafe { OH_ResourceManager_OpenRawFile(self.native_resource_manager, path.as_ref().as_ptr()) };
        if raw_file.is_null() {
            let msg = format!("open file {} failed", path.as_ref());
            return Err(Error::new(ErrorKind::NotConnected,msg));
        }
        let file_length = unsafe { OH_ResourceManager_GetRawFileSize(raw_file) };
        if file_length <= 0 {
            let _ = unsafe { OH_ResourceManager_CloseRawFile(raw_file)};
            buf.clear();
            return Ok(0);
        }
        buf.resize(file_length.try_into().unwrap(), 0 as u8);
        let read_length =  unsafe { OH_ResourceManager_ReadRawFile(raw_file, buf.as_ptr() as * mut ::core::ffi::c_void, file_length.try_into().unwrap())};
        if i64::from(read_length) < file_length {
            buf.resize(read_length.try_into().unwrap(), 0 as u8);
        }
        let _ = unsafe { OH_ResourceManager_CloseRawFile(raw_file)};
        return Ok(read_length.try_into().unwrap());
    }
}

impl Drop for RawFileMgr {
    fn drop(&mut self) {
        unsafe { OH_ResourceManager_ReleaseNativeResourceManager(self.native_resource_manager);}
    }
}




