use {
    self::super::super::{gl_sys, select_timer::SelectTimers},
    self::super::{oh_callbacks::*, oh_media::CxOpenHarmonyMedia},
    crate::{
        cx::{Cx, OpenHarmonyParams, OsType},
        cx_api::{CxOsApi, CxOsOp, OpenUrlInPlace},
        egl_sys::{self, LibEgl, EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE},
        event::{Event, TouchUpdateEvent, WindowGeom},
        gpu_info::GpuPerformance,
        makepad_math::*,
        os::cx_native::EventFlow,
        //window::CxWindowPool,
        pass::CxPassParent,
        pass::{PassClearColor, PassClearDepth, PassId},
        thread::SignalToUI,
        window::CxWindowPool,
    },
    napi_derive_ohos::napi,
    napi_ohos::{Env, JsObject},
    std::{ffi::CString, os::raw::c_void, sync::mpsc, time::Instant},
};

#[napi]
pub fn init_makepad(init_opts: OpenHarmonyInitOptions) -> napi_ohos::Result<()> {
    crate::log!("call initMakePad from XComponent.onLoad, display_density = {}",init_opts.display_density);
    send_from_ohos_message(FromOhosMessage::Init(init_opts));
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
            register_vsync_callback(from_ohos_tx);

            std::thread::spawn(move || {
                let mut cx = startup();
                let mut libegl = LibEgl::try_load().expect("can't load LibEGL");
                let window = loop {
                    match from_ohos_rx.try_recv() {
                        Ok(FromOhosMessage::Init(params)) => {
                            cx.os.dpi_factor = params.display_density;
                            cx.os_type = OsType::OpenHarmony(OpenHarmonyParams {
                                device_type: params.device_type,
                                os_full_name: params.os_full_name,
                                display_density: params.display_density,
                            });
                        }
                        Ok(FromOhosMessage::SurfaceCreated {
                            window,
                            width,
                            height,
                        }) => {
                            cx.os.display_size = dvec2(width as f64, height as f64);
                            cx.os.dpi_factor = 3.25; //TODO, get from screen api
                            break window;
                        }
                        _ => {}
                    }
                };

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

                cx.main_loop(from_ohos_rx);
                //TODO, destroy surface
            });
        } else {
            crate::log!("Failed to get xcomponent in ohos_init");
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
                _ => (),
            }
        }
        EventFlow::Poll
    }
}

impl CxOsApi for Cx {
    fn init_cx_os(&mut self) {
        self.live_registry.borrow_mut().package_root = Some("/system/fonts".to_string());
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
            start_time: Instant::now(),
            display: None,
        }
    }
}

impl CxOhosDisplay {
    unsafe fn destroy_surface(&mut self) {
        (self.libegl.eglMakeCurrent.unwrap())(
            self.egl_display,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        (self.libegl.eglDestroySurface.unwrap())(self.egl_display, self.surface);
        self.surface = std::ptr::null_mut();
    }

    unsafe fn update_surface(&mut self, window: *mut c_void) {
        if !self.window.is_null() {
            //todo release window
        }
        self.window = window;
        if self.surface.is_null() == false {
            self.destroy_surface();
        }

        let win_attr = vec![EGL_GL_COLORSPACE_KHR, EGL_GL_COLORSPACE_SRGB_KHR, EGL_NONE];
        self.surface = (self.libegl.eglCreateWindowSurface.unwrap())(
            self.egl_display,
            self.egl_config,
            self.window as _,
            win_attr.as_ptr() as _,
        );

        if self.surface.is_null() {
            let err_code = unsafe { (self.libegl.eglGetError.unwrap())() };
            crate::log!("eglCreateWindowSurface error code:{}", err_code);
        }

        assert!(!self.surface.is_null());

        self.make_current();
    }

    unsafe fn swap_buffers(&mut self) {
        (self.libegl.eglSwapBuffers.unwrap())(self.egl_display, self.surface);
    }

    unsafe fn make_current(&mut self) {
        if (self.libegl.eglMakeCurrent.unwrap())(
            self.egl_display,
            self.surface,
            self.surface,
            self.egl_context,
        ) == 0
        {
            panic!();
        }
    }
}
