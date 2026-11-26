slint::include_modules!();

mod store;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};

use store::{Note, Store};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn preview_of(note: &Note) -> String {
    let body = note.body.trim();
    if body.is_empty() {
        return "No additional text".to_string();
    }
    body.lines().next().unwrap_or("").chars().take(90).collect()
}

fn relative_time(updated: u64) -> String {
    let age = now_secs().saturating_sub(updated);
    match age {
        0..=59 => "just now".to_string(),
        60..=3_599 => format!("{}m ago", age / 60),
        3_600..=86_399 => format!("{}h ago", age / 3_600),
        86_400..=604_799 => format!("{}d ago", age / 86_400),
        604_800..=2_591_999 => format!("{}w ago", age / 604_800),
        _ => format!("{}mo ago", age / 2_592_000),
    }
}

fn refresh(ui: &AppWindow, store: &Store, query: &str) {
    let needle = query.trim().to_lowercase();
    let items: Vec<NoteItem> = store
        .notes
        .iter()
        .filter(|n| {
            needle.is_empty()
                || n.title.to_lowercase().contains(&needle)
                || n.body.to_lowercase().contains(&needle)
        })
        .map(|n| NoteItem {
            id: n.id as i32,
            title: if n.title.trim().is_empty() {
                SharedString::from("Untitled")
            } else {
                SharedString::from(n.title.clone())
            },
            preview: SharedString::from(preview_of(n)),
            updated: SharedString::from(relative_time(n.updated)),
        })
        .collect();

    let total = store.notes.len();
    ui.set_notes(ModelRc::new(VecModel::from(items)));
    ui.set_count_label(SharedString::from(if total == 1 {
        "1 note".to_string()
    } else {
        format!("{total} notes")
    }));
}

fn persist_draft(ui: &AppWindow, store: &Rc<RefCell<Store>>, current: &Rc<RefCell<Option<u64>>>) {
    let Some(id) = *current.borrow() else { return };
    let title = ui.get_draft_title().to_string();
    let body = ui.get_draft_body().to_string();
    let mut store = store.borrow_mut();
    if let Some(note) = store.notes.iter_mut().find(|n| n.id == id) {
        if note.title != title || note.body != body {
            note.title = title;
            note.body = body;
            note.updated = now_secs();
        }
    }
    let _ = store.save();
}

pub fn run(data_dir: Option<PathBuf>) {
    let dir = data_dir.unwrap_or_else(store::default_data_dir);
    let _ = std::fs::create_dir_all(&dir);

    let ui = AppWindow::new().expect("failed to create the window");
    let store = Rc::new(RefCell::new(Store::load(&dir)));
    let current: Rc<RefCell<Option<u64>>> = Rc::new(RefCell::new(None));

    refresh(&ui, &store.borrow(), "");

    ui.on_new_note({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        let current = current.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let id = store.borrow().next_id();
            store.borrow_mut().upsert(Note {
                id,
                title: String::new(),
                body: String::new(),
                updated: now_secs(),
            });
            let _ = store.borrow().save();
            *current.borrow_mut() = Some(id);
            ui.set_draft_title(SharedString::default());
            ui.set_draft_body(SharedString::default());
            ui.set_status(SharedString::from("0 words · Saved"));
            ui.set_screen(1);
        }
    });

    ui.on_open_note({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        let current = current.clone();
        move |id| {
            let Some(ui) = ui_weak.upgrade() else { return };
            let store_ref = store.borrow();
            if let Some(note) = store_ref.notes.iter().find(|n| n.id == id as u64) {
                *current.borrow_mut() = Some(note.id);
                ui.set_draft_title(SharedString::from(note.title.clone()));
                ui.set_draft_body(SharedString::from(note.body.clone()));
                ui.set_status(SharedString::from(format!(
                    "{} words · Saved",
                    word_count(&note.body)
                )));
                ui.set_screen(1);
            }
        }
    });

    ui.on_delete_note({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        let current = current.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let Some(id) = *current.borrow() else { return };
            store.borrow_mut().delete(id);
            let _ = store.borrow().save();
            *current.borrow_mut() = None;
            ui.set_screen(0);
            let query = ui.get_query();
            refresh(&ui, &store.borrow(), &query);
        }
    });

    ui.on_go_back({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        let current = current.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            persist_draft(&ui, &store, &current);
            *current.borrow_mut() = None;
            ui.set_screen(0);
            let query = ui.get_query();
            refresh(&ui, &store.borrow(), &query);
        }
    });

    ui.on_search_changed({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let query = ui.get_query();
            refresh(&ui, &store.borrow(), &query);
        }
    });

    // Typing marks the note dirty straight away; the actual disk write is
    // debounced so a fast typist doesn't hammer storage on every keystroke.
    let save_timer = Rc::new(Timer::default());
    ui.on_edited({
        let ui_weak = ui.as_weak();
        let store = store.clone();
        let current = current.clone();
        let save_timer = save_timer.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let title = ui.get_draft_title().to_string();
            let body = ui.get_draft_body().to_string();
            ui.set_status(SharedString::from(format!(
                "{} words · Saving…",
                word_count(&body)
            )));

            if let Some(id) = *current.borrow() {
                let mut store_ref = store.borrow_mut();
                if let Some(note) = store_ref.notes.iter_mut().find(|n| n.id == id) {
                    note.title = title;
                    note.body = body;
                    note.updated = now_secs();
                }
            }

            let ui_weak = ui_weak.clone();
            let store = store.clone();
            save_timer.start(TimerMode::SingleShot, Duration::from_millis(700), move || {
                let Some(ui) = ui_weak.upgrade() else { return };
                let _ = store.borrow().save();
                let words = word_count(&ui.get_draft_body());
                ui.set_status(SharedString::from(format!("{words} words · Saved")));
            });
        }
    });

    ui.run().expect("UI error");
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: slint::android::AndroidApp) {
    let data_dir = app.internal_data_path();
    let _ = slint::android::init(app);
    run(data_dir);
}

/// The prebuilt Skia binaries Slint links against were compiled with an older
/// libc++, which exported `std::__ndk1::__hash_memory`. NDK 28's libc++ no longer
/// ships it, so the linker leaves the symbol undefined and Android refuses to
/// dlopen the library. Any deterministic hash is a valid stand-in: libc++ only
/// uses it to choose a bucket inside its hash containers.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn _ZNSt6__ndk113__hash_memoryEPKvm(ptr: *const u8, len: usize) -> usize {
    let mut hash: usize = 0xcbf2_9ce4_8422_2325;
    unsafe {
        for i in 0..len {
            hash ^= *ptr.add(i) as usize;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}
