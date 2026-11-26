use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub updated: u64,
}

pub struct Store {
    path: PathBuf,
    pub notes: Vec<Note>,
}

impl Store {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("notes.json");
        let notes = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Vec<Note>>(&raw).ok())
            .unwrap_or_default();
        let mut store = Store { path, notes };
        store.sort();
        store
    }

    fn sort(&mut self) {
        self.notes.sort_by(|a, b| b.updated.cmp(&a.updated));
    }

    pub fn next_id(&self) -> u64 {
        self.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    pub fn upsert(&mut self, note: Note) {
        match self.notes.iter_mut().find(|n| n.id == note.id) {
            Some(existing) => *existing = note,
            None => self.notes.push(note),
        }
        self.sort();
    }

    pub fn delete(&mut self, id: u64) {
        self.notes.retain(|n| n.id != id);
    }

    /// Write to a sibling temp file first, then rename, so a crash mid-write
    /// never leaves a half-written notes.json behind.
    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self.notes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)
    }
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("RustNotepad");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/rust-notepad");
    }
    std::env::temp_dir().join("rust-notepad")
}
