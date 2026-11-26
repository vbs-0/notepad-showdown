//! App state, layout and hit-testing. All geometry is in dp; the Gfx layer
//! multiplies by density.

use alloc::string::String;
use alloc::vec::Vec;

use crate::store::Note;
use crate::{
    Gfx, COL_ACCENT, COL_ACCENT_SOFT, COL_DANGER, COL_INK, COL_INK_2, COL_INK_3, COL_LINE,
    COL_SURFACE,
};

pub const LIST_Y: f32 = 166.0;
pub const ITEM_H: f32 = 108.0;
pub const CARD_H: f32 = 96.0;

pub struct App {
    pub notes: Vec<Note>,
    pub screen: u8,
    pub query: String,
    pub title: String,
    pub body: String,
    pub current: Option<u32>,
    pub scroll: f32,
    pub content_h: f32,
    pub data_dir: String,
    pub editing: bool,
    pub status: String,
    pub edit_field: u8,
    pub field_dirty: bool,
    pub dirty: bool,
    pub last_edit_ms: u64,
    pub cursor: usize,
    pub caret_on: bool,
}

impl App {
    pub fn new(data_dir: String) -> Self {
        let notes = crate::store::load(&data_dir);
        App {
            notes,
            screen: 0,
            query: String::new(),
            title: String::new(),
            body: String::new(),
            current: None,
            scroll: 0.0,
            content_h: 0.0,
            data_dir,
            editing: false,
            status: String::new(),
            edit_field: 0,
            field_dirty: false,
            dirty: false,
            last_edit_ms: 0,
            cursor: 0,
            caret_on: true,
        }
    }

    pub fn save(&self) {
        crate::store::save(&self.data_dir, &self.notes);
    }

    fn next_id(&self) -> u32 {
        self.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    fn sort(&mut self) {
        // Insertion sort: lists are tiny and this avoids pulling in core's
        // quicksort/drift machinery (~2.6 KB of code for nothing).
        for i in 1..self.notes.len() {
            let mut j = i;
            while j > 0 && self.notes[j - 1].updated < self.notes[j].updated {
                self.notes.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    pub fn new_note(&mut self) {
        let id = self.next_id();
        self.notes.push(Note {
            id,
            updated: now(),
            title: String::new(),
            body: String::new(),
        });
        self.sort();
        self.current = Some(id);
        self.title.clear();
        self.body.clear();
        self.status = String::from("0 words · Saved");
        self.screen = 1;
        self.editing = true;
        self.edit_field = 1;
        self.field_dirty = true;
        self.save();
    }

    pub fn open(&mut self, id: u32) {
        if let Some(n) = self.notes.iter().find(|n| n.id == id) {
            self.title = n.title.clone();
            self.body = n.body.clone();
            self.current = Some(id);
            self.status = word_status(&self.body);
            self.screen = 1;
            self.editing = true;
            self.edit_field = 1;
            self.field_dirty = true;
        }
    }

    pub fn delete_current(&mut self) {
        if let Some(id) = self.current {
            self.notes.retain(|n| n.id != id);
            self.current = None;
            self.screen = 0;
            self.editing = false;
            self.save();
        }
    }

    pub fn back(&mut self) {
        self.flush();
        self.current = None;
        self.screen = 0;
        self.editing = false;
        self.edit_field = 0;
    }

    /// Write the draft into the model and persist.
    pub fn flush(&mut self) {
        if let Some(id) = self.current {
            let t = self.title.clone();
            let b = self.body.clone();
            if let Some(n) = self.notes.iter_mut().find(|n| n.id == id) {
                if n.title != t || n.body != b {
                    n.title = t;
                    n.body = b;
                    n.updated = now();
                }
            }
            self.sort();
            self.save();
            self.status = word_status(&self.body);
        }
        self.dirty = false;
    }

    /// Called whenever the IME produced new text.
    pub fn mark_edit(&mut self) {
        self.dirty = true;
        self.last_edit_ms = now_ms();
        let mut s = String::new();
        push_num(&mut s, self.body.split_whitespace().count() as u64);
        s.push_str(" words · Saving…");
        self.status = s;
    }

    /// Debounced autosave: 800 ms after the last keystroke.
    pub fn autosave_if_due(&mut self) -> bool {
        if self.dirty && now_ms().saturating_sub(self.last_edit_ms) > 800 {
            self.flush();
            true
        } else {
            false
        }
    }

    pub fn filtered(&self) -> Vec<&Note> {
        if self.query.trim().is_empty() {
            return self.notes.iter().collect();
        }
        let q = self.query.as_bytes();
        self.notes
            .iter()
            .filter(|n| contains_ci(n.title.as_bytes(), q) || contains_ci(n.body.as_bytes(), q))
            .collect()
    }

    // ------------------------------------------------------------ input

    /// Returns true if the UI needs a redraw.
    pub fn tap(&mut self, x: f32, y: f32) -> bool {
        let top = inset_top();
        if self.screen == 0 {
            // + button
            let cx = self_w() - 50.0;
            let cy = top + 54.0;
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= 26.0 * 26.0 {
                self.new_note();
                return true;
            }
            // search field: focus the IME on it, and a clear button when it has text
            if y >= top + 108.0 && y <= top + 154.0 && x >= 22.0 && x <= self_w() - 22.0 {
                if !self.query.is_empty() && x >= self_w() - 56.0 {
                    self.query.clear();
                    self.edit_field = 3;
                    self.field_dirty = true;
                    return true;
                }
                if self.edit_field != 3 {
                    self.edit_field = 3;
                    self.field_dirty = true;
                    return true;
                }
                return false;
            }
            // cards
            if y >= top + LIST_Y {
                let idx = ((y - top - LIST_Y + self.scroll) / ITEM_H) as usize;
                let list = self.filtered();
                if idx < list.len() {
                    let id = list[idx].id;
                    self.open(id);
                    return true;
                }
            }
            false
        } else {
            // back
            if x <= 60.0 && y <= top + 60.0 {
                self.back();
                return true;
            }
            // delete
            if x >= self_w() - 104.0 && y <= top + 55.0 {
                self.delete_current();
                return true;
            }
            // tap into the title or the body: hand focus to the hidden EditText
            let field = if y >= top + 60.0 && y < top + 140.0 {
                1
            } else if y >= top + 140.0 {
                2
            } else {
                0
            };
            if field != 0 && field != self.edit_field {
                self.edit_field = field;
                self.field_dirty = true;
                return true;
            }
            false
        }
    }

    pub fn scroll_by(&mut self, dy: f32) {
        if self.screen != 0 {
            return;
        }
        let view_h = self_h() - LIST_Y - inset_top() - inset_bottom();
        let max = (self.content_h - view_h).max(0.0);
        self.scroll = (self.scroll - dy).clamp(0.0, max);
    }

    // ------------------------------------------------------------ drawing

    pub fn draw(&mut self, g: &Gfx) {
        if self.screen == 0 {
            self.draw_list(g);
        } else {
            self.draw_editor(g);
        }
    }

    fn draw_list(&mut self, g: &Gfx) {
        let w = g.w / g.d;
        let top = g.inset_top;

        g.text("Notes", 24.0, top + 62.0, 34.0, COL_INK, true);

        let count = self.notes.len();
        let label = if count == 1 {
            String::from("1 note")
        } else {
            let mut s = String::from("");
            push_num(&mut s, count as u64);
            s.push_str(" notes");
            s
        };
        g.text(&label, 24.0, top + 88.0, 13.0, COL_INK_3, false);

        // + button
        g.circle(w - 50.0, top + 54.0, 26.0, COL_ACCENT);
        g.text("+", w - 60.0, top + 67.0, 30.0, 0xFFFFFFFFu32 as i32, false);

        // search
        g.round_rect(22.0, top + 108.0, w - 44.0, 46.0, 14.0, COL_SURFACE);
        if self.query.is_empty() {
            g.text("Search notes", 38.0, top + 137.0, 14.0, COL_INK_3, false);
        } else {
            g.text(&self.query, 38.0, top + 137.0, 14.0, COL_INK, false);
            // clear button
            let cx = w - 44.0;
            let cy = top + 131.0;
            g.line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, COL_INK_3);
            g.line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, COL_INK_3);
        }

        if self.edit_field == 3 && self.caret_on {
            let prefix: String = self.query.chars().take(self.cursor).collect();
            let x = 38.0 + g.text_width(&prefix, 14.0, false);
            g.rect(x, top + 122.0, 1.6, 24.0, COL_INK);
        }

        self.content_h = self.filtered().len() as f32 * ITEM_H;
        let list = self.filtered();

        let view_top = top + LIST_Y;
        let view_h = self_h() - LIST_Y - g.inset_bottom;
        g.save();
        g.clip(0.0, view_top, w, view_h);

        if list.is_empty() {
            let cx = w / 2.0;
            let cy = view_top + view_h / 2.0;
            g.round_rect(cx - 39.0, cy - 90.0, 78.0, 78.0, 26.0, COL_ACCENT_SOFT);
            g.rect(cx - 16.0, cy - 63.0, 32.0, 2.0, COL_ACCENT);
            g.rect(cx - 16.0, cy - 53.0, 22.0, 2.0, COL_ACCENT);
            g.rect(cx - 16.0, cy - 43.0, 27.0, 2.0, COL_ACCENT);
            let msg = if self.query.is_empty() {
                "No notes yet"
            } else {
                "Nothing found"
            };
            g.text_center(msg, cx, cy - 2.0, 17.0, COL_INK, true);
            let sub = if self.query.is_empty() {
                "Tap + to write your first one."
            } else {
                "Try a different word."
            };
            g.text_center(sub, cx, cy + 22.0, 13.0, COL_INK_3, false);
        } else {
            for (i, n) in list.iter().enumerate() {
                let y = view_top + i as f32 * ITEM_H - self.scroll;
                if y + CARD_H < view_top || y > view_top + view_h {
                    continue;
                }
                g.round_rect(22.0, y, w - 44.0, CARD_H, 16.0, COL_SURFACE);
                let title = if n.title.trim().is_empty() {
                    "Untitled"
                } else {
                    n.title.as_str()
                };
                g.text_elide(title, 40.0, y + 32.0, 16.0, COL_INK, true, w - 90.0);
                g.text_elide(&preview(n), 40.0, y + 54.0, 13.0, COL_INK_2, false, w - 90.0);
                g.text(&rel_time(n.updated), 40.0, y + 75.0, 11.0, COL_INK_3, false);
                // chevron
                g.line(w - 52.0, y + 42.0, w - 46.0, y + 48.0, 1.6, COL_INK_3);
                g.line(w - 46.0, y + 48.0, w - 52.0, y + 54.0, 1.6, COL_INK_3);
            }
        }
        g.restore();
    }

    fn draw_editor(&mut self, g: &Gfx) {
        let w = g.w / g.d;
        let top = g.inset_top;
        let bottom = g.inset_bottom;

        // back chevron
        g.line(24.0, top + 22.0, 17.0, top + 30.0, 2.0, COL_INK);
        g.line(17.0, top + 30.0, 24.0, top + 38.0, 2.0, COL_INK);

        // delete pill
        g.round_rect(w - 104.0, top + 17.0, 88.0, 34.0, 17.0, COL_SURFACE);
        g.text_center("Delete", w - 60.0, top + 39.0, 13.0, COL_DANGER, true);

        if self.title.is_empty() {
            g.text("Title", 24.0, top + 108.0, 26.0, COL_INK_3, true);
        } else {
            g.text_elide(&self.title, 24.0, top + 108.0, 26.0, COL_INK, true, w - 48.0);
        }

        g.rect(24.0, top + 136.0, w - 48.0, 1.0, COL_LINE);

        let body_top = top + 152.0;
        let body_h = self_h() - 152.0 - 48.0 - bottom;

        if self.body.is_empty() {
            g.text("Start writing…", 24.0, body_top + 24.0, 16.0, COL_INK_3, false);
        } else {
            let lines = wrap_indexed(g, &self.body, w - 48.0, 16.0);
            let mut y = body_top + 24.0;
            g.save();
            g.clip(0.0, body_top, w, body_h);
            for (line, _) in lines.iter() {
                if y > body_top + body_h {
                    break;
                }
                g.text(line, 24.0, y, 16.0, COL_INK, false);
                y += 22.0;
            }
            g.restore();
        }

        // Blinking caret, drawn where the IME's selection sits.
        if self.edit_field != 0 && self.current.is_some() && self.caret_on {
            if self.edit_field == 2 {
                let lines = wrap_indexed(g, &self.body, w - 48.0, 16.0);
                let mut li = 0usize;
                for (i, (_, start)) in lines.iter().enumerate() {
                    if *start <= self.cursor {
                        li = i;
                    } else {
                        break;
                    }
                }
                let (line, start) = &lines[li];
                let col = self.cursor.saturating_sub(*start).min(line.chars().count());
                let prefix: String = line.chars().take(col).collect();
                let x = 24.0 + g.text_width(&prefix, 16.0, false);
                let y = body_top + 24.0 + li as f32 * 22.0;
                g.rect(x, y - 15.0, 1.6, 20.0, COL_INK);
            } else {
                let prefix: String = self.title.chars().take(self.cursor).collect();
                let x = 24.0 + g.text_width(&prefix, 26.0, true);
                g.rect(x, top + 84.0, 1.6, 30.0, COL_INK);
            }
        }

        g.text(&self.status, 24.0, self_h() - bottom - 16.0, 11.5, COL_INK_3, false);
    }
}

// --------------------------------------------------------------- helpers

pub fn now() -> u64 {
    #[repr(C)]
    struct Ts {
        sec: i64,
        nsec: i64,
    }
    extern "C" {
        fn clock_gettime(clk: i32, tp: *mut Ts) -> i32;
    }
    let mut t = Ts { sec: 0, nsec: 0 };
    unsafe { clock_gettime(0, &mut t) };
    t.sec.max(0) as u64
}

pub fn now_ms() -> u64 {
    #[repr(C)]
    struct Ts {
        sec: i64,
        nsec: i64,
    }
    extern "C" {
        fn clock_gettime(clk: i32, tp: *mut Ts) -> i32;
    }
    let mut t = Ts { sec: 0, nsec: 0 };
    unsafe { clock_gettime(1, &mut t) };
    t.sec.max(0) as u64 * 1000 + (t.nsec.max(0) as u64) / 1_000_000
}

fn word_status(body: &str) -> String {
    let mut s = String::new();
    push_num(&mut s, body.split_whitespace().count() as u64);
    s.push_str(" words · Saved");
    s
}

pub fn push_num(s: &mut String, mut n: u64) {
    if n == 0 {
        s.push('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    for &b in &buf[i..] {
        s.push(b as char);
    }
}

fn preview(n: &Note) -> String {
    let body = n.body.trim();
    if body.is_empty() {
        return String::from("No additional text");
    }
    let line = body.lines().next().unwrap_or("");
    let mut s = String::new();
    for c in line.chars().take(90) {
        s.push(c);
    }
    s
}

fn rel_time(updated: u64) -> String {
    let age = now().saturating_sub(updated);
    let mut s = String::new();
    match age {
        0..=59 => s.push_str("just now"),
        60..=3599 => {
            push_num(&mut s, age / 60);
            s.push_str("m ago");
        }
        3600..=86399 => {
            push_num(&mut s, age / 3600);
            s.push_str("h ago");
        }
        86400..=604799 => {
            push_num(&mut s, age / 86400);
            s.push_str("d ago");
        }
        604800..=2591999 => {
            push_num(&mut s, age / 604800);
            s.push_str("w ago");
        }
        _ => {
            push_num(&mut s, age / 2592000);
            s.push_str("mo ago");
        }
    }
    s
}

/// Word-wrap that also reports the char index each line starts at, so the caret
/// can be placed without a second pass over the text.
/// ASCII case-insensitive substring search. `str::to_lowercase` + `contains`
/// pull in the whole Unicode case-folding table (~4.5 KB) for a search box.
fn contains_ci(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > hay.len() {
        return false;
    }
    for i in 0..=(hay.len() - needle.len()) {
        let mut ok = true;
        for j in 0..needle.len() {
            if hay[i + j].to_ascii_lowercase() != needle[j].to_ascii_lowercase() {
                ok = false;
                break;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

fn wrap_indexed(g: &Gfx, text: &str, max_w: f32, size: f32) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    for para in text.split('\n') {
        let para_start = idx;
        idx += para.chars().count() + 1;
        if para.is_empty() {
            out.push((String::new(), para_start));
            continue;
        }
        let mut line = String::new();
        let mut line_start = para_start;
        let mut word_idx = para_start;
        for word in para.split(' ') {
            let mut cand = line.clone();
            if !cand.is_empty() {
                cand.push(' ');
            }
            cand.push_str(word);
            if g.text_width(&cand, size, false) > max_w && !line.is_empty() {
                out.push((line, line_start));
                line = String::from(word);
                line_start = word_idx;
            } else {
                line = cand;
            }
            word_idx += word.chars().count() + 1;
        }
        out.push((line, line_start));
    }
    out
}

// Window size in dp is stashed here so layout code can use it without threading
// Gfx through every hit-test.
static mut VIEW_W: f32 = 412.0;
static mut VIEW_H: f32 = 760.0;
static mut INSET_TOP: f32 = 28.0;
static mut INSET_BOTTOM: f32 = 16.0;

pub fn set_view(w: f32, h: f32, top: f32, bottom: f32) {
    unsafe {
        VIEW_W = w;
        VIEW_H = h;
        INSET_TOP = top;
        INSET_BOTTOM = bottom;
    }
}

fn inset_top() -> f32 {
    unsafe { INSET_TOP }
}

fn inset_bottom() -> f32 {
    unsafe { INSET_BOTTOM }
}

fn self_w() -> f32 {
    unsafe { VIEW_W }
}

fn self_h() -> f32 {
    unsafe { VIEW_H }
}
