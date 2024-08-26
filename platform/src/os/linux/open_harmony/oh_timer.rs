use core::time;
use std::alloc::Layout;
use std::os::raw::c_void;

use napi_ohos::sys::napi_env;
use napi_ohos::sys::napi_value;
use super::uv_sys::*;
use super::oh_util;

struct OhUvTimer{
    timer_id : u64,
    interval :f64,
    repeats: bool,
    used: bool,
    timer: * mut uv_timer_t
}

impl OhUvTimer {
    fn as_mut_ptr(& mut self) -> * mut OhUvTimer {
        self as * mut OhUvTimer
    }
}

struct OhUvAsyncWorker{
    used: bool,
    worker: * mut uv_async_t
}

impl OhUvAsyncWorker {
    fn as_mut_ptr(& mut self) -> * mut OhUvAsyncWorker {
        self as * mut OhUvAsyncWorker
    }
    
}

struct OhTimer{
    raw_env : napi_env,
    uv_loop : *mut uv_loop_t,
    timers : Vec<OhUvTimer>,
    workers : Vec<OhUvAsyncWorker>,
}

impl Drop for OhTimer {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::new::<uv_timer_t>();
        for w in self.timers.iter() {
            if w.timer.is_null() == false {
                unsafe { std::alloc::dealloc(w.timer as *mut u8, layout) };
            }
        }
        let layout = std::alloc::Layout::new::<uv_async_t>();
        for w in self.workers.iter() {
            if w.worker.is_null() == false {
                unsafe { std::alloc::dealloc(w.worker as * mut u8, layout)};
            }
        }
    }
}

impl OhTimer{
    pub fn new(env: napi_env) ->Self {
        let uv_loop = oh_util::get_uv_loop(env).unwrap();
        OhTimer{
            raw_env:env,
            uv_loop,
            timers : Vec::new(),
            workers:Vec::new()
        }
    }

    pub fn start_timer(& mut self, timer_id : u64, interval :f64, repeats: bool) {
        let timer = self.get_unused_timer(timer_id,interval,repeats);
        let worker =  self.get_unused_worker();
        let async_w = unsafe { (*worker).worker };
        unsafe {uv_async_init(self.uv_loop, async_w, Some(Self::async_cb));}
    }

    fn get_unused_timer(& mut self, timer_id : u64, interval :f64, repeats: bool)->* mut OhUvTimer {
        for i in 0..self.timers.len() {
            if self.timers[i].used == false{
                self.timers[i].used = true;
                self.timers[i].timer_id = timer_id;
                self.timers[i].interval = interval;
                self.timers[i].repeats = repeats;
                return self.timers[i].as_mut_ptr();
            }
        }
        let layout = std::alloc::Layout::new::<uv_timer_t>();
        let w = unsafe { std::alloc::alloc(layout) } as * mut uv_timer_t;
        self.timers.push(OhUvTimer{timer_id,interval,repeats, used:true, timer:w});
        let idx = self.timers.len() - 1;
        return self.timers.get_mut(idx).unwrap().as_mut_ptr();
    }

    fn get_unused_worker(&mut self) -> * mut OhUvAsyncWorker {
        for i in 0..self.workers.len() {
            if self.workers[i].used == false {
                self.workers[i].used = true;
                let worker = self.workers[i].worker;
                return self.workers[i].as_mut_ptr();
            }
        }
        let layout = std::alloc::Layout::new::<uv_async_t>();
        let w = unsafe { std::alloc::alloc(layout) } as * mut uv_async_t;
        self.workers.push(OhUvAsyncWorker{used:true,worker:w});
        let idx = self.workers.len() - 1;
        return self.workers.get_mut(idx).unwrap().as_mut_ptr();
    }

    extern "C" fn async_cb(handle: *mut uv_async_t) {

    }

}

