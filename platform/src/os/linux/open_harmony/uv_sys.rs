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
pub type uv_timer_t = uv_timer_s;
pub type uv_handle_type = u32;
pub type uv_close_cb = Option<unsafe extern "C" fn(handle: *mut uv_handle_t)>;
pub type uv_handle_t = uv_handle_s;
pub type uv_timer_cb = Option<unsafe extern "C" fn(handle: *mut uv_timer_t)>;
pub type uv_async_t = uv_async_s;
pub type uv_async_cb = Option<unsafe extern "C" fn(handle: *mut uv_async_t)>;

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

#[repr(C)]
pub struct uv_async_s {
    pub data: *mut c_void,
    pub loop_: *mut uv_loop_t,
    pub type_: uv_handle_type,
    pub close_cb: uv_close_cb,
    pub handle_queue: [*mut c_void; 2],
    pub u: uv_async_s__bindgen_ty_1,
    pub next_closing: *mut uv_handle_t,
    pub flags: c_uint,
    pub async_cb: uv_async_cb,
    pub queue: [*mut c_void; 2],
    pub pending: c_int,
}

#[repr(C)]
pub union uv_async_s__bindgen_ty_1 {
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