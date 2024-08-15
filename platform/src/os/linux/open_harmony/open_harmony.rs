use {
    self::super::{
        super::{gl_sys, select_timer::SelectTimers},
        oh_callbacks::*,
        oh_media::CxOpenHarmonyMedia,
        raw_file::*,
    },
    crate::{
        cx::{Cx, OpenHarmonyParams, OsType},
        cx_api::{CxOsApi, CxOsOp, OpenUrlInPlace},
        egl_sys::{self, LibEgl, EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE},
        event::{Event, TouchUpdateEvent, WindowGeom},
        gpu_info::GpuPerformance,
        makepad_math::*,
        os::cx_native::EventFlow,
        pass::{CxPassParent, PassClearColor, PassClearDepth, PassId},
        thread::SignalToUI,
        window::CxWindowPool,
    },
    napi_derive_ohos::napi,
    napi_ohos::{sys::*, Env, JsObject, NapiRaw},
    std::{ffi::CString, os::raw::c_void, rc::Rc, sync::mpsc, time::Instant},
};

#[napi]
pub fn init_makepad(env: Env, init_opts: OpenHarmonyInitOptions, ark_ts: JsObject) -> napi_ohos::Result<()> {
    crate::log!(
        "call initMakePad from XComponent.onLoad, display_density = {}",
        init_opts.display_density
    );
    let (raw_env, res_mgr) = match RawFileMgr::get_resource_manager(&env) {
        Some((raw_env, res_mgr)) => (raw_env, res_mgr),
        None => (std::ptr::null_mut(), std::ptr::null_mut()),
    };

    let raw_ark = unsafe { ark_ts.raw() };
    // let mut show = std::ptr::null_mut();
    // let status = unsafe { napi_get_named_property(raw_env, raw_ark, c"showInputText".as_ptr(), & mut show) };
    // assert!(status == 0);

    // let mut napi_type: napi_valuetype = 0;
    // let _ = unsafe { napi_typeof(raw_env, show, &mut napi_type) };
    // assert!(napi_type == napi_ohos::sys::ValueType::napi_function);

    let mut arkts_ref = std::ptr::null_mut();
    let status = unsafe { napi_create_reference(raw_env, raw_ark, 1,  & mut arkts_ref) };
    assert!(status == 0);

    crate::log!("get showInputText from object");

    send_from_ohos_message(FromOhosMessage::Init {
        option: init_opts,
        raw_env,
        arkts_ref,
        res_mgr,
    });
    Ok(())
}

impl Cx {
    fn main_loop(&mut self, from_ohos_rx: mpsc::Receiver<FromOhosMessage>) {
        crate::log!("entry main_loop");

        self.gpu_info.performance = GpuPerformance::Tier1;

        self.call_event_handler(&Event::Startup);
        self.redraw_all();

        while !self.os.quit {
            match from_ohos_rx.recv() {
                Ok(FromOhosMessage::VSync) => {
                    self.handle_all_pending_messages(&from_ohos_rx);
                    self.handle_other_events();
                    self.handle_drawing();
                }
                Ok(message) => self.handle_message(message),
                Err(e) => {
                    crate::error!("Error receiving message: {:?}", e);
                }
            }
        }
    }

    fn handle_all_pending_messages(&mut self, from_ohos_rx: &mpsc::Receiver<FromOhosMessage>) {
        // Handle the messages that arrived during the last frame
        while let Ok(msg) = from_ohos_rx.try_recv() {
            self.handle_message(msg);
        }
    }

    fn handle_other_events(&mut self) {
        // Timers
        // for event in self.os.timers.get_dispatch() {
        //     self.call_event_handler(&event);
        // }

        // Signals
        if SignalToUI::check_and_clear_ui_signal() {
            self.handle_media_signals();
            self.call_event_handler(&Event::Signal);
        }

        // Video updates
        // let to_dispatch = self.get_video_updates();
        // for video_id in to_dispatch {
        //     let e = Event::VideoTextureUpdated(
        //         VideoTextureUpdatedEvent {
        //             video_id,
        //         }
        //     );
        //     self.call_event_handler(&e);
        // }

        // Live edits
        if self.handle_live_edit() {
            self.call_event_handler(&Event::LiveEdit);
            self.redraw_all();
        }

        // Platform operations
        self.handle_platform_ops();
    }

    fn handle_drawing(&mut self) {
        if self.new_next_frames.len() != 0 {
            self.call_next_frame_event(self.os.timers.time_now());
        }
        if self.need_redrawing() {
            self.call_draw_event();
            //direct_app.egl.make_current();
            self.opengl_compile_shaders();
        }
        // ok here we send out to all our childprocesses
        //profile_end("paint event handling", p);
        //let p = profile_start();
        self.handle_repaint();
    }

    fn handle_message(&mut self, msg: FromOhosMessage) {
        match msg {
            FromOhosMessage::Touch(point) => {
                let mut point = point;
                let time = point.time;
                let window = &mut self.windows[CxWindowPool::id_zero()];
                let dpi_factor = window.dpi_override.unwrap_or(self.os.dpi_factor);
                point.abs /= dpi_factor;
                let touches = vec![point];
                self.fingers.process_touch_update_start(time, &touches);
                let e = Event::TouchUpdate(TouchUpdateEvent {
                    time,
                    window_id: CxWindowPool::id_zero(),
                    touches,
                    modifiers: Default::default(),
                });
                self.call_event_handler(&e);
                let e = if let Event::TouchUpdate(e) = e {
                    e
                } else {
                    panic!()
                };
                self.fingers.process_touch_update_end(&e.touches);
            }
            _ => {}
        }
    }

    fn handle_surface_created(
        &mut self,
        from_ohos_rx: &mpsc::Receiver<FromOhosMessage>,
    ) -> *mut c_void {
        loop {
            match from_ohos_rx.recv() {
                Ok(FromOhosMessage::SurfaceCreated {
                    window,
                    width,
                    height,
                }) => {
                    loop {
                        match from_ohos_rx.recv() {
                            Ok(FromOhosMessage::Init {
                                option,
                                raw_env,
                                arkts_ref,
                                res_mgr,
                            }) => {
                                self.os.dpi_factor = option.display_density;
                                self.os.raw_env = raw_env;
                                self.os.arkts_ref = arkts_ref;
                                self.os.res_mgr = res_mgr;
                                self.os_type = OsType::OpenHarmony(OpenHarmonyParams {
                                    device_type: option.device_type,
                                    os_full_name: option.os_full_name,
                                    display_density: option.display_density,
                                });
                                break;
                            }
                            _ => {}
                        }
                    }
                    self.os.display_size = dvec2(width as f64, height as f64);
                    crate::log!(
                        "handle surface created, width={}, height={}, display_density={}",
                        width,
                        height,
                        self.os.dpi_factor
                    );
                    return window;
                }
                _ => {}
            }
        }
    }

    pub fn ohos_init<F>(exports: JsObject, env: Env, startup: F)
    where
        F: FnOnce() -> Box<Cx> + Send + 'static,
    {
        crate::log!("ohos init");
        std::panic::set_hook(Box::new(|info| {
            crate::log!("custom panic hook: {}", info);
        }));

        if let Ok(xcomponent) = exports.get_named_property::<JsObject>("__NATIVE_XCOMPONENT_OBJ__")
        {
            let (from_ohos_tx, from_ohos_rx) = mpsc::channel();
            let ohos_tx = from_ohos_tx.clone();
            init_globals(ohos_tx);

            register_xcomponent_callbacks(&env, &xcomponent);

            std::thread::spawn(move || {
                let mut cx = startup();
                let mut libegl = LibEgl::try_load().expect("can't load LibEGL");
                let window = cx.handle_surface_created(&from_ohos_rx);
                cx.ohos_load_dependencies();

                let (egl_context, egl_config, egl_display) = unsafe {
                    egl_sys::create_egl_context(&mut libegl).expect("Can't create EGL context")
                };
                unsafe {
                    gl_sys::load_with(|s| {
                        let s = CString::new(s).unwrap();
                        libegl.eglGetProcAddress.unwrap()(s.as_ptr())
                    })
                };

                let win_attr = vec![EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE];
                let surface = unsafe {
                    (libegl.eglCreateWindowSurface.unwrap())(
                        egl_display,
                        egl_config,
                        window as _,
                        win_attr.as_ptr() as _,
                    )
                };

                if surface.is_null() {
                    let err_code = unsafe { (libegl.eglGetError.unwrap())() };
                    crate::log!("eglCreateWindowSurface error code:{}", err_code);
                }
                assert!(!surface.is_null());

                crate::log!("eglCreateWindowSurface success");
                unsafe {
                    (libegl.eglSwapBuffers.unwrap())(egl_display, surface);
                }

                if unsafe {
                    (libegl.eglMakeCurrent.unwrap())(egl_display, surface, surface, egl_context)
                } == 0
                {
                    panic!();
                }

                cx.os.display = Some(CxOhosDisplay {
                    libegl,
                    egl_display,
                    egl_config,
                    egl_context,
                    surface,
                    window,
                });

                register_vsync_callback(from_ohos_tx);
                cx.main_loop(from_ohos_rx);
                //TODO, destroy surface
            });
        } else {
            crate::log!("Failed to get xcomponent in ohos_init");
        }
    }

    pub fn ohos_load_dependencies(&mut self) {
        let mut raw_mgr = RawFileMgr::new(self.os.raw_env, self.os.res_mgr);
        for (path, dep) in &mut self.dependencies {
            let mut buffer = Vec::<u8>::new();
            if let Ok(_) = raw_mgr.read_to_end(path, &mut buffer) {
                dep.data = Some(Ok(Rc::new(buffer)));
            } else {
                dep.data = Some(Err("read_to_end failed".to_string()));
            }
        }
    }

    pub fn draw_pass_to_fullscreen(&mut self, pass_id: PassId) {
        let draw_list_id = self.passes[pass_id].main_draw_list_id.unwrap();

        self.setup_render_pass(pass_id);

        // keep repainting in a loop
        //self.passes[pass_id].paint_dirty = false;

        unsafe {
            //direct_app.egl.make_current();
            gl_sys::Viewport(
                0,
                0,
                self.os.display_size.x as i32,
                self.os.display_size.y as i32,
            );
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

        unsafe { self.os.display.as_mut().unwrap().swap_buffers() };

        //unsafe {
        //direct_app.drm.swap_buffers_and_wait(&direct_app.egl);
        //}
    }

    pub(crate) fn handle_repaint(&mut self) {
        let mut passes_todo = Vec::new();
        self.compute_pass_repaint_order(&mut passes_todo);
        self.repaint_id += 1;
        for pass_id in &passes_todo {
            self.passes[*pass_id].set_time(self.os.timers.time_now() as f32);
            match self.passes[*pass_id].parent.clone() {
                CxPassParent::Window(_window_id) => {
                    self.draw_pass_to_fullscreen(*pass_id);
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

    fn handle_platform_ops(&mut self) -> EventFlow {
        while let Some(op) = self.platform_ops.pop() {
            //crate::log!("============ handle_platform_ops");
            match op {
                CxOsOp::CreateWindow(window_id) => {
                    let window = &mut self.windows[window_id];
                    let size = dvec2(
                        self.os.display_size.x / self.os.dpi_factor,
                        self.os.display_size.y / self.os.dpi_factor,
                    );
                    window.window_geom = WindowGeom {
                        dpi_factor: self.os.dpi_factor,
                        can_fullscreen: false,
                        xr_is_presenting: false,
                        is_fullscreen: true,
                        is_topmost: true,
                        position: dvec2(0.0, 0.0),
                        inner_size: size,
                        outer_size: size,
                    };
                    window.is_created = true;
                }
                CxOsOp::SetCursor(_cursor) => {
                    //xlib_app.set_mouse_cursor(cursor);
                }
                CxOsOp::StartTimer {
                    timer_id,
                    interval,
                    repeats,
                } => {
                    self.os.timers.start_timer(timer_id, interval, repeats);
                }
                CxOsOp::StopTimer(timer_id) => {
                    self.os.timers.stop_timer(timer_id);
                }
                CxOsOp::ShowTextIME(_area, _pos) => {
                    let mut show = std::ptr::null_mut();
                    let mut arkts = std::ptr::null_mut();
                    let recv = std::ptr::null_mut();
                    let mut result = std::ptr::null_mut();
                    unsafe {
                        assert!(napi_get_reference_value(self.os.raw_env, self.os.arkts_ref, & mut arkts))==0;
                        assert!(napi_get_named_property(self.os.raw_env, arkts, c"showInputText".as_ptr(), & mut show)==0);
                        assert!(napi_call_function(self.os.raw_env, recv, show, 0, std::ptr::null(), & mut result)==0);
                    }



                    //self.os.keyboard_trigger_position = area.get_clipped_rect(self).pos;
                    //unsafe {android_jni::to_java_show_keyboard(true);}
                }
                CxOsOp::HideTextIME => {
                    //self.os.keyboard_visible = false;
                    //unsafe {android_jni::to_java_show_keyboard(false);}
                }
                _ => (),
            }
        }
        EventFlow::Poll
    }
}

impl CxOsApi for Cx {
    fn init_cx_os(&mut self) {
        self.live_registry.borrow_mut().package_root = Some("makepad".to_string());
        self.live_expand();
        self.live_scan_dependencies();

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
    pub libegl: LibEgl,
    pub egl_display: egl_sys::EGLDisplay,
    pub egl_config: egl_sys::EGLConfig,
    pub egl_context: egl_sys::EGLContext,
    pub surface: egl_sys::EGLSurface,
    pub window: *mut c_void, //event_handler: Box<dyn EventHandler>,
}

pub struct CxOs {
    pub display_size: DVec2,
    pub dpi_factor: f64,
    pub media: CxOpenHarmonyMedia,
    pub quit: bool,
    pub timers: SelectTimers,
    pub raw_env: napi_ohos::sys::napi_env,
    pub arkts_ref: napi_ohos::sys::napi_ref,
    pub res_mgr: napi_ohos::sys::napi_value,
    pub(crate) start_time: Instant,
    pub(crate) display: Option<CxOhosDisplay>,
}

impl Default for CxOs {
    fn default() -> Self {
        Self {
            display_size: dvec2(1260 as f64, 2503 as f64),
            dpi_factor: 3.25,
            media: Default::default(),
            quit: false,
            timers: SelectTimers::new(),
            raw_env: std::ptr::null_mut(),
            arkts_ref: std::ptr::null_mut(),
            res_mgr: std::ptr::null_mut(),
            start_time: Instant::now(),
            display: None,
        }
    }
}

impl CxOhosDisplay {
    //unsafe fn destroy_surface(&mut self) {
    //    (self.libegl.eglMakeCurrent.unwrap())(
    //        self.egl_display,
    //        std::ptr::null_mut(),
    //        std::ptr::null_mut(),
    //        std::ptr::null_mut(),
    //    );
    //    (self.libegl.eglDestroySurface.unwrap())(self.egl_display, self.surface);
    //    self.surface = std::ptr::null_mut();
    //}

    //unsafe fn update_surface(&mut self, window: *mut c_void) {
    //    if !self.window.is_null() {
    //        //todo release window
    //    }
    //    self.window = window;
    //    if self.surface.is_null() == false {
    //        self.destroy_surface();
    //    }

    //    let win_attr = vec![EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE];
    //    self.surface = (self.libegl.eglCreateWindowSurface.unwrap())(
    //        self.egl_display,
    //        self.egl_config,
    //        self.window as _,
    //        win_attr.as_ptr() as _,
    //    );

    //    if self.surface.is_null() {
    //        let err_code = unsafe { (self.libegl.eglGetError.unwrap())() };
    //        crate::log!("eglCreateWindowSurface error code:{}", err_code);
    //    }

    //    assert!(!self.surface.is_null());

    //    self.make_current();
    //}

    unsafe fn swap_buffers(&mut self) {
        (self.libegl.eglSwapBuffers.unwrap())(self.egl_display, self.surface);
    }

    //unsafe fn make_current(&mut self) {
    //    if (self.libegl.eglMakeCurrent.unwrap())(
    //        self.egl_display,
    //        self.surface,
    //        self.surface,
    //        self.egl_context,
    //    ) == 0
    //    {
    //        panic!();
    //    }
    //}
}
