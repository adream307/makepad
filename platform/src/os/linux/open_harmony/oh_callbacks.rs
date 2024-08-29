use self::super::oh_sys::*;
use crate::area::Area;
use crate::event::{TouchPoint, TouchState};
use crate::makepad_math::*;
use napi_derive_ohos::napi;
use napi_ohos::{Env, JsObject, JsString, NapiRaw};
use ohos_sys::xcomponent::{
    OH_NativeXComponent, OH_NativeXComponent_Callback, OH_NativeXComponent_GetTouchEvent,
    OH_NativeXComponent_GetXComponentSize, OH_NativeXComponent_RegisterCallback,
    OH_NativeXComponent_TouchEvent, OH_NativeXComponent_TouchEventType,
};
use std::cell::{Cell, RefCell};
use std::mem::MaybeUninit;
use std::os::raw::c_void;
use std::sync::mpsc;

#[napi(object)]
#[derive(Clone, Debug)]
pub struct OpenHarmonyInitOptions {
    pub device_type: String,
    pub os_full_name: String,
    pub display_density: f64,
}

unsafe impl Send for FromOhosMessage {}

struct VSyncParams {
    pub vsync: *mut OH_NativeVSync,
    pub tx: mpsc::Sender<FromOhosMessage>,
}

thread_local! {
    static OHOS_MSG_TX: RefCell<Option<mpsc::Sender<FromOhosMessage>>> = RefCell::new(None);
}

pub fn send_from_ohos_message(message: FromOhosMessage) {
    OHOS_MSG_TX.with(|tx| {
        let mut tx = tx.borrow_mut();
        tx.as_mut().unwrap().send(message).unwrap();
    });
}

#[no_mangle]
extern "C" fn on_surface_created_cb(xcomponent: *mut OH_NativeXComponent, window: *mut c_void) {
    let mut width: u64 = 0;
    let mut height: u64 = 0;

    let ret = unsafe {
        OH_NativeXComponent_GetXComponentSize(xcomponent, window, &mut width, &mut height)
    };

    crate::log!(
        "OnSurfaceCreateCallBack,OH_NativeXComponent_GetXComponentSize={},width={},hight={}",
        ret,
        width,
        height
    );
    send_from_ohos_message(FromOhosMessage::SurfaceCreated {
        window,
        width: width as i32,
        height: height as i32,
    });
}

#[no_mangle]
extern "C" fn on_surface_changed_cb(xcomponent: *mut OH_NativeXComponent, window: *mut c_void) {
    let mut width: u64 = 0;
    let mut height: u64 = 0;

    let ret = unsafe {
        OH_NativeXComponent_GetXComponentSize(xcomponent, window, &mut width, &mut height)
    };

    crate::log!(
        "OnSurfaceChangeCallBack,OH_NativeXComponent_GetXComponentSize={},width={},hight={}",
        ret,
        width,
        height
    );
    send_from_ohos_message(FromOhosMessage::SurfaceChanged {
        window,
        width: width as i32,
        height: height as i32,
    });
}

#[no_mangle]
extern "C" fn on_surface_destroyed_cb(_component: *mut OH_NativeXComponent, _window: *mut c_void) {
    crate::log!("OnSurcefaceDestroyCallBack");
}

#[no_mangle]
extern "C" fn on_dispatch_touch_event_cb(component: *mut OH_NativeXComponent, window: *mut c_void) {
    let mut touch_event: MaybeUninit<OH_NativeXComponent_TouchEvent> = MaybeUninit::uninit();
    let res =
        unsafe { OH_NativeXComponent_GetTouchEvent(component, window, touch_event.as_mut_ptr()) };
    if res != 0 {
        crate::error!("OH_NativeXComponent_GetTouchEvent failed with {res}");
        return;
    }
    let touch_event = unsafe { touch_event.assume_init() };

    let touch_state = match touch_event.type_ {
        OH_NativeXComponent_TouchEventType::OH_NATIVEXCOMPONENT_DOWN => TouchState::Start,
        OH_NativeXComponent_TouchEventType::OH_NATIVEXCOMPONENT_UP => TouchState::Stop,
        OH_NativeXComponent_TouchEventType::OH_NATIVEXCOMPONENT_MOVE => TouchState::Move,
        OH_NativeXComponent_TouchEventType::OH_NATIVEXCOMPONENT_CANCEL => TouchState::Move,
        _ => {
            crate::error!(
                "Failed to dispatch call for touch Event {:?}",
                touch_event.type_
            );
            TouchState::Move
        }
    };
    let point = TouchPoint {
        state: touch_state,
        abs: dvec2(touch_event.x as f64, touch_event.y as f64),
        time: touch_event.timeStamp as f64 / 1000.0,
        uid: touch_event.id as u64,
        rotation_angle: 0.0,
        force: touch_event.force as f64,
        radius: dvec2(1.0, 1.0),
        handled: Cell::new(Area::Empty),
        sweep_lock: Cell::new(Area::Empty),
    };
    send_from_ohos_message(FromOhosMessage::Touch(point));
    //crate::log!("OnDispatchTouchEventCallBack");
}

#[no_mangle]
extern "C" fn on_vsync_cb(_timestamp: ::core::ffi::c_longlong, data: *mut c_void) {
    unsafe {
        let _ = (*(data as *mut VSyncParams))
            .tx
            .send(FromOhosMessage::VSync);
    }
    let res = unsafe {
        OH_NativeVSync_RequestFrame((*(data as *mut VSyncParams)).vsync, on_vsync_cb, data)
    };
    if res != 0 {
        crate::error!("Failed to register vsync callbacks");
    }
    //crate::log!("OnVSyncCallBack, timestamp = {}, register call back = {}",timestamp,res);
}

pub fn init_globals(from_ohos_tx: mpsc::Sender<FromOhosMessage>) {
    OHOS_MSG_TX.with(move |messages_tx| *messages_tx.borrow_mut() = Some(from_ohos_tx));
}

pub fn register_xcomponent_callbacks(env: &Env, xcomponent: &JsObject) {
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
}

pub fn register_vsync_callback(from_ohos_tx: mpsc::Sender<FromOhosMessage>) {
    //vsync call back
    let vsync = unsafe { OH_NativeVSync_Create(c"makepad".as_ptr(), 7) };
    let param = VSyncParams {
        vsync: vsync,
        tx: from_ohos_tx,
    };
    let data = Box::new(param);
    let res = unsafe {
        OH_NativeVSync_RequestFrame(vsync, on_vsync_cb, Box::into_raw(data) as *mut c_void)
    };
    if res != 0 {
        crate::error!("Failed to register vsync callbacks");
    } else {
        crate::log!("Registerd callbacks vsync successfully");
    }
}

#[allow(unused)]
pub fn debug_jsobject(obj: &JsObject, obj_name: &str) -> napi_ohos::Result<()> {
    let names = obj.get_property_names()?;
    crate::log!("Getting property names of object {obj_name}");
    let len = names.get_array_length()?;
    crate::log!("{obj_name} has {len} elements");
    for i in 0..len {
        let name: JsString = names.get_element(i)?;
        let name = name.into_utf8()?;
        crate::log!("{obj_name} property {i}: {}", name.as_str()?)
    }
    Ok(())
}

#[derive(Debug)]
pub enum FromOhosMessage {
    Init {
        option: OpenHarmonyInitOptions,
        raw_env: napi_ohos::sys::napi_env,
        res_mgr: napi_ohos::sys::napi_value
    },
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
    VSync,
    Touch(TouchPoint),
}
//TODO DIP
