#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use napi_ohos::sys::*;
use std::ffi::*;
use std::ptr::null_mut;
use std::sync::mpsc;
use super::oh_util;

// libuv

pub type uv_loop_t = napi_ohos::sys::uv_loop_s;
pub type uv_req_type = u32;
pub type uv_work_t = uv_work_s;
pub type uv_work_cb = Option<unsafe extern "C" fn(req: *mut uv_work_t)>;
pub type uv_after_work_cb = Option<unsafe extern "C" fn(req: *mut uv_work_t, status: c_int)>;
pub type uv_timer_t = uv_timer_s;
pub type uv_handle_type = u32;
pub type uv_close_cb = Option<unsafe extern "C" fn(handle: *mut uv_handle_t)>;
pub type uv_handle_t = uv_handle_s;
pub type uv_timer_cb = Option<unsafe extern "C" fn(handle: *mut uv_timer_t)>;

#[repr(C)]
pub struct uv_work_s {
    pub data: *mut c_void,
    pub type_: uv_req_type,
    pub reserved: [*mut c_void; 6],
    pub loop_: *mut uv_loop_t,
    pub work_cb: uv_work_cb,
    pub after_work_cb: uv_after_work_cb,
    pub work_req: uv__work,
}

#[repr(C)]
pub struct uv__work {
    pub work: Option<unsafe extern "C" fn(w: *mut uv__work)>,
    pub done: Option<unsafe extern "C" fn(w: *mut uv__work, status: c_int)>,
    pub loop_: *mut uv_loop_s,
    pub wq: [*mut c_void; 2],
}


#[repr(C)]
pub struct uv_timer_s {
    pub data: *mut c_void,
    pub loop_: *mut uv_loop_t,
    pub type_: uv_handle_type,
    pub close_cb: uv_close_cb,
    pub handle_queue: [*mut c_void; 2],
    pub u: uv_timer_s__bindgen_ty_1,
    pub next_closing: *mut uv_handle_t,
    pub flags: c_uint,
    pub timer_cb: uv_timer_cb,
    pub heap_node: [*mut c_void; 3],
    pub timeout: u64,
    pub repeat: u64,
    pub start_id: u64,
}

#[repr(C)]
pub struct uv_handle_s {
    pub data: *mut c_void,
    pub loop_: *mut uv_loop_t,
    pub type_: uv_handle_type,
    pub close_cb: uv_close_cb,
    pub handle_queue: [*mut c_void; 2],
    pub u: uv_handle_s__bindgen_ty_1,
    pub next_closing: *mut uv_handle_t,
    pub flags: c_uint,
}

#[repr(C)]
pub union uv_timer_s__bindgen_ty_1 {
    pub fd: c_int,
    pub reserved: [*mut c_void; 4],
    /* private fields */
}

#[repr(C)]
pub union uv_handle_s__bindgen_ty_1 {
    pub fd: c_int,
    pub reserved: [*mut c_void; 4],
    /* private fields */
}


#[link(name = "uv")]
extern "C" {
    pub fn uv_queue_work(
        loop_: *mut uv_loop_t,
        req: *mut uv_work_t,
        work_cb: uv_work_cb,
        after_work_cb: uv_after_work_cb,
    ) -> c_int;

    pub fn uv_timer_init(
        arg1: *mut uv_loop_t,
        handle: *mut uv_timer_t
    ) -> c_int;

    pub fn uv_timer_set_repeat(
        handle: *mut uv_timer_t,
        repeat: u64
    );

    pub fn uv_timer_start(
        handle: *mut uv_timer_t,
        cb: uv_timer_cb,
        timeout: u64,
        repeat: u64
    ) -> c_int;

    pub fn uv_timer_stop(
        handle: *mut uv_timer_t
    ) -> c_int;
}
#[derive(Clone, Debug)]
pub enum ArkTsObjErr {
    NullError,
    InvalidGlobal,
    InvalidGlobalThis,
    InvalidGlobalThisType,
    InvalidProperty,
    InvalidStringValue,
    InvalidNumberValue,
    InvalidFunction,
    InvalidObjectValue,
    InvalidUvLoop,
    CallJsFailed,
    UnDefinedPropertyType,
}

impl From<NulError> for ArkTsObjErr {
    fn from(_value: NulError) -> Self {
        ArkTsObjErr::NullError
    }
}

pub struct ArkTsObjRef {
    raw_env: napi_env,
    obj_ref: napi_ref,
    val_tx: mpsc::Sender<Result<napi_value, ArkTsObjErr>>,
    val_rx: mpsc::Receiver<Result<napi_value, ArkTsObjErr>>,
}

struct WorkArgs<'a> {
    pub env: &'a ArkTsObjRef,
    pub fn_name: String,
    pub argc: usize,
    pub argv: *const napi_value,
}

impl ArkTsObjRef {
    pub fn new(env: napi_env, obj: napi_ref) -> Self {
        let (tx, rx) = mpsc::channel();
        ArkTsObjRef {
            raw_env: env,
            obj_ref: obj,
            val_tx: tx,
            val_rx: rx,
        }
    }

    fn get_ref_value(&self) -> Result<napi_value, ArkTsObjErr> {
        let mut result = null_mut();
        let napi_status =
            unsafe { napi_get_reference_value(self.raw_env, self.obj_ref, &mut result) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to get value from reference");
            return Err(ArkTsObjErr::InvalidObjectValue);
        }
        return Ok(result);
    }

    fn get_loop(&self) -> Result<*mut uv_loop_s, ArkTsObjErr> {
        let mut uv_loop = std::ptr::null_mut();
        let napi_status = unsafe { napi_get_uv_event_loop(self.raw_env, &mut uv_loop) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to get uv loop from env");
            return Err(ArkTsObjErr::InvalidUvLoop);
        }
        return Ok(uv_loop);
    }

    fn alloca_work_t(args: WorkArgs) -> *mut uv_work_t {
        let layout = std::alloc::Layout::new::<uv_work_t>();
        let req = unsafe { std::alloc::alloc(layout) as *mut uv_work_t };
        let bargs = Box::new(args);
        unsafe { (*req).data = Box::into_raw(bargs) as *mut c_void };
        return req;
    }

    fn dealloca_work_s(req: *mut uv_work_t) {
        let layout = std::alloc::Layout::new::<uv_work_t>();
        unsafe { std::alloc::dealloc(req as *mut u8, layout) };
    }

    extern "C" fn js_work_cb(_req: *mut uv_work_t) {}

    extern "C" fn js_after_work_cb(req: *mut uv_work_t, _status: c_int) {
        let args = unsafe { Box::from_raw((*req).data as *mut WorkArgs) };
        let mut arkts_obj = null_mut();

        let napi_status =
            unsafe { napi_get_reference_value(args.env.raw_env, args.env.obj_ref, &mut arkts_obj) };
        if napi_status != Status::napi_ok {
            crate::error!("failed to get value from reference");
            let _ = args.env.val_tx.send(Err(ArkTsObjErr::InvalidObjectValue));
            return;
        }

        let fn_name = CString::new(args.fn_name.clone()).unwrap();
        let mut js_fn = null_mut();
        let napi_status = unsafe {
            napi_get_named_property(args.env.raw_env, arkts_obj, fn_name.as_ptr(), &mut js_fn)
        };
        if napi_status != Status::napi_ok {
            crate::error!("failed to get function {} from arkts object", args.fn_name);
            let _ = args.env.val_tx.send(Err(ArkTsObjErr::InvalidProperty));
            return;
        }

        let mut napi_type: napi_valuetype = 0;
        let _ = unsafe { napi_typeof(args.env.raw_env, js_fn, &mut napi_type) };
        if napi_type != ValueType::napi_function {
            crate::error!("property {} is not function", args.fn_name);
            let _ = args.env.val_tx.send(Err(ArkTsObjErr::InvalidFunction));
            return;
        }

        let mut call_result = null_mut();
        let napi_status = unsafe {
            napi_call_function(
                args.env.raw(),
                arkts_obj,
                js_fn,
                args.argc,
                args.argv,
                &mut call_result,
            )
        };
        if napi_status != Status::napi_ok {
            crate::error!("failed to call js function:{}", args.fn_name);
            let _ = args.env.val_tx.send(Err(ArkTsObjErr::CallJsFailed));
            return;
        }
        let _ = args.env.val_tx.send(Ok(call_result));
    }

    pub fn get_property(&self, name: &str) -> Result<napi_value, ArkTsObjErr> {
        let object = self.get_ref_value()?;
        match oh_util::get_object_property(self.raw_env, object, &name) {
            Some(val) => Ok(val),
            None => Err(ArkTsObjErr::InvalidProperty)
        }
    }

    pub fn get_string(&self, name: &str) -> Result<String, ArkTsObjErr> {
        let property = self.get_property(name)?;
        match oh_util::get_value_string(self.raw_env, property) {
            Some(val) => Ok(val),
            None => Err(ArkTsObjErr::InvalidStringValue)
        }
    }

    pub fn get_number(&self, name: &str) -> Result<f64, ArkTsObjErr> {
        let property = self.get_property(name)?;
        match oh_util::get_value_f64(self.raw_env, property) {
            Some(val) => Ok(val),
            None => Err(ArkTsObjErr::InvalidNumberValue)
        }
    }

    pub fn call_js_function(
        &self,
        name: &str,
        argc: usize,
        argv: *const napi_value,
    ) -> Result<napi_value, ArkTsObjErr> {
        let args = WorkArgs {
            env: &self,
            fn_name: name.to_string(),
            argc: argc,
            argv: argv,
        };
        let req = Self::alloca_work_t(args);
        let uv_loop = self.get_loop()?;

        let _ = unsafe {
            uv_queue_work(
                uv_loop,
                req,
                Some(Self::js_work_cb),
                Some(Self::js_after_work_cb),
            )
        };
        let ret = match self.val_rx.recv() {
            Ok(r) => r,
            Err(e) => {
                crate::error!(
                    "failed to get result for js function {}, error = {}",
                    name,
                    e
                );
                Err(ArkTsObjErr::CallJsFailed)
            }
        };
        Self::dealloca_work_s(req);
        ret
    }

    pub fn raw(&self) -> napi_env {
        self.raw_env
    }
}
