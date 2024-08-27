#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use napi_ohos::sys::*;
use std::ffi::*;

// libuv

pub type uv_loop_t = napi_ohos::sys::uv_loop_s;
pub type uv_req_type = u32;
pub type uv_work_t = uv_work_s;
pub type uv_work_cb = Option<unsafe extern "C" fn(req: *mut uv_work_t)>;
pub type uv_after_work_cb = Option<unsafe extern "C" fn(req: *mut uv_work_t, status: c_int)>;

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


#[link(name = "uv")]
extern "C" {
    pub fn uv_queue_work(
        loop_: *mut uv_loop_t,
        req: *mut uv_work_t,
        work_cb: uv_work_cb,
        after_work_cb: uv_after_work_cb,
    ) -> c_int;
}