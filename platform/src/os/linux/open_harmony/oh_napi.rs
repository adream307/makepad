use napi_ohos::sys::*;
use std::{ffi::CString, ffi::NulError,  ptr::null_mut};



#[derive(Clone,Debug)]
pub enum NapiError {
    NullError,
    InvalidGlobal,
    InvalidGlobalThis,
    InvalidGlobalThisType,
    InvalidProperty,
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

    pub fn raw(&self) -> napi_env {
        self.0
    }
}

