#![no_std]

extern crate alloc;

mod android;
mod app;
mod jni;
mod store;

use alloc::string::String;
use core::cell::UnsafeCell;
use core::ffi::{c_char, c_void};
use core::panic::PanicInfo;
use core::ptr;

use android::*;
use jni::*;

use linked_list_allocator::LockedHeap;

const HEAP_SIZE: usize = 1 << 20;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

pub fn log(s: &str) {
    extern "C" {
        fn __android_log_write(prio: i32, tag: *const c_char, text: *const c_char) -> i32;
    }
    let mut buf = [0u8; 200];
    let mut n = 0;
    for b in s.as_bytes() {
        if n >= 199 {
            break;
        }
        buf[n] = *b;
        n += 1;
    }
    buf[n] = 0;
    unsafe {
        __android_log_write(4, b"TINY\0".as_ptr() as *const c_char, buf.as_ptr() as *const c_char);
    }
}

// ------------------------------------------------------------------ colours

pub const COL_BG: i32 = 0xFFF4F2EDu32 as i32;
pub const COL_SURFACE: i32 = 0xFFFFFFFFu32 as i32;
pub const COL_INK: i32 = 0xFF171614u32 as i32;
pub const COL_INK_2: i32 = 0xFF6B675Eu32 as i32;
pub const COL_INK_3: i32 = 0xFF9C978Cu32 as i32;
pub const COL_LINE: i32 = 0xFFE6E1D6u32 as i32;
pub const COL_ACCENT: i32 = 0xFFC0562Fu32 as i32;
pub const COL_ACCENT_SOFT: i32 = 0xFFF6E7DFu32 as i32;
pub const COL_DANGER: i32 = 0xFFA83A2Cu32 as i32;

// -------------------------------------------------------------------- gfx

pub struct Gfx {
    pub env: *mut c_void,
    canvas: JObject,
    pub d: f32,
    pub w: f32,
    pub h: f32,
    pub inset_top: f32,
    pub inset_bottom: f32,
    paint_shape: JObject,
    paint_text: JObject,
    paint_line: JObject,
    m_set_color: JMethod,
    m_set_text_size: JMethod,
    m_set_fake_bold: JMethod,
    m_measure_text: JMethod,
    m_draw_rect: JMethod,
    m_draw_round_rect: JMethod,
    m_draw_text: JMethod,
    m_draw_circle: JMethod,
    m_draw_line: JMethod,
    m_save: JMethod,
    m_restore: JMethod,
    m_clip_rect: JMethod,
}

impl Gfx {
    fn color(&self, paint: JObject, c: i32) {
        unsafe { call_void(self.env, paint, self.m_set_color, &[JValue::int(c)]) }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, c: i32) {
        let d = self.d;
        self.color(self.paint_shape, c);
        unsafe {
            call_void(
                self.env,
                self.canvas,
                self.m_draw_rect,
                &[
                    JValue::float(x * d),
                    JValue::float(y * d),
                    JValue::float((x + w) * d),
                    JValue::float((y + h) * d),
                    JValue::obj(self.paint_shape),
                ],
            )
        }
    }

    pub fn round_rect(&self, x: f32, y: f32, w: f32, h: f32, r: f32, c: i32) {
        let d = self.d;
        self.color(self.paint_shape, c);
        unsafe {
            call_void(
                self.env,
                self.canvas,
                self.m_draw_round_rect,
                &[
                    JValue::float(x * d),
                    JValue::float(y * d),
                    JValue::float((x + w) * d),
                    JValue::float((y + h) * d),
                    JValue::float(r * d),
                    JValue::float(r * d),
                    JValue::obj(self.paint_shape),
                ],
            )
        }
    }

    pub fn circle(&self, cx: f32, cy: f32, r: f32, c: i32) {
        let d = self.d;
        self.color(self.paint_shape, c);
        unsafe {
            call_void(
                self.env,
                self.canvas,
                self.m_draw_circle,
                &[
                    JValue::float(cx * d),
                    JValue::float(cy * d),
                    JValue::float(r * d),
                    JValue::obj(self.paint_shape),
                ],
            )
        }
    }

    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32, c: i32) {
        let d = self.d;
        self.color(self.paint_line, c);
        unsafe {
            call_void(
                self.env,
                self.paint_line,
                self.m_set_text_size,
                &[JValue::float(w * d)],
            );
            call_void(
                self.env,
                self.canvas,
                self.m_draw_line,
                &[
                    JValue::float(x1 * d),
                    JValue::float(y1 * d),
                    JValue::float(x2 * d),
                    JValue::float(y2 * d),
                    JValue::obj(self.paint_line),
                ],
            )
        }
    }

    fn with_jstr<R>(&self, s: &str, f: impl FnOnce(JString) -> R) -> R {
        let mut buf = [0u8; 1024];
        let mut n = 0;
        for ch in s.chars() {
            let mut tmp = [0u8; 4];
            let enc = ch.encode_utf8(&mut tmp);
            if n + enc.len() > 1022 {
                break;
            }
            buf[n..n + enc.len()].copy_from_slice(enc.as_bytes());
            n += enc.len();
        }
        buf[n] = 0;
        let js = unsafe { new_string(self.env, &buf[..n + 1]) };
        let r = f(js);
        unsafe { delete_local(self.env, js) };
        r
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, c: i32, bold: bool) {
        let d = self.d;
        unsafe {
            call_void(
                self.env,
                self.paint_text,
                self.m_set_text_size,
                &[JValue::float(size * d)],
            );
            call_void(
                self.env,
                self.paint_text,
                self.m_set_fake_bold,
                &[JValue::boolean(bold)],
            );
        }
        self.color(self.paint_text, c);
        self.with_jstr(s, |js| unsafe {
            call_void(
                self.env,
                self.canvas,
                self.m_draw_text,
                &[
                    JValue::obj(js),
                    JValue::float(x * d),
                    JValue::float(y * d),
                    JValue::obj(self.paint_text),
                ],
            )
        });
    }

    pub fn text_center(&self, s: &str, cx: f32, y: f32, size: f32, c: i32, bold: bool) {
        let w = self.text_width(s, size, bold);
        self.text(s, cx - w / 2.0, y, size, c, bold);
    }

    pub fn text_elide(&self, s: &str, x: f32, y: f32, size: f32, c: i32, bold: bool, max_w: f32) {
        if self.text_width(s, size, bold) <= max_w {
            self.text(s, x, y, size, c, bold);
            return;
        }
        let mut cut = String::new();
        for ch in s.chars() {
            cut.push(ch);
            let mut probe = cut.clone();
            probe.push('…');
            if self.text_width(&probe, size, bold) > max_w {
                cut.pop();
                break;
            }
        }
        cut.push('…');
        self.text(&cut, x, y, size, c, bold);
    }

    pub fn text_width(&self, s: &str, size: f32, bold: bool) -> f32 {
        unsafe {
            call_void(
                self.env,
                self.paint_text,
                self.m_set_text_size,
                &[JValue::float(size * self.d)],
            );
            call_void(
                self.env,
                self.paint_text,
                self.m_set_fake_bold,
                &[JValue::boolean(bold)],
            );
        }
        self.with_jstr(s, |js| unsafe {
            call_float(
                self.env,
                self.paint_text,
                self.m_measure_text,
                &[JValue::obj(js)],
            )
        }) / self.d
    }

    pub fn save(&self) {
        unsafe { call_int(self.env, self.canvas, self.m_save, &[]) };
    }

    pub fn restore(&self) {
        unsafe { call_void(self.env, self.canvas, self.m_restore, &[]) };
    }

    pub fn clip(&self, x: f32, y: f32, w: f32, h: f32) {
        let d = self.d;
        unsafe {
            call_boolean(
                self.env,
                self.canvas,
                self.m_clip_rect,
                &[
                    JValue::float(x * d),
                    JValue::float(y * d),
                    JValue::float((x + w) * d),
                    JValue::float((y + h) * d),
                ],
            );
        }
    }
}

// -------------------------------------------------------------------- state

struct State {
    vm: *mut c_void,
    env: *mut c_void,
    clazz: JObject,
    window: *mut ANativeWindow,
    surface: JObject,
    queue: *mut AInputQueue,
    width: i32,
    height: i32,
    density: f32,
    inset_top: f32,
    inset_bottom: f32,
    app: Option<app::App>,
    down_x: f32,
    down_y: f32,
    down_ok: bool,
    // cached method ids
    paint_shape: JObject,
    paint_text: JObject,
    paint_line: JObject,
    m_set_color: JMethod,
    m_set_text_size: JMethod,
    m_set_fake_bold: JMethod,
    m_measure_text: JMethod,
    m_draw_rect: JMethod,
    m_draw_round_rect: JMethod,
    m_draw_text: JMethod,
    m_draw_circle: JMethod,
    m_draw_line: JMethod,
    m_save: JMethod,
    m_restore: JMethod,
    m_clip_rect: JMethod,
    m_lock_canvas: JMethod,
    m_unlock_canvas: JMethod,
    // keyboard bridge
    edit_text: JObject,
    cls_string: JClass,
    m_value_of: JMethod,
    m_get_text: JMethod,
    m_set_text: JMethod,
    m_set_selection: JMethod,
    m_request_focus: JMethod,
    m_get_service: JMethod,
    m_show_ime: JMethod,
    m_hide_ime: JMethod,
    m_get_sel: JMethod,
    kb_visible: bool,
}

impl State {
    const fn new() -> Self {
        State {
            vm: ptr::null_mut(),
            env: ptr::null_mut(),
            clazz: ptr::null_mut(),
            window: ptr::null_mut(),
            surface: ptr::null_mut(),
            queue: ptr::null_mut(),
            width: 0,
            height: 0,
            density: 1.0,
            inset_top: 0.0,
            inset_bottom: 0.0,
            app: None,
            down_x: 0.0,
            down_y: 0.0,
            down_ok: false,
            paint_shape: ptr::null_mut(),
            paint_text: ptr::null_mut(),
            paint_line: ptr::null_mut(),
            m_set_color: ptr::null_mut(),
            m_set_text_size: ptr::null_mut(),
            m_set_fake_bold: ptr::null_mut(),
            m_measure_text: ptr::null_mut(),
            m_draw_rect: ptr::null_mut(),
            m_draw_round_rect: ptr::null_mut(),
            m_draw_text: ptr::null_mut(),
            m_draw_circle: ptr::null_mut(),
            m_draw_line: ptr::null_mut(),
            m_save: ptr::null_mut(),
            m_restore: ptr::null_mut(),
            m_clip_rect: ptr::null_mut(),
            m_lock_canvas: ptr::null_mut(),
            m_unlock_canvas: ptr::null_mut(),
            edit_text: ptr::null_mut(),
            cls_string: ptr::null_mut(),
            m_value_of: ptr::null_mut(),
            m_get_text: ptr::null_mut(),
            m_set_text: ptr::null_mut(),
            m_set_selection: ptr::null_mut(),
            m_request_focus: ptr::null_mut(),
            m_get_service: ptr::null_mut(),
            m_show_ime: ptr::null_mut(),
            m_hide_ime: ptr::null_mut(),
            m_get_sel: ptr::null_mut(),
            kb_visible: false,
        }
    }
}

struct Global(UnsafeCell<State>);
unsafe impl Sync for Global {}
static G: Global = Global(UnsafeCell::new(State::new()));

// --------------------------------------------------------------------- init

unsafe fn setup_jni(activity: *mut ANativeActivity) {
    let st = &mut *G.0.get();
    st.vm = (*activity).vm;
    st.clazz = (*activity).clazz;
    let env = vm_env(st.vm);
    st.env = env;
    if env.is_null() {
        return;
    }

    let cls_act = object_class(env, st.clazz);
    let m_res = get_method(env, cls_act, b"getResources\0", b"()Landroid/content/res/Resources;\0");
    let res = call_object(env, st.clazz, m_res, &[]);
    let cls_res = object_class(env, res);
    let m_dm = get_method(env, cls_res, b"getDisplayMetrics\0", b"()Landroid/util/DisplayMetrics;\0");
    let dm = call_object(env, res, m_dm, &[]);
    let cls_dm = object_class(env, dm);
    let f_density = get_field(env, cls_dm, b"density\0", b"F\0");
    st.density = get_float_field(env, dm, f_density);
    if st.density <= 0.0 {
        st.density = 1.0;
    }

    // safe-area insets
    let m_window = get_method(env, cls_act, b"getWindow\0", b"()Landroid/view/Window;\0");
    let win = call_object(env, st.clazz, m_window, &[]);
    if !win.is_null() {
        let cls_win = object_class(env, win);
        let m_decor = get_method(env, cls_win, b"getDecorView\0", b"()Landroid/view/View;\0");
        let decor = call_object(env, win, m_decor, &[]);
        if !decor.is_null() {
            let cls_view = object_class(env, decor);
            let m_insets = get_method(
                env,
                cls_view,
                b"getRootWindowInsets\0",
                b"()Landroid/view/WindowInsets;\0",
            );
            let insets = call_object(env, decor, m_insets, &[]);
            if !insets.is_null() {
                let cls_ins = object_class(env, insets);
                let m_top = get_method(env, cls_ins, b"getSystemWindowInsetTop\0", b"()I\0");
                let m_bot = get_method(env, cls_ins, b"getSystemWindowInsetBottom\0", b"()I\0");
                st.inset_top = call_int(env, insets, m_top, &[]) as f32 / st.density;
                st.inset_bottom = call_int(env, insets, m_bot, &[]) as f32 / st.density;
            }
        }
    }
    if st.inset_top < 20.0 {
        st.inset_top = 28.0;
    }
    if st.inset_bottom < 8.0 {
        st.inset_bottom = 16.0;
    }

    let cls_paint = find_class(env, b"android/graphics/Paint\0");
    let ctor = get_method(env, cls_paint, b"<init>\0", b"()V\0");
    let p1 = new_object(env, cls_paint, ctor, &[]);
    let p2 = new_object(env, cls_paint, ctor, &[]);
    let p3 = new_object(env, cls_paint, ctor, &[]);
    st.paint_shape = global_ref(env, p1);
    st.paint_text = global_ref(env, p2);
    st.paint_line = global_ref(env, p3);

    st.m_set_color = get_method(env, cls_paint, b"setColor\0", b"(I)V\0");
    st.m_set_text_size = get_method(env, cls_paint, b"setTextSize\0", b"(F)V\0");
    st.m_set_fake_bold = get_method(env, cls_paint, b"setFakeBoldText\0", b"(Z)V\0");
    st.m_measure_text = get_method(env, cls_paint, b"measureText\0", b"(Ljava/lang/String;)F\0");
    let m_aa = get_method(env, cls_paint, b"setAntiAlias\0", b"(Z)V\0");
    let m_style = get_method(env, cls_paint, b"setStyle\0", b"(Landroid/graphics/Paint$Style;)V\0");
    let m_stroke = get_method(env, cls_paint, b"setStrokeWidth\0", b"(F)V\0");

    call_void(env, st.paint_shape, m_aa, &[JValue::boolean(true)]);
    call_void(env, st.paint_text, m_aa, &[JValue::boolean(true)]);
    call_void(env, st.paint_line, m_aa, &[JValue::boolean(true)]);

    // stroke style for the line paint
    let cls_style = find_class(env, b"android/graphics/Paint$Style\0");
    let f_stroke = get_static_field(env, cls_style, b"STROKE\0", b"Landroid/graphics/Paint$Style;\0");
    let stroke = get_static_object(env, cls_style, f_stroke);
    call_void(env, st.paint_line, m_style, &[JValue::obj(stroke)]);

    let cls_canvas = find_class(env, b"android/graphics/Canvas\0");
    st.m_draw_rect = get_method(env, cls_canvas, b"drawRect\0", b"(FFFFLandroid/graphics/Paint;)V\0");
    st.m_draw_round_rect = get_method(
        env,
        cls_canvas,
        b"drawRoundRect\0",
        b"(FFFFFFLandroid/graphics/Paint;)V\0",
    );
    st.m_draw_text = get_method(
        env,
        cls_canvas,
        b"drawText\0",
        b"(Ljava/lang/String;FFLandroid/graphics/Paint;)V\0",
    );
    st.m_draw_circle = get_method(env, cls_canvas, b"drawCircle\0", b"(FFFLandroid/graphics/Paint;)V\0");
    st.m_draw_line = get_method(env, cls_canvas, b"drawLine\0", b"(FFFFLandroid/graphics/Paint;)V\0");
    st.m_save = get_method(env, cls_canvas, b"save\0", b"()I\0");
    st.m_restore = get_method(env, cls_canvas, b"restore\0", b"()V\0");
    st.m_clip_rect = get_method(env, cls_canvas, b"clipRect\0", b"(FFFF)Z\0");

    let cls_surface = find_class(env, b"android/view/Surface\0");
    st.m_lock_canvas = get_method(
        env,
        cls_surface,
        b"lockCanvas\0",
        b"(Landroid/graphics/Rect;)Landroid/graphics/Canvas;\0",
    );
    st.m_unlock_canvas = get_method(
        env,
        cls_surface,
        b"unlockCanvasAndPost\0",
        b"(Landroid/graphics/Canvas;)V\0",
    );

    // ---- keyboard bridge: a 1x1 invisible EditText owns the IME for us ----
    let cls_et = find_class(env, b"android/widget/EditText\0");
    let ctor_et = get_method(env, cls_et, b"<init>\0", b"(Landroid/content/Context;)V\0");
    let et = new_object(env, cls_et, ctor_et, &[JValue::obj(st.clazz)]);
    if !et.is_null() {
        let cls_view = find_class(env, b"android/view/View\0");
        let m_alpha = get_method(env, cls_view, b"setAlpha\0", b"(F)V\0");
        call_void(env, et, m_alpha, &[JValue::float(0.0)]);

        let cls_lp = find_class(env, b"android/view/ViewGroup$LayoutParams\0");
        let ctor_lp = get_method(env, cls_lp, b"<init>\0", b"(II)V\0");
        let lp = new_object(env, cls_lp, ctor_lp, &[JValue::int(1), JValue::int(1)]);
        let m_add = get_method(
            env,
            cls_act,
            b"addContentView\0",
            b"(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V\0",
        );
        call_void(env, st.clazz, m_add, &[JValue::obj(et), JValue::obj(lp)]);

        st.edit_text = global_ref(env, et);
        st.m_get_text = get_method(env, cls_et, b"getText\0", b"()Landroid/text/Editable;\0");
        st.m_set_text = get_method(env, cls_et, b"setText\0", b"(Ljava/lang/CharSequence;)V\0");
        st.m_set_selection = get_method(env, cls_et, b"setSelection\0", b"(I)V\0");
        st.m_request_focus = get_method(env, cls_et, b"requestFocus\0", b"()Z\0");
        st.m_get_sel = get_method(env, cls_et, b"getSelectionStart\0", b"()I\0");
    }
    // Global ref: this one is used from the timer callback, long after the JNI
    // frame that created it is gone.
    st.cls_string = global_ref(env, find_class(env, b"java/lang/String\0"));
    st.m_value_of = get_static_method(
        env,
        st.cls_string,
        b"valueOf\0",
        b"(Ljava/lang/Object;)Ljava/lang/String;\0",
    );
    st.m_get_service = get_method(
        env,
        cls_act,
        b"getSystemService\0",
        b"(Ljava/lang/String;)Ljava/lang/Object;\0",
    );
    let cls_imm = find_class(env, b"android/view/inputmethod/InputMethodManager\0");
    st.m_show_ime = get_method(env, cls_imm, b"showSoftInput\0", b"(Landroid/view/View;I)Z\0");
    st.m_hide_ime = get_method(
        env,
        cls_imm,
        b"hideSoftInputFromWindow\0",
        b"(Landroid/os/IBinder;I)Z\0",
    );

    // data dir
    let mut dir = String::from("/data/data/com.vbs.tinynotepad/files");
    let p = (*activity).internalDataPath;
    if !p.is_null() {
        let mut s = String::new();
        let mut i = 0;
        loop {
            let c = *p.add(i) as u8;
            if c == 0 {
                break;
            }
            s.push(c as char);
            i += 1;
            if i > 400 {
                break;
            }
        }
        if !s.is_empty() {
            dir = s;
        }
    }
    st.app = Some(app::App::new(dir));

    let _ = m_stroke;
}

// ------------------------------------------------------------------- drawing

unsafe fn redraw() {
    let st = &mut *G.0.get();
    if st.surface.is_null() || st.app.is_none() {
        return;
    }
    let env = st.env;
    let canvas = call_object(env, st.surface, st.m_lock_canvas, &[JValue::obj(ptr::null_mut())]);
    if canvas.is_null() || exception_check(env) {
        exception_clear(env);
        return;
    }

    let d = st.density;
    let w = st.width as f32 / d;
    let h = st.height as f32 / d;
    app::set_view(w, h, st.inset_top, st.inset_bottom);

    let g = Gfx {
        env,
        canvas,
        d,
        w: st.width as f32,
        h: st.height as f32,
        inset_top: st.inset_top,
        inset_bottom: st.inset_bottom,
        paint_shape: st.paint_shape,
        paint_text: st.paint_text,
        paint_line: st.paint_line,
        m_set_color: st.m_set_color,
        m_set_text_size: st.m_set_text_size,
        m_set_fake_bold: st.m_set_fake_bold,
        m_measure_text: st.m_measure_text,
        m_draw_rect: st.m_draw_rect,
        m_draw_round_rect: st.m_draw_round_rect,
        m_draw_text: st.m_draw_text,
        m_draw_circle: st.m_draw_circle,
        m_draw_line: st.m_draw_line,
        m_save: st.m_save,
        m_restore: st.m_restore,
        m_clip_rect: st.m_clip_rect,
    };

    g.rect(0.0, 0.0, w, h, COL_BG);
    if let Some(app) = st.app.as_mut() {
        app.draw(&g);
    }

    call_void(env, st.surface, st.m_unlock_canvas, &[JValue::obj(canvas)]);
    delete_local(env, canvas);
}

// ------------------------------------------------------------------ keyboard

unsafe fn input_method(st: &State) -> JObject {
    let env = st.env;
    let js = new_string(env, b"input_method\0");
    let imm = call_object(env, st.clazz, st.m_get_service, &[JValue::obj(js)]);
    delete_local(env, js);
    imm
}

unsafe fn show_keyboard(st: &mut State) {
    if st.edit_text.is_null() {
        return;
    }
    let env = st.env;
    call_boolean(env, st.edit_text, st.m_request_focus, &[]);
    let imm = input_method(st);
    if !imm.is_null() {
        call_boolean(env, imm, st.m_show_ime, &[JValue::obj(st.edit_text), JValue::int(1)]);
        delete_local(env, imm);
    }
}

unsafe fn hide_keyboard(st: &mut State) {
    if st.edit_text.is_null() {
        return;
    }
    let env = st.env;
    let imm = input_method(st);
    if imm.is_null() {
        return;
    }
    let cls_view = find_class(env, b"android/view/View\0");
    let m_token = get_method(env, cls_view, b"getWindowToken\0", b"()Landroid/os/IBinder;\0");
    let token = call_object(env, st.edit_text, m_token, &[]);
    if !token.is_null() {
        call_boolean(env, imm, st.m_hide_ime, &[JValue::obj(token), JValue::int(0)]);
        delete_local(env, token);
    }
    delete_local(env, imm);
}

unsafe fn set_edit_text(st: &State, s: &str) {
    if st.edit_text.is_null() {
        return;
    }
    let env = st.env;
    let mut buf = [0u8; 4096];
    let mut n = 0;
    for ch in s.chars() {
        let mut tmp = [0u8; 4];
        let enc = ch.encode_utf8(&mut tmp);
        if n + enc.len() > 4094 {
            break;
        }
        buf[n..n + enc.len()].copy_from_slice(enc.as_bytes());
        n += enc.len();
    }
    buf[n] = 0;
    let js = new_string(env, &buf[..n + 1]);
    call_void(env, st.edit_text, st.m_set_text, &[JValue::obj(js)]);
    delete_local(env, js);
    call_void(env, st.edit_text, st.m_set_selection, &[JValue::int(n as i32)]);
}

/// Pull whatever the IME has produced into the model. Returns true if it changed.
unsafe fn poll_edit(st: &mut State) -> bool {
    if st.edit_text.is_null() {
        return false;
    }
    let env = st.env;
    let ed = call_object(env, st.edit_text, st.m_get_text, &[]);
    if ed.is_null() {
        return false;
    }
    let js = call_static_object(env, st.cls_string, st.m_value_of, &[JValue::obj(ed)]);
    delete_local(env, ed);
    let mut buf = [0u8; 4096];
    let n = read_string(env, js, &mut buf);
    delete_local(env, js);
    let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
    let sel = if st.m_get_sel.is_null() {
        -1
    } else {
        call_int(env, st.edit_text, st.m_get_sel, &[])
    };

    let Some(app) = st.app.as_mut() else {
        return false;
    };
    if sel >= 0 {
        app.cursor = sel as usize;
    }
    if app.edit_field == 1 && app.title.as_str() != s {
        app.title = String::from(s);
        app.mark_edit();
        return true;
    }
    if app.edit_field == 2 && app.body.as_str() != s {
        app.body = String::from(s);
        app.mark_edit();
        return true;
    }
    if app.edit_field == 3 && app.query.as_str() != s {
        app.query = String::from(s);
        return true;
    }
    false
}

unsafe extern "C" fn on_timer(fd: i32, _events: i32, _data: *mut c_void) -> i32 {
    let mut buf = [0u8; 8];
    read(fd, buf.as_mut_ptr() as *mut c_void, 8);
    let mut needs = false;
    {
        let st = &mut *G.0.get();
        if poll_edit(st) {
            needs = true;
        }
        let phase = (app::now_ms() / 500) % 2 == 0;
        if let Some(a) = st.app.as_mut() {
            if a.edit_field != 0 && a.caret_on != phase {
                a.caret_on = phase;
                needs = true;
            }
            if a.autosave_if_due() {
                needs = true;
            }
        }
    }
    if needs {
        redraw();
    }
    1
}

// ----------------------------------------------------------------- callbacks

unsafe extern "C" fn on_window_created(activity: *mut ANativeActivity, window: *mut ANativeWindow) {
    let st = &mut *G.0.get();
    st.env = vm_env(st.vm);
    st.window = window;
    st.width = ANativeWindow_getWidth(window);
    st.height = ANativeWindow_getHeight(window);
    let surface = ANativeWindow_toSurface(st.env, window);
    if !surface.is_null() {
        st.surface = global_ref(st.env, surface);
        delete_local(st.env, surface);
    }
    let _ = activity;
    redraw();
}

unsafe extern "C" fn on_window_redraw(_a: *mut ANativeActivity, _w: *mut ANativeWindow) {
    redraw();
}

unsafe extern "C" fn on_window_destroyed(_a: *mut ANativeActivity, _w: *mut ANativeWindow) {
    let st = &mut *G.0.get();
    st.surface = ptr::null_mut();
    st.window = ptr::null_mut();
}

unsafe extern "C" fn on_input_created(_a: *mut ANativeActivity, queue: *mut AInputQueue) {
    let st = &mut *G.0.get();
    st.queue = queue;
    let mut looper = ALooper_forThread();
    log(if looper.is_null() { "input: no looper" } else { "input: looper ok" });
    if looper.is_null() {
        looper = ALooper_prepare(ALOOPER_PREPARE_ALLOW_NON_CALLBACKS);
        log(if looper.is_null() { "input: prepare failed" } else { "input: prepared" });
    }
    AInputQueue_attachLooper(queue, looper, ALOOPER_POLL_CALLBACK, Some(on_input), ptr::null_mut());
    log("input: attached");

    // 80ms tick so IME edits show up without a Java callback
    let tfd = timerfd_create(1, 0);
    if tfd >= 0 {
        let spec = Itimerspec {
            it_interval: Timespec { tv_sec: 0, tv_nsec: 80_000_000 },
            it_value: Timespec { tv_sec: 0, tv_nsec: 80_000_000 },
        };
        timerfd_settime(tfd, 0, &spec, ptr::null_mut());
        ALooper_addFd(looper, tfd, 0, ALOOPER_EVENT_INPUT, Some(on_timer), ptr::null_mut());
    }
}

unsafe extern "C" fn on_input_destroyed(_a: *mut ANativeActivity, queue: *mut AInputQueue) {
    AInputQueue_detachLooper(queue);
    let st = &mut *G.0.get();
    st.queue = ptr::null_mut();
}

unsafe extern "C" fn on_input(_fd: i32, _events: i32, _data: *mut c_void) -> i32 {
    let mut dirty = false;
    let mut want_show = false;
    let mut want_hide = false;
    let mut sync_text = false;
    {
        let st = &mut *G.0.get();
        if st.queue.is_null() {
            return 1;
        }
        loop {
            let mut ev: *mut AInputEvent = ptr::null_mut();
            if AInputQueue_getEvent(st.queue, &mut ev) < 0 {
                break;
            }
            if AInputQueue_preDispatchEvent(st.queue, ev) == 0 {
                dirty |= handle_event(st, ev);
                AInputQueue_finishEvent(st.queue, ev, 1);
            }
        }
        if let Some(app) = st.app.as_mut() {
            if app.field_dirty {
                app.field_dirty = false;
                sync_text = true;
            }
            let should_show = app.edit_field != 0;
            if should_show && !st.kb_visible {
                st.kb_visible = true;
                want_show = true;
            } else if !should_show && st.kb_visible {
                st.kb_visible = false;
                want_hide = true;
            }
        }
        if sync_text {
            let text = match st.app.as_ref() {
                Some(a) if a.edit_field == 1 => a.title.clone(),
                Some(a) if a.edit_field == 3 => a.query.clone(),
                Some(a) => a.body.clone(),
                None => String::new(),
            };
            set_edit_text(st, &text);
        }
        if want_show {
            show_keyboard(st);
        }
        if want_hide {
            hide_keyboard(st);
        }
    }
    if dirty {
        redraw();
    }
    1
}

unsafe fn handle_event(st: &mut State, ev: *mut AInputEvent) -> bool {
    if AInputEvent_getType(ev) != AINPUT_EVENT_TYPE_MOTION {
        log("not motion");
        return false;
    }
    let action = AMotionEvent_getAction(ev) & 0xff;
    let x = AMotionEvent_getX(ev, 0) / st.density;
    let y = AMotionEvent_getY(ev, 0) / st.density;
    {
        let mut s = String::new();
        s.push_str("act=");
        app::push_num(&mut s, action as u64);
        s.push_str(" x=");
        app::push_num(&mut s, x as u64);
        s.push_str(" y=");
        app::push_num(&mut s, y as u64);
        log(&s);
    }

    match action {
        AMOTION_EVENT_ACTION_DOWN => {
            st.down_x = x;
            st.down_y = y;
            st.down_ok = true;
            false
        }
        AMOTION_EVENT_ACTION_MOVE => {
            if st.down_ok {
                let dy = y - st.down_y;
                if dy.abs() > 6.0 {
                    if let Some(app) = st.app.as_mut() {
                        app.scroll_by(dy);
                    }
                    st.down_y = y;
                    return true;
                }
            }
            false
        }
        AMOTION_EVENT_ACTION_UP => {
            if st.down_ok && (x - st.down_x).abs() < 12.0 && (y - st.down_y).abs() < 12.0 {
                st.down_ok = false;
                if let Some(app) = st.app.as_mut() {
                    return app.tap(x, y);
                }
            }
            st.down_ok = false;
            false
        }
        AMOTION_EVENT_ACTION_CANCEL => {
            st.down_ok = false;
            false
        }
        _ => false,
    }
}

#[inline]
unsafe fn AMontion_get_y(st: &State, ev: *mut AInputEvent) -> f32 {
    AMotionEvent_getY(ev, 0) / st.density
}

unsafe extern "C" fn on_destroy(_a: *mut ANativeActivity) {}

#[no_mangle]
pub extern "C" fn ANativeActivity_onCreate(
    activity: *mut ANativeActivity,
    _saved_state: *mut c_void,
    _saved_size: usize,
) {
    unsafe {
        ALLOCATOR
            .lock()
            .init(core::ptr::addr_of_mut!(HEAP) as *mut u8, HEAP_SIZE);
        setup_jni(activity);
        let cb = (*activity).callbacks;
        (*cb).onNativeWindowCreated = Some(on_window_created);
        (*cb).onNativeWindowRedrawNeeded = Some(on_window_redraw);
        (*cb).onNativeWindowDestroyed = Some(on_window_destroyed);
        (*cb).onInputQueueCreated = Some(on_input_created);
        (*cb).onInputQueueDestroyed = Some(on_input_destroyed);
        (*cb).onDestroy = Some(on_destroy);
    }
}

#[allow(dead_code)]
fn _unused(_: *const c_char) {}
