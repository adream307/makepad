use napi_ohos::sys::*;
use std::{ffi::CString, ffi::NulError,  ptr::null_mut};



#[derive(Clone,Debug)]
pub enum NapiError {
    NullError,
    InvalidGlobal,
    InvalidGlobalThis,
    InvalidGlobalThisType,
    InvalidProperty,
    InvalidStringValue,
    UnDefinedPropertyType,
}

impl From<NulError> for NapiError {
    fn from(_value: NulError) -> Self {
        NapiError::NullError
    }
}

pub struct NapiEnv(pub(crate) napi_env);

impl From<napi_env> for NapiEnv {
    fn from(env: napi_env) -> Self {
      NapiEnv(env)
    }
}

impl NapiEnv {

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

    fn get_global_this(&self) -> Result<napi_value,NapiError> {
        let mut global_obj = null_mut();
        let napi_status = unsafe { napi_get_global(self.0, & mut global_obj)};
        if napi_status != Status::napi_ok {
            crate::error!("get global from env failed, error code = {}", napi_status);
            return Err(NapiError::InvalidGlobal);
        }

        let mut global_this = null_mut();
        let napi_status = unsafe {
            napi_get_named_property(
                self.0,
                global_obj,
                c"globalThis".as_ptr(),
                &mut global_this,
            )
        };
        if napi_status != Status::napi_ok {
            crate::error!(
                "get globalThis from global failed, error code = {}",
                napi_status
            );
            return Err(NapiError::InvalidGlobalThis);
        }

        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(self.0, global_this, &mut napi_type) };
        if napi_type != ValueType::napi_object {
            crate::error!(
                "globalThis expect to be object, current data type = {}",
                Self::value_type_to_string(&napi_type)
            );
            return Err(NapiError::InvalidGlobalThisType);
        }
        return Ok(global_this);
    }

    fn get_property(&self, object: napi_value, name: &str) -> Result<napi_value, NapiError> {
        let cname = CString::new(name)?;
        let  mut result = null_mut();
        let napi_status = unsafe { napi_get_named_property(
            self.0, object, cname.as_ptr(), & mut result)};
        if napi_status != Status::napi_ok {
            crate::error!("get property {} failed", name);
            return Err(NapiError::InvalidProperty);
        }
        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(self.0, result, &mut napi_type) };
        if napi_type == ValueType::napi_undefined {
            crate::error!("property {} is undefined", name);
            return Err(NapiError::UnDefinedPropertyType);
        }
        return Ok(result);
    }

    fn get_string(&self, object: napi_value, name: &str) -> Result<String,NapiError> {
        let property =  self.get_property(object, name)?;
        let mut len = 0;
        let napi_status = unsafe { napi_get_value_string_utf8(self.0, property, null_mut(), 0, & mut len)};
        if napi_status != Status::napi_ok {
            crate::error!("failed to get string {} from napi_value", name);
            return Err(NapiError::InvalidStringValue);
        }

        len += 1;
        let mut ret = Vec::with_capacity(len);
        let buf_ptr = ret.as_mut_ptr();
        let mut written_char_count = 0;
        let napi_status = unsafe { napi_get_value_string_utf8(self.0, property, buf_ptr, len, & mut written_char_count) };
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

    pub fn raw(&self) -> napi_env {
        self.0
    }
}

