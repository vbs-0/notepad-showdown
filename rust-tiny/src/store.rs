//! Notes on disk: a length-prefixed binary blob. No serde, no JSON parser —
//! every byte of parsing code is bytes in the APK.

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

extern "C" {
    fn open(path: *const c_char, flags: i32, mode: i32) -> i32;
    fn read(fd: i32, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: i32, buf: *const c_void, n: usize) -> isize;
    fn close(fd: i32) -> i32;
    fn rename(old: *const c_char, new: *const c_char) -> i32;
}

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_CREAT: i32 = 0x40;
const O_TRUNC: i32 = 0x200;

#[derive(Clone)]
pub struct Note {
    pub id: u32,
    pub updated: u64,
    pub title: String,
    pub body: String,
}

pub fn load(dir: &str) -> Vec<Note> {
    let mut path = String::from(dir);
    path.push_str("/notes.bin");
    path.push('\0');

    let fd = unsafe { open(path.as_ptr() as *const c_char, O_RDONLY, 0) };
    if fd < 0 {
        return Vec::new();
    }
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = unsafe { read(fd, chunk.as_mut_ptr() as *mut c_void, chunk.len()) };
        if n <= 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n as usize]);
    }
    unsafe { close(fd) };

    let mut notes = Vec::new();
    let mut p = 0usize;
    let count = take_u32(&data, &mut p);
    for _ in 0..count {
        let id = take_u32(&data, &mut p);
        let updated = take_u64(&data, &mut p);
        let tl = take_u32(&data, &mut p) as usize;
        let title = take_str(&data, &mut p, tl);
        let bl = take_u32(&data, &mut p) as usize;
        let body = take_str(&data, &mut p, bl);
        notes.push(Note {
            id,
            updated,
            title,
            body,
        });
    }
    // Insertion sort, same reason as in app.rs: no quicksort machinery.
    for i in 1..notes.len() {
        let mut j = i;
        while j > 0 && notes[j - 1].updated < notes[j].updated {
            notes.swap(j - 1, j);
            j -= 1;
        }
    }
    notes
}

fn take_str(d: &[u8], p: &mut usize, len: usize) -> String {
    if *p + len > d.len() {
        return String::new();
    }
    // `from_utf8_lossy` drags in the UTF-8 chunk iterator (~450 B); our own
    // files are always valid UTF-8, so a plain check is enough.
    let s = match core::str::from_utf8(&d[*p..*p + len]) {
        Ok(s) => String::from(s),
        Err(_) => String::new(),
    };
    *p += len;
    s
}

pub fn save(dir: &str, notes: &[Note]) -> bool {
    let mut path = String::from(dir);
    path.push_str("/notes.bin.tmp");
    path.push('\0');

    let mut out = Vec::with_capacity(1024);
    out.extend_from_slice(&(notes.len() as u32).to_le_bytes());
    for n in notes {
        out.extend_from_slice(&n.id.to_le_bytes());
        out.extend_from_slice(&n.updated.to_le_bytes());
        out.extend_from_slice(&(n.title.len() as u32).to_le_bytes());
        out.extend_from_slice(n.title.as_bytes());
        out.extend_from_slice(&(n.body.len() as u32).to_le_bytes());
        out.extend_from_slice(n.body.as_bytes());
    }

    let fd = unsafe { open(path.as_ptr() as *const c_char, O_WRONLY | O_CREAT | O_TRUNC, 0o600) };
    if fd < 0 {
        return false;
    }
    let mut written = 0usize;
    while written < out.len() {
        let n = unsafe {
            write(
                fd,
                out.as_ptr().add(written) as *const c_void,
                out.len() - written,
            )
        };
        if n <= 0 {
            break;
        }
        written += n as usize;
    }
    unsafe { close(fd) };
    if written != out.len() {
        return false;
    }
    // Swap the finished temp file into place, so a crash mid-write can never
    // leave a half-written notes.bin.
    let mut final_path = String::from(dir);
    final_path.push_str("/notes.bin");
    final_path.push('\0');
    unsafe {
        rename(
            path.as_ptr() as *const c_char,
            final_path.as_ptr() as *const c_char,
        ) == 0
    }
}

fn take_u32(d: &[u8], p: &mut usize) -> u32 {
    if *p + 4 > d.len() {
        return 0;
    }
    let v = u32::from_le_bytes([d[*p], d[*p + 1], d[*p + 2], d[*p + 3]]);
    *p += 4;
    v
}

fn take_u64(d: &[u8], p: &mut usize) -> u64 {
    if *p + 8 > d.len() {
        return 0;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[*p..*p + 8]);
    *p += 8;
    u64::from_le_bytes(b)
}
