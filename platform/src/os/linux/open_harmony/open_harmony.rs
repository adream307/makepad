use crate::{egl_sys::{EGL_GL_COLORSPACE_KHR,EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE}, event::window};

use {
    self::super::super::{gl_sys, select_timer::SelectTimers},
    self::super::{oh_event::*, oh_media::CxOpenHarmonyMedia},
    crate::{
        cx::{Cx, OpenHarmonyParams, OsType},
        cx_api::{CxOsApi, CxOsOp, OpenUrlInPlace},
        event::{Event, TimerEvent, WindowGeom},
        gpu_info::GpuPerformance,
        makepad_live_id::*,
        makepad_math::*,
        os::cx_native::EventFlow,
        //window::CxWindowPool,
        pass::CxPassParent,
        pass::{PassClearColor, PassClearDepth, PassId},
        thread::SignalToUI,
        window::CxWindowPool,
    },
    std::cell::RefCell,
    std::rc::Rc,
    std::time::Instant,
};

//----------------------

use self::super::super::egl_sys::{self, LibEgl};
use napi_derive_ohos::{module_exports, napi};
use napi_ohos::bindgen_prelude::Undefined;
use napi_ohos::threadsafe_function::{
    ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_ohos::{Env, JsFunction, JsObject, JsString, NapiRaw};
use ohos_sys::xcomponent::{
    self, OH_NativeXComponent, OH_NativeXComponent_Callback, OH_NativeXComponent_GetTouchEvent,
    OH_NativeXComponent_RegisterCallback, OH_NativeXComponent_TouchEvent,
    OH_NativeXComponent_TouchEventType,OH_NativeXComponent_GetXComponentSize
};
use std::{ffi::CString, os::raw::c_void};
use std::ptr::null;
use std::sync::mpsc;

// Todo: in the future these libraries should be added by Rust sys-crates
#[link(name = "ace_napi.z")]
#[link(name = "ace_ndk.z")]
#[link(name = "hilog_ndk.z")]
#[link(name = "native_window")]
extern "C" {}

pub struct OpenHarmonyApp {
    timers: SelectTimers,
    dpi_factor: f64,
    width: f64,
    height: f64,
    //add egl here etc
}

#[derive(Debug)]
pub enum FromOhosMessage {
    Init(OpenHarmonyInitOptions),
    SurfaceChanged {
        window: *mut c_void,
        width: i32,
        height: i32,
    },
    SurfaceCreated {
        window: *mut c_void,
        width: i32,
        height: i32,
    },
    Paint,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct OpenHarmonyInitOptions {
    pub device_type: String,
    pub os_full_name: String,
    pub display_density: f64,
}

unsafe impl Send for FromOhosMessage {}

thread_local! {
    static OHOS_MSG_TX: RefCell<Option<mpsc::Sender<FromOhosMessage>>> = RefCell::new(None);
}

fn send_from_ohos_message(message: FromOhosMessage) {
    OHOS_MSG_TX.with(|tx| {
        let mut tx = tx.borrow_mut();
        tx.as_mut().unwrap().send(message).unwrap();
    })
}

impl OpenHarmonyApp {
    fn new() -> Self {
        Self {
            dpi_factor: 1.5,
            width: 1260.0,
            height: 2954.0,
            timers: SelectTimers::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn on_surface_created_cb(xcomponent: *mut OH_NativeXComponent, window: *mut c_void) {
    let mut width :u64 = 0;
    let mut height :u64 = 0;

    let ret = unsafe {OH_NativeXComponent_GetXComponentSize(
        xcomponent,
        window,
        & mut width,
        & mut height)};

    crate::log!("OnSurfaceCreateCallBack,OH_NativeXComponent_GetXComponentSize={},width={},hight={}",ret,width,height);
    send_from_ohos_message(FromOhosMessage::SurfaceCreated { window, width: width as i32, height:height as i32 });
}

#[no_mangle]
pub extern "C" fn on_surface_changed_cb(xcomponent: *mut OH_NativeXComponent, window: *mut c_void) {
    let mut width :u64 = 0;
    let mut height :u64 = 0;

    let ret = unsafe {OH_NativeXComponent_GetXComponentSize(
        xcomponent,
        window,
        & mut width,
        & mut height)};

    crate::log!("OnSurfaceChangeCallBack,OH_NativeXComponent_GetXComponentSize={},width={},hight={}",ret,width,height);
    send_from_ohos_message(FromOhosMessage::SurfaceChanged { window, width: width as i32, height:height as i32 });
}

#[no_mangle]
pub extern "C" fn on_surface_destroyed_cb(
    component: *mut OH_NativeXComponent,
    window: *mut c_void,
) {
    crate::log!("OnSurcefaceDestroyCallBack");
}

#[no_mangle]
pub extern "C" fn on_dispatch_touch_event_cb(
    component: *mut OH_NativeXComponent,
    window: *mut c_void,
) {
    crate::log!("OnDispatchTouchEventCallBack");
}

#[napi]
pub fn init_makepad(init_opts: OpenHarmonyInitOptions) -> napi_ohos::Result<()>{
    send_from_ohos_message(FromOhosMessage::Init(init_opts));
    Ok(())
}

impl Cx {
    fn main_loop(&mut self, from_ohos_rx:mpsc::Receiver<FromOhosMessage>){
        let mut app = OpenHarmonyApp::new();
        //app.dpi_factor = self.os.dpi_factor;
        app.width = self.os.display_size.x;
        app.height = self.os.display_size.y;

        self.gpu_info.performance = GpuPerformance::Tier1;


        self.call_event_handler(&Event::Startup);
        self.redraw_all();

        while !self.os.quit {
            std::thread::sleep(std::time::Duration::from_millis(100));
            self.draw_pain(&mut app)

        }

    }

    fn handle_message(&mut self, msg: FromOhosMessage){

    }

    pub fn ohos_init<F>(exports: JsObject, env: Env, startup: F)
    where
        F: FnOnce() -> Box<Cx> + Send + 'static,
    {
        std::panic::set_hook(Box::new(|info| {
            crate::log!("Custom panic hook: {}", info);
        }));

        if let Ok(xcomponent) = exports.get_named_property::<JsObject>("__NATIVE_XCOMPONENT_OBJ__")
        {
            let _ = Cx::register_xcomponent_callbacks(&env, &xcomponent);

            let (from_ohos_tx, from_ohos_rx) = mpsc::channel();
            OHOS_MSG_TX.with(move |message_tx| *message_tx.borrow_mut() = Some(from_ohos_tx));

            std::thread::spawn(move || {
                let mut cx = startup();
                let mut libegl = LibEgl::try_load().expect("Cant load LibEGL");
                let window = loop {
                    match from_ohos_rx.try_recv() {
                        Ok(FromOhosMessage::Init(params)) => {
                            cx.os.dpi_factor = params.display_density;
                            cx.os_type = OsType::OpenHarmony(OpenHarmonyParams { device_type: params.device_type, os_full_name: params.os_full_name, display_density: params.display_density });
                        }
                        Ok(FromOhosMessage::SurfaceCreated {
                            window,
                            width,
                            height,
                        }) => {
                            cx.os.display_size = dvec2(width as f64, height as f64);
                            break window;
                        }
                        _ => {}
                    }
                };
                let (egl_context, egl_config, egl_display ) = unsafe {
                    egl_sys::create_egl_context(&mut libegl).expect("Can't create EGL context")};
                unsafe { gl_sys::load_with(|s| {
                    let s = CString::new(s).unwrap();
                    libegl.eglGetProcAddress.unwrap()(s.as_ptr())
                })};

                let win_attr = vec![EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE];
                let surface = unsafe {(libegl.eglCreateWindowSurface.unwrap())(
                    egl_display,
                    egl_context,
                    window as _,
                    win_attr.as_ptr() as _
                )};

                if unsafe {(libegl.eglMakeCurrent.unwrap())(egl_display,surface,surface,egl_context)}==0{
                    panic!();
                }

                cx.os.display = Some(CxOhosDisplay{
                    libegl,
                    egl_display,
                    egl_config,
                    egl_context,surface,
                    window
                });

                cx.main_loop(from_ohos_rx);
                //TODO, destroy surface
            });


        } else {
            crate::log!("Failed to get xcomponent in ohos_init");
        }
    }

    fn register_xcomponent_callbacks(env: &Env, xcomponent: &JsObject) -> napi_ohos::Result<()> {
        crate::log!("reginter xcomponent callbacks");
        let raw = unsafe { xcomponent.raw() };
        let raw_env = env.raw();
        let mut native_xcomponent: *mut OH_NativeXComponent = core::ptr::null_mut();
        unsafe {
            let res = napi_ohos::sys::napi_unwrap(
                raw_env,
                raw,
                &mut native_xcomponent as *mut *mut OH_NativeXComponent as *mut *mut c_void,
            );
            assert!(res == 0);
        }
        crate::log!("Got native_xcomponent!");
        let cbs = Box::new(OH_NativeXComponent_Callback {
            OnSurfaceCreated: Some(on_surface_created_cb),
            OnSurfaceChanged: Some(on_surface_changed_cb),
            OnSurfaceDestroyed: Some(on_surface_destroyed_cb),
            DispatchTouchEvent: Some(on_dispatch_touch_event_cb),
        });
        let res = unsafe {
            OH_NativeXComponent_RegisterCallback(native_xcomponent, Box::leak(cbs) as *mut _)
        };
        if res != 0 {
            crate::error!("Failed to register callbacks");
        } else {
            crate::log!("Registerd callbacks successfully");
        }
        Ok(())
    }

    // pub fn event_loop(cx: Rc<RefCell<Cx>>) {
    //     let mut cx = cx.borrow_mut();

    //     //cx.os_type = OsType::OpenHarmony(OpenHarmonyParams {});
    //     cx.gpu_info.performance = GpuPerformance::Tier1;

    //     cx.call_event_handler(&Event::Startup);
    //     cx.redraw_all();

    //     let mut app = OpenHarmonyApp::new();
    //     app.timers.start_timer(0, 0.008, true);
    //     // lets run the kms eventloop
    //     let mut event_flow = EventFlow::Poll;
    //     let mut timer_ids = Vec::new();

    //     while event_flow != EventFlow::Exit {
    //         if event_flow == EventFlow::Wait {
    //             //    kms_app.timers.select(signal_fds[0]);
    //         }
    //         app.timers.update_timers(&mut timer_ids);
    //         let time = app.timers.time_now();
    //         for timer_id in &timer_ids {
    //             cx.oh_event_callback(
    //                 &mut app,
    //                 OpenHarmonyEvent::Timer(TimerEvent {
    //                     timer_id: *timer_id,
    //                     time: Some(time),
    //                 }),
    //             );
    //         }
    //         /*let input_events = direct_app.raw_input.poll_raw_input(
    //             direct_app.timers.time_now(),
    //             CxWindowPool::id_zero()
    //         );
    //         for event in input_events {
    //             cx.direct_event_callback(
    //                 &mut direct_app,
    //                 event
    //             );
    //         }*/

    //         event_flow = cx.oh_event_callback(&mut app, OpenHarmonyEvent::Paint);
    //     }
    // }

    fn draw_pain(&mut self, app: &mut OpenHarmonyApp) {
        self.call_draw_event();
        self.opengl_compile_shaders();
        self.handle_repaint(app);
    }

    // fn oh_event_callback(
    //     &mut self,
    //     app: &mut OpenHarmonyApp,
    //     event: OpenHarmonyEvent,
    // ) -> EventFlow {
    //     if let EventFlow::Exit = self.handle_platform_ops(app) {
    //         return EventFlow::Exit;
    //     }

    //     //self.process_desktop_pre_event(&mut event);
    //     match event {
    //         OpenHarmonyEvent::Paint => {
    //             //let p = profile_start();
    //             if self.new_next_frames.len() != 0 {
    //                 self.call_next_frame_event(app.timers.time_now());
    //             }
    //             if self.need_redrawing() {
    //                 self.call_draw_event();
    //                 //direct_app.egl.make_current();
    //                 self.opengl_compile_shaders();
    //             }
    //             // ok here we send out to all our childprocesses
    //             //profile_end("paint event handling", p);
    //             //let p = profile_start();
    //             self.handle_repaint(app);
    //             //profile_end("paint openGL", p);
    //         }
    //         OpenHarmonyEvent::MouseDown(e) => {
    //             self.fingers.process_tap_count(e.abs, e.time);
    //             self.fingers.mouse_down(e.button, CxWindowPool::id_zero());
    //             self.call_event_handler(&Event::MouseDown(e.into()))
    //         }
    //         OpenHarmonyEvent::MouseMove(e) => {
    //             self.call_event_handler(&Event::MouseMove(e.into()));
    //             self.fingers.cycle_hover_area(live_id!(mouse).into());
    //             self.fingers.switch_captures();
    //         }
    //         OpenHarmonyEvent::MouseUp(e) => {
    //             let button = e.button;
    //             self.call_event_handler(&Event::MouseUp(e.into()));
    //             self.fingers.mouse_up(button);
    //             self.fingers.cycle_hover_area(live_id!(mouse).into());
    //         }
    //         OpenHarmonyEvent::Scroll(e) => self.call_event_handler(&Event::Scroll(e.into())),
    //         OpenHarmonyEvent::KeyDown(e) => {
    //             self.keyboard.process_key_down(e.clone());
    //             self.call_event_handler(&Event::KeyDown(e))
    //         }
    //         OpenHarmonyEvent::KeyUp(e) => {
    //             self.keyboard.process_key_up(e.clone());
    //             self.call_event_handler(&Event::KeyUp(e))
    //         }
    //         OpenHarmonyEvent::TextInput(e) => self.call_event_handler(&Event::TextInput(e)),
    //         OpenHarmonyEvent::Timer(e) => {
    //             if e.timer_id == 0 {
    //                 if SignalToUI::check_and_clear_ui_signal() {
    //                     self.handle_media_signals();
    //                     self.call_event_handler(&Event::Signal);
    //                 }
    //             } else {
    //                 self.call_event_handler(&Event::Timer(e))
    //             }
    //         }
    //     }
    //     if self.any_passes_dirty() || self.need_redrawing() || self.new_next_frames.len() != 0 {
    //         EventFlow::Poll
    //     } else {
    //         EventFlow::Wait
    //     }
    // }

    pub fn draw_pass_to_fullscreen(&mut self, pass_id: PassId, app: &mut OpenHarmonyApp) {
        let draw_list_id = self.passes[pass_id].main_draw_list_id.unwrap();

        self.setup_render_pass(pass_id);

        // keep repainting in a loop
        //self.passes[pass_id].paint_dirty = false;

        unsafe {
            //direct_app.egl.make_current();
            gl_sys::Viewport(0, 0, app.width as i32, app.height as i32);
        }

        let clear_color = if self.passes[pass_id].color_textures.len() == 0 {
            self.passes[pass_id].clear_color
        } else {
            match self.passes[pass_id].color_textures[0].clear_color {
                PassClearColor::InitWith(color) => color,
                PassClearColor::ClearWith(color) => color,
            }
        };
        let clear_depth = match self.passes[pass_id].clear_depth {
            PassClearDepth::InitWith(depth) => depth,
            PassClearDepth::ClearWith(depth) => depth,
        };

        if !self.passes[pass_id].dont_clear {
            unsafe {
                gl_sys::BindFramebuffer(gl_sys::FRAMEBUFFER, 0);
                gl_sys::ClearDepthf(clear_depth as f32);
                gl_sys::ClearColor(clear_color.x, clear_color.y, clear_color.z, clear_color.w);
                gl_sys::Clear(gl_sys::COLOR_BUFFER_BIT | gl_sys::DEPTH_BUFFER_BIT);
            }
        }
        Self::set_default_depth_and_blend_mode();

        let mut zbias = 0.0;
        let zbias_step = self.passes[pass_id].zbias_step;

        self.render_view(pass_id, draw_list_id, &mut zbias, zbias_step);

        //unsafe {
        //direct_app.drm.swap_buffers_and_wait(&direct_app.egl);
        //}
    }

    pub(crate) fn handle_repaint(&mut self, app: &mut OpenHarmonyApp) {
        let mut passes_todo = Vec::new();
        self.compute_pass_repaint_order(&mut passes_todo);
        self.repaint_id += 1;
        for pass_id in &passes_todo {
            self.passes[*pass_id].set_time(app.timers.time_now() as f32);
            match self.passes[*pass_id].parent.clone() {
                CxPassParent::Window(_window_id) => {
                    self.draw_pass_to_fullscreen(*pass_id, app);
                }
                CxPassParent::Pass(_) => {
                    self.draw_pass_to_magic_texture(*pass_id);
                }
                CxPassParent::None => {
                    self.draw_pass_to_magic_texture(*pass_id);
                }
            }
        }
    }

}

impl CxOsApi for Cx {
    fn init_cx_os(&mut self) {
        self.live_expand();
        self.live_scan_dependencies();
        self.native_load_dependencies();
    }

    fn spawn_thread<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        std::thread::spawn(f);
    }

    fn open_url(&mut self, _url: &str, _in_place: OpenUrlInPlace) {
        crate::error!("open_url not implemented on this platform");
    }

    fn seconds_since_app_start(&self) -> f64 {
        Instant::now()
            .duration_since(self.os.start_time)
            .as_secs_f64()
    }
}

pub struct CxOhosDisplay {
    libegl: LibEgl,
    egl_display: egl_sys::EGLDisplay,
    egl_config: egl_sys::EGLConfig,
    egl_context: egl_sys::EGLContext,
    surface: egl_sys::EGLSurface,
    window: *mut c_void, //event_handler: Box<dyn EventHandler>,
}

pub struct CxOs {
    pub display_size: DVec2,
    pub dpi_factor: f64,
    pub media: CxOpenHarmonyMedia,
    pub quit : bool,
    pub(crate) start_time: Instant,
    pub(crate) display : Option<CxOhosDisplay>,

}

impl Default for CxOs {
    fn default() -> Self {
        Self {
            display_size: dvec2(100 as f64, 100 as f64),
            dpi_factor: 1.5,
            media: Default::default(),
            quit: false,
            start_time: Instant::now(),
            display:None
        }
    }
}
