use makepad_futures::channel::oneshot::Receiver;
use napi_ohos::sys::*;
use super::oh_sys::*;
use std::ffi::*;
use std::ptr::null_mut;
use std::sync::mpsc;



#[derive(Clone,Debug)]
pub enum NapiError {
    NullError,
    InvalidGlobal,
    InvalidGlobalThis,
    InvalidGlobalThisType,
    InvalidProperty,
    InvalidStringValue,
    InvalidNumberValue,
    InvalidFunction,
    InvalidObjectValue,
    UnDefinedPropertyType,
}

impl From<NulError> for NapiError {
    fn from(_value: NulError) -> Self {
        NapiError::NullError
    }
}

pub struct NapiEnv {
    raw_env: napi_env,
    obj_ref: napi_ref,
}

struct WorkArgs{
    pub env: napi_env,
    pub obj: napi_value,
    pub js_fn : napi_value,
    pub argc : usize,
    pub argv : * const napi_value,
    pub fn_name : String,
    pub is_void : bool,
    pub val_tx: Option<mpsc::Sender<napi_value>>,
    pub val_rx: Option<mpsc::Receiver<napi_value>>
}

impl NapiEnv {
    pub fn new(env: napi_env, obj:napi_ref) -> Self {
        NapiEnv{
            raw_env:env,
            obj_ref:obj,
        }
    }

    pub fn get_ref_value(&self) -> Result<napi_value, NapiError> {
        let mut result = null_mut();
        let napi_status = unsafe { napi_get_reference_value(self.raw_env, self.obj_ref, & mut result)};
        if napi_status!=Status::napi_ok {
            crate::error!("failed to get value from reference");
            return Err(NapiError::InvalidObjectValue);
        }
        return Ok(result);
    }

    fn alloca_work_t(args: WorkArgs) -> * mut uv_work_t {
        let layout = std::alloc::Layout::new::<uv_work_t>();
        let req = unsafe{std::alloc::alloc(layout) as * mut uv_work_t};
        let bargs = Box::new(args);
        unsafe { (*req).data = Box::into_raw(bargs) as * mut c_void };
        return req;
    }

    fn dealloca_work_s(req: * mut uv_work_t) {
        let layout = std::alloc::Layout::new::<uv_work_t>();
        unsafe {std::alloc::dealloc(req as * mut u8, layout)};
    }

    fn value_type_to_string(val_type: &napi_valuetype) -> String {
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

    extern "C" fn js_work_cb(_req: * mut uv_work_t) {
    }

    extern "C" fn js_after_work_cb(req: * mut uv_work_t, _status: c_int) {
        let mut result = null_mut();
        let args = unsafe { Box::from_raw((*req).data as * mut WorkArgs) };
        let napi_status = unsafe { napi_call_function(args.env, args.obj, args.js_fn, args.argc, args.argv, & mut result) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to call js function:{}", args.fn_name);
        }
        if !args.is_void {
            let _ = args.val_tx.unwrap().send(result);
        }
    }

    fn get_property(&self, name: &str) -> Result<napi_value, NapiError> {
        let cname = CString::new(name)?;
        let  mut result = null_mut();
        let object = self.get_ref_value()?;
        let napi_status = unsafe { napi_get_named_property(
            self.raw_env, object, cname.as_ptr(), & mut result)};
        if napi_status != Status::napi_ok {
            crate::error!("get property {} failed", name);
            return Err(NapiError::InvalidProperty);
        }
        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(self.raw_env, result, &mut napi_type) };
        if napi_type == ValueType::napi_undefined {
            crate::error!("property {} is undefined", name);
            return Err(NapiError::UnDefinedPropertyType);
        }
        return Ok(result);
    }

    pub fn get_string(&self, name: &str) -> Result<String,NapiError> {
        let property =  self.get_property(name)?;
        let mut len = 0;
        let napi_status = unsafe { napi_get_value_string_utf8(self.raw_env, property, null_mut(), 0, & mut len)};
        if napi_status != Status::napi_ok {
            crate::error!("failed to get string {} from napi_value", name);
            return Err(NapiError::InvalidStringValue);
        }

        len += 1;
        let mut ret = Vec::with_capacity(len);
        let buf_ptr = ret.as_mut_ptr();
        let mut written_char_count = 0;
        let napi_status = unsafe { napi_get_value_string_utf8(self.raw_env, property, buf_ptr, len, & mut written_char_count) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to get string {} from napi_value", name);
            return Err(NapiError::InvalidStringValue);
        }

        let mut ret = std::mem::ManuallyDrop::new(ret);
        let buf_ptr = ret.as_mut_ptr();
        let bytes = unsafe { Vec::from_raw_parts(buf_ptr as *mut u8, written_char_count, len) };
        match String::from_utf8(bytes) {
            Err(e) =>{
                crate::error!("failed to read utf8 string, {}", e);
                Err(NapiError::InvalidStringValue)
            },
            Ok(s) => Ok(s),
        }
    }

    pub fn get_number(&self, name: &str) -> Result<f64, NapiError> {
        let property = self.get_property( name)?;
        let mut result:f64 = 0.0;
        let napi_status = unsafe { napi_get_value_double(self.raw_env, property, & mut result) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to read double from property {}",name);
            return Err(NapiError::InvalidNumberValue);
        }
        return Ok(result);
    }

    pub fn call_js_function(&self, name: &str, argc: usize, argv: *const napi_value,) -> Result<napi_value, NapiError> {
        let property = self.get_property(name)?;
        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(self.raw_env, property, &mut napi_type) };
        if napi_type != ValueType::napi_function {
            crate::error!("{}' type expect to be function, current type is {}", name, Self::value_type_to_string(&napi_type));
            return Err(NapiError::InvalidFunction);
        }
        return Ok(property);

    }

    pub fn raw(&self) -> napi_env {
        self.raw_env
    }
}