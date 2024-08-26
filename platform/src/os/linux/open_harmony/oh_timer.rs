use napi_ohos::sys::napi_env;
use napi_ohos::sys::napi_value;
use super::uv_sys::*;
use super::oh_util;

struct OhTimerWorker{
    timer_id : u64,
    used: bool,
    worker: * mut uv_timer_t
}

struct OhTimer{
    raw_env : napi_env,
    uv_loop : *mut uv_loop_t,
    timers : Vec<OhTimerWorker>
}

impl Drop for OhTimer {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::new::<uv_timer_t>();
        for w in self.timers.iter() {
            if w.worker.is_null() == false {
                unsafe { std::alloc::dealloc(w.worker as *mut u8, layout) };
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
            timers : Vec::new()
        }
    }

    pub fn start_timer(timer_id : u64, interval :f64, repeats: bool) {

    }

    fn get_unused_workder(& mut self, id: u64)->i32 {
        for i in 0..self.timers.len() {
            if self.timers[i].used == false{
                self.timers[i].used = true;
                self.timers[i].timer_id = id;
                return i as i32;
            }
        }
        let layout = std::alloc::Layout::new::<uv_timer_t>();
        let w = unsafe { std::alloc::alloc(layout) } as * mut uv_timer_t;
        self.timers.push(OhTimerWorker{timer_id:id, used:true, worker:w});
        return self.timers.len() as i32 -1;
    }




}

