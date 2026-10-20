//! Newest-first sync history stored in `history.json`, bounded by `LIMIT`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

pub const LIMIT: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub time: String,
    pub folder: String,
    pub path: String,
    /// `added`, `modified`, `deleted` or `failed`.
    pub action: String,
    pub direction: Direction,
    /// A copy kept because both sides changed the file.
    #[serde(default)]
    pub conflict: bool,
}

/// Converts file-related engine events into history entries.
pub fn entry_of(event: &Value) -> Option<Entry> {
    let data = &event["data"];
    let time = event["time"].as_str()?.to_owned();
    let folder = data["folder"]
        .as_str()
        .or_else(|| data["folderID"].as_str())?
        .to_owned();
    let (path, action, direction) = match event["type"].as_str()? {
        "LocalChangeDetected" | "RemoteChangeDetected" => {
            if data["type"].as_str() != Some("file") {
                return None;
            }
            let action = data["action"].as_str()?;
            if !matches!(action, "added" | "modified" | "deleted") {
                return None;
            }
            let direction = if event["type"] == "LocalChangeDetected" {
                Direction::Upload
            } else {
                Direction::Download
            };
            (data["path"].as_str()?, action, direction)
        }
        "ItemFinished" => {
            data["error"].as_str().filter(|error| !error.is_empty())?;
            (data["item"].as_str()?, "failed", Direction::Download)
        }
        _ => return None,
    };
    let path = path.replace('\\', "/");
    Some(Entry {
        conflict: path.contains(".sync-conflict-"),
        time,
        folder,
        path,
        action: action.to_owned(),
        direction,
    })
}

#[derive(Debug, Default)]
pub struct History {
    entries: VecDeque<Entry>,
}

impl History {
    pub fn from_entries(entries: Vec<Entry>) -> Self {
        let mut history = Self::default();
        for entry in entries.into_iter().rev() {
            history.push(entry);
        }
        history
    }

    /// Adds the newest entry, dropping the oldest beyond `LIMIT`.
    pub fn push(&mut self, entry: Entry) {
        self.entries.push_front(entry);
        self.entries.truncate(LIMIT);
    }

    /// The newest `count` entries.
    pub fn latest(&self, count: usize) -> Vec<Entry> {
        self.entries.iter().take(count).cloned().collect()
    }

    pub fn all(&self) -> Vec<Entry> {
        self.entries.iter().cloned().collect()
    }

    /// Forgets the entries of folders no longer synced.
    pub fn retain_folders(&mut self, folders: &[String]) {
        self.entries.retain(|entry| folders.contains(&entry.folder));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "../tests/history/mod.rs"]
mod tests;
