//! Raw NDK bindings. No `jni`/`ndk` crates — those pull in std.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use core::ffi::{c_char, c_int, c_void};

pub type ANativeWindow = c_void;
pub type AInputQueue = c_void;
pub type AInputEvent = c_void;
pub type ALooper = c_void;

#[repr(C)]
pub struct ARect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct ANativeActivityCallbacks {
    pub onStart: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onResume: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onSaveInstanceState: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void, usize)>,
    pub onPause: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onStop: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onDestroy: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onWindowFocusChanged: Option<unsafe extern "C" fn(*mut ANativeActivity, c_int)>,
    pub onNativeWindowCreated: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut ANativeWindow)>,
    pub onNativeWindowResized: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut ANativeWindow)>,
    pub onNativeWindowRedrawNeeded:
        Option<unsafe extern "C" fn(*mut ANativeActivity, *mut ANativeWindow)>,
    pub onNativeWindowDestroyed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut ANativeWindow)>,
    pub onInputQueueCreated: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut AInputQueue)>,
    pub onInputQueueDestroyed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut AInputQueue)>,
    pub onContentRectChanged: Option<unsafe extern "C" fn(*mut ANativeActivity, *const ARect)>,
    pub onConfigurationChanged: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub onLowMemory: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
}

#[repr(C)]
pub struct ANativeActivity {
    pub callbacks: *mut ANativeActivityCallbacks,
    pub vm: *mut c_void,
    pub env: *mut c_void,
    pub clazz: *mut c_void,
    pub internalDataPath: *const c_char,
    pub externalDataPath: *const c_char,
    pub sdkVersion: i32,
    pub instance: *mut c_void,
    pub assetManager: *mut c_void,
    pub obbPath: *const c_char,
}

pub type ALooperCallback = Option<unsafe extern "C" fn(fd: i32, events: i32, data: *mut c_void) -> i32>;

pub const ALOOPER_PREPARE_ALLOW_NON_CALLBACKS: i32 = 1;
pub const ALOOPER_POLL_CALLBACK: i32 = -2;
pub const ALOOPER_EVENT_INPUT: i32 = 1;

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
pub struct Itimerspec {
    pub it_interval: Timespec,
    pub it_value: Timespec,
}
pub const AINPUT_EVENT_TYPE_MOTION: i32 = 2;
pub const AMOTION_EVENT_ACTION_DOWN: i32 = 0;
pub const AMOTION_EVENT_ACTION_UP: i32 = 1;
pub const AMOTION_EVENT_ACTION_MOVE: i32 = 2;
pub const AMOTION_EVENT_ACTION_CANCEL: i32 = 3;

extern "C" {
    pub fn ANativeWindow_toSurface(env: *mut c_void, window: *mut ANativeWindow) -> *mut c_void;
    pub fn ANativeWindow_getWidth(window: *mut ANativeWindow) -> i32;
    pub fn ANativeWindow_getHeight(window: *mut ANativeWindow) -> i32;

    pub fn AInputQueue_attachLooper(
        queue: *mut AInputQueue,
        looper: *mut ALooper,
        ident: i32,
        callback: ALooperCallback,
        data: *mut c_void,
    );
    pub fn AInputQueue_detachLooper(queue: *mut AInputQueue);
    pub fn AInputQueue_getEvent(queue: *mut AInputQueue, out_event: *mut *mut AInputEvent) -> i32;
    pub fn AInputQueue_preDispatchEvent(queue: *mut AInputQueue, event: *mut AInputEvent) -> i32;
    pub fn AInputQueue_finishEvent(queue: *mut AInputQueue, event: *mut AInputEvent, handled: i32);

    pub fn AInputEvent_getType(event: *const AInputEvent) -> i32;
    pub fn AMotionEvent_getAction(event: *const AInputEvent) -> i32;
    pub fn AMotionEvent_getX(event: *const AInputEvent, pointer_index: usize) -> f32;
    pub fn AMotionEvent_getY(event: *const AInputEvent, pointer_index: usize) -> f32;

    pub fn ALooper_forThread() -> *mut ALooper;
    pub fn ALooper_prepare(opts: i32) -> *mut ALooper;
    pub fn ALooper_addFd(
        looper: *mut ALooper,
        fd: i32,
        ident: i32,
        events: i32,
        callback: ALooperCallback,
        data: *mut c_void,
    ) -> i32;

    pub fn timerfd_create(clockid: i32, flags: i32) -> i32;
    pub fn timerfd_settime(
        fd: i32,
        flags: i32,
        new_value: *const Itimerspec,
        old_value: *mut Itimerspec,
    ) -> i32;
    pub fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
}
