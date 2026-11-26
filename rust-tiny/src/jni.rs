//! Minimal JNI bindings. Function-table indices were derived from the real
//! `JNINativeInterface_` layout (jni-sys), not from memory — a wrong index is a
//! silent crash.
#![allow(dead_code)]

use core::ffi::{c_char, c_void};

pub type JObject = *mut c_void;
pub type JClass = *mut c_void;
pub type JMethod = *mut c_void;
pub type JString = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub union JValue {
    pub z: u8,
    pub b: i8,
    pub c: u16,
    pub s: i16,
    pub i: i32,
    pub j: i64,
    pub f: f32,
    pub d: f64,
    pub l: JObject,
}

impl JValue {
    pub fn int(v: i32) -> Self {
        JValue { i: v }
    }
    pub fn float(v: f32) -> Self {
        JValue { f: v }
    }
    pub fn obj(v: JObject) -> Self {
        JValue { l: v }
    }
    pub fn boolean(v: bool) -> Self {
        JValue { z: v as u8 }
    }
}

const IDX_FIND_CLASS: usize = 6;
const IDX_EXCEPTION_CLEAR: usize = 17;
const IDX_NEW_GLOBAL_REF: usize = 21;
const IDX_DELETE_LOCAL_REF: usize = 23;
const IDX_NEW_OBJECT_A: usize = 30;
const IDX_GET_METHOD_ID: usize = 33;
const IDX_CALL_OBJECT_A: usize = 36;
const IDX_CALL_BOOLEAN_A: usize = 39;
const IDX_CALL_INT_A: usize = 51;
const IDX_CALL_FLOAT_A: usize = 57;
const IDX_CALL_VOID_A: usize = 63;
const IDX_GET_STATIC_METHOD_ID: usize = 113;
const IDX_CALL_STATIC_OBJECT_A: usize = 116;
const IDX_NEW_STRING_UTF: usize = 167;
const IDX_GET_STRING_UTF_CHARS: usize = 169;
const IDX_RELEASE_STRING_UTF_CHARS: usize = 170;
const IDX_EXCEPTION_CHECK: usize = 228;
const IDX_GET_FIELD_ID: usize = 94;
const IDX_GET_FLOAT_FIELD: usize = 102;
const IDX_GET_OBJECT_CLASS: usize = 31;
const IDX_GET_STATIC_FIELD_ID: usize = 144;
const IDX_GET_STATIC_OBJECT_FIELD: usize = 145;

pub unsafe fn get_static_field(env: *mut c_void, class: JClass, name: &[u8], sig: &[u8]) -> JMethod {
    type F = unsafe extern "system" fn(*mut c_void, JClass, *const c_char, *const c_char) -> JMethod;
    let f: F = core::mem::transmute(slot(env, IDX_GET_STATIC_FIELD_ID));
    f(
        env,
        class,
        name.as_ptr() as *const c_char,
        sig.as_ptr() as *const c_char,
    )
}

pub unsafe fn get_static_object(env: *mut c_void, class: JClass, field: JMethod) -> JObject {
    type F = unsafe extern "system" fn(*mut c_void, JClass, JMethod) -> JObject;
    let f: F = core::mem::transmute(slot(env, IDX_GET_STATIC_OBJECT_FIELD));
    f(env, class, field)
}

pub unsafe fn object_class(env: *mut c_void, obj: JObject) -> JClass {
    type F = unsafe extern "system" fn(*mut c_void, JObject) -> JClass;
    let f: F = core::mem::transmute(slot(env, IDX_GET_OBJECT_CLASS));
    f(env, obj)
}

pub unsafe fn get_field(env: *mut c_void, class: JClass, name: &[u8], sig: &[u8]) -> JMethod {
    type F = unsafe extern "system" fn(*mut c_void, JClass, *const c_char, *const c_char) -> JMethod;
    let f: F = core::mem::transmute(slot(env, IDX_GET_FIELD_ID));
    f(
        env,
        class,
        name.as_ptr() as *const c_char,
        sig.as_ptr() as *const c_char,
    )
}

pub unsafe fn get_float_field(env: *mut c_void, obj: JObject, field: JMethod) -> f32 {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod) -> f32;
    let f: F = core::mem::transmute(slot(env, IDX_GET_FLOAT_FIELD));
    f(env, obj, field)
}

/// JavaVM function table: 4 = AttachCurrentThread, 6 = GetEnv.
pub unsafe fn vm_env(vm: *mut c_void) -> *mut c_void {
    let table = *(vm as *mut *const *const c_void);
    type GetEnv = unsafe extern "system" fn(*mut c_void, *mut *mut c_void, i32) -> i32;
    type Attach = unsafe extern "system" fn(*mut c_void, *mut *mut c_void, *mut c_void) -> i32;
    let mut env: *mut c_void = core::ptr::null_mut();
    let get: GetEnv = core::mem::transmute(*table.add(6));
    if get(vm, &mut env, 0x0001_0006) != 0 {
        let attach: Attach = core::mem::transmute(*table.add(4));
        attach(vm, &mut env, core::ptr::null_mut());
    }
    env
}

#[inline(always)]
unsafe fn slot(env: *mut c_void, idx: usize) -> *const c_void {
    let table = *(env as *mut *const *const c_void);
    *table.add(idx)
}

pub unsafe fn find_class(env: *mut c_void, name: &[u8]) -> JClass {
    type F = unsafe extern "system" fn(*mut c_void, *const c_char) -> JClass;
    let f: F = core::mem::transmute(slot(env, IDX_FIND_CLASS));
    f(env, name.as_ptr() as *const c_char)
}

pub unsafe fn get_method(env: *mut c_void, class: JClass, name: &[u8], sig: &[u8]) -> JMethod {
    type F = unsafe extern "system" fn(*mut c_void, JClass, *const c_char, *const c_char) -> JMethod;
    let f: F = core::mem::transmute(slot(env, IDX_GET_METHOD_ID));
    f(
        env,
        class,
        name.as_ptr() as *const c_char,
        sig.as_ptr() as *const c_char,
    )
}

pub unsafe fn get_static_method(
    env: *mut c_void,
    class: JClass,
    name: &[u8],
    sig: &[u8],
) -> JMethod {
    type F = unsafe extern "system" fn(*mut c_void, JClass, *const c_char, *const c_char) -> JMethod;
    let f: F = core::mem::transmute(slot(env, IDX_GET_STATIC_METHOD_ID));
    f(
        env,
        class,
        name.as_ptr() as *const c_char,
        sig.as_ptr() as *const c_char,
    )
}

pub unsafe fn new_object(env: *mut c_void, class: JClass, method: JMethod, args: &[JValue]) -> JObject {
    type F = unsafe extern "system" fn(*mut c_void, JClass, JMethod, *const JValue) -> JObject;
    let f: F = core::mem::transmute(slot(env, IDX_NEW_OBJECT_A));
    f(env, class, method, args.as_ptr())
}

pub unsafe fn call_object(env: *mut c_void, obj: JObject, method: JMethod, args: &[JValue]) -> JObject {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod, *const JValue) -> JObject;
    let f: F = core::mem::transmute(slot(env, IDX_CALL_OBJECT_A));
    f(env, obj, method, args.as_ptr())
}

pub unsafe fn call_static_object(
    env: *mut c_void,
    class: JClass,
    method: JMethod,
    args: &[JValue],
) -> JObject {
    type F = unsafe extern "system" fn(*mut c_void, JClass, JMethod, *const JValue) -> JObject;
    let f: F = core::mem::transmute(slot(env, IDX_CALL_STATIC_OBJECT_A));
    f(env, class, method, args.as_ptr())
}

pub unsafe fn call_void(env: *mut c_void, obj: JObject, method: JMethod, args: &[JValue]) {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod, *const JValue);
    let f: F = core::mem::transmute(slot(env, IDX_CALL_VOID_A));
    f(env, obj, method, args.as_ptr())
}

pub unsafe fn call_int(env: *mut c_void, obj: JObject, method: JMethod, args: &[JValue]) -> i32 {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod, *const JValue) -> i32;
    let f: F = core::mem::transmute(slot(env, IDX_CALL_INT_A));
    f(env, obj, method, args.as_ptr())
}

pub unsafe fn call_float(env: *mut c_void, obj: JObject, method: JMethod, args: &[JValue]) -> f32 {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod, *const JValue) -> f32;
    let f: F = core::mem::transmute(slot(env, IDX_CALL_FLOAT_A));
    f(env, obj, method, args.as_ptr())
}

pub unsafe fn call_boolean(env: *mut c_void, obj: JObject, method: JMethod, args: &[JValue]) -> bool {
    type F = unsafe extern "system" fn(*mut c_void, JObject, JMethod, *const JValue) -> u8;
    let f: F = core::mem::transmute(slot(env, IDX_CALL_BOOLEAN_A));
    f(env, obj, method, args.as_ptr()) != 0
}

pub unsafe fn new_string(env: *mut c_void, s: &[u8]) -> JString {
    type F = unsafe extern "system" fn(*mut c_void, *const c_char) -> JString;
    let f: F = core::mem::transmute(slot(env, IDX_NEW_STRING_UTF));
    f(env, s.as_ptr() as *const c_char)
}

pub unsafe fn global_ref(env: *mut c_void, obj: JObject) -> JObject {
    type F = unsafe extern "system" fn(*mut c_void, JObject) -> JObject;
    let f: F = core::mem::transmute(slot(env, IDX_NEW_GLOBAL_REF));
    f(env, obj)
}

pub unsafe fn delete_local(env: *mut c_void, obj: JObject) {
    type F = unsafe extern "system" fn(*mut c_void, JObject);
    let f: F = core::mem::transmute(slot(env, IDX_DELETE_LOCAL_REF));
    f(env, obj)
}

pub unsafe fn exception_check(env: *mut c_void) -> bool {
    type F = unsafe extern "system" fn(*mut c_void) -> u8;
    let f: F = core::mem::transmute(slot(env, IDX_EXCEPTION_CHECK));
    f(env) != 0
}

pub unsafe fn exception_clear(env: *mut c_void) {
    type F = unsafe extern "system" fn(*mut c_void);
    let f: F = core::mem::transmute(slot(env, IDX_EXCEPTION_CLEAR));
    f(env);
}

/// Reads a Java string into `buf` as UTF-8-ish bytes. Returns the length.
pub unsafe fn read_string(env: *mut c_void, s: JString, buf: &mut [u8]) -> usize {
    if s.is_null() {
        return 0;
    }
    type GetChars = unsafe extern "system" fn(*mut c_void, JString, *mut u8) -> *const c_char;
    type Release = unsafe extern "system" fn(*mut c_void, JString, *const c_char);
    let get: GetChars = core::mem::transmute(slot(env, IDX_GET_STRING_UTF_CHARS));
    let rel: Release = core::mem::transmute(slot(env, IDX_RELEASE_STRING_UTF_CHARS));

    let ptr = get(env, s, core::ptr::null_mut());
    if ptr.is_null() {
        return 0;
    }
    let mut len = 0;
    while len < buf.len() - 1 && *ptr.add(len) != 0 {
        buf[len] = *ptr.add(len) as u8;
        len += 1;
    }
    rel(env, s, ptr);
    len
}
