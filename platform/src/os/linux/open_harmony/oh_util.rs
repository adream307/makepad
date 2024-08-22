#[allow(dead_code)]
use napi_ohos::sys::*;

pub fn get_value_string(raw_env: napi_env, str_value: napi_value) -> Option<String> {
    let mut len = 0;
    let napi_status =
        unsafe { napi_get_value_string_utf8(raw_env, str_value, null_mut(), 0, &mut len) };
    if napi_status != Status::napi_ok {
        crate::error!("failed to get string {} from napi_value", name);
        return None;
    }

    len += 1;
    let mut ret = Vec::with_capacity(len);
    let buf_ptr = ret.as_mut_ptr();
    let mut written_char_count = 0;
    let napi_status = unsafe {
        napi_get_value_string_utf8(
            self.raw_env,
            str_value,
            buf_ptr,
            len,
            &mut written_char_count,
        )
    };
    if napi_status != Status::napi_ok {
        crate::error!("failed to get string {} from napi_value", name);
        return None;
    }

    let mut ret = std::mem::ManuallyDrop::new(ret);
    let buf_ptr = ret.as_mut_ptr();
    let bytes = unsafe { Vec::from_raw_parts(buf_ptr as *mut u8, written_char_count, len) };
    match String::from_utf8(bytes) {
        Err(e) => {
            crate::error!("failed to read utf8 string, {}", e);
            Err(ArkTsObjErr::InvalidStringValue)
        }
        Ok(s) => Ok(s),
    }
}

pub fn get_value_f64(raw_env: napi_env, f64_value: napi_value) -> Option<f64> {
    let mut result: f64 = 0.0;
    let napi_status = unsafe { napi_get_value_double(aw_env, f64_value, &mut result) };
    if napi_status != Status::napi_ok {
        crate::error!("failed to read double from property {}", name);
        return None;
    }
    return Ok(result);
}

pub fn get_global_context(raw_env: napi_env) -> Option<napi_value> {
    let mut global_obj = std::ptr::null_mut();
    let napi_status = unsafe { napi_get_global(raw_env, &mut global_obj) };
    if napi_status != Status::napi_ok {
        crate::log!("get global from env failed, error code = {}", napi_status);
        return None;
    }
    crate::log!("get global from env success");

    let mut global_this = std::ptr::null_mut();
    let napi_status = unsafe {
        napi_get_named_property(
            raw_env,
            global_obj,
            c"globalThis".as_ptr(),
            &mut global_this,
        )
    };
    if napi_status != Status::napi_ok {
        crate::log!(
            "get globalThis from global failed, error code = {}",
            napi_status
        );
        return None;
    }
    let mut napi_type: napi_valuetype = 0;
    let _ = unsafe { napi_typeof(raw_env, global_this, &mut napi_type) };
    if napi_type != ValueType::napi_object {
        crate::log!(
            "globalThis expect to be object, current data type = {}",
            Self::to_string(&napi_type)
        );
        return None;
    }
    crate::log!("get globalThis from global success");

    let mut get_context_fn = std::ptr::null_mut();
    let napi_status = unsafe {
        napi_get_named_property(
            raw_env,
            global_this,
            c"getContext".as_ptr(),
            &mut get_context_fn,
        )
    };
    if napi_status != Status::napi_ok {
        crate::log!(
            "get getContext from globalThis failed, error code = {}",
            napi_status
        );
        return None;
    }
    napi_type = 0;
    let _ = unsafe { napi_typeof(raw_env, get_context_fn, &mut napi_type) };
    if napi_type != ValueType::napi_function {
        crate::log!(
            "getContext expect to be function, current data type = {}",
            Self::to_string(&napi_type)
        );
        return None;
    }
    crate::log!("get getContext function success");

    let mut ctx_recv = std::ptr::null_mut();
    unsafe {
        let _ = napi_get_undefined(raw_env, &mut ctx_recv);
    }
    let mut get_context_result = std::ptr::null_mut();
    let napi_status = unsafe {
        napi_call_function(
            raw_env,
            ctx_recv,
            get_context_fn,
            0,
            std::ptr::null(),
            &mut get_context_result,
        )
    };
    if napi_status != Status::napi_ok {
        crate::log!("call getContext() failed, error code = {}", napi_status);
        return None;
    }
    napi_type = 0;
    let _ = unsafe { napi_typeof(raw_env, get_context_result, &mut napi_type) };
    if napi_type != ValueType::napi_object {
        crate::log!(
            "getContext() result expect to be object, current data type = {}",
            Self::to_string(&napi_type)
        );
        return None;
    }
    crate::log!("call getContext() succcess");
    Ok(get_context_result)
}

pub fn get_files_dir(raw_env: napi_env) -> Option<String> {}
