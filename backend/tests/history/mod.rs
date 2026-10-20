use super::*;
use serde_json::json;

fn change(kind: &str, action: &str, path: &str) -> Value {
    json!({
        "id": 1,
        "type": kind,
        "time": "2026-10-16T10:00:00Z",
        "data": { "folder": "f-1", "type": "file", "action": action, "path": path },
    })
}

#[test]
fn local_and_remote_changes_of_files_become_entries() {
    let upload = entry_of(&change(
        "LocalChangeDetected",
        "modified",
        "docs\\report.pdf",
    ))
    .unwrap();
    assert_eq!(upload.direction, Direction::Upload);
    assert_eq!(upload.path, "docs/report.pdf");
    assert_eq!(upload.action, "modified");
    assert!(!upload.conflict);
    let download = entry_of(&change(
        "RemoteChangeDetected",
        "added",
        "photo.sync-conflict-20261016-100000-ABCDEFG.jpg",
    ))
    .unwrap();
    assert_eq!(download.direction, Direction::Download);
    assert!(download.conflict);
}

#[test]
fn folders_unknown_actions_and_other_events_are_left_out() {
    let mut folder = change("LocalChangeDetected", "added", "docs");
    folder["data"]["type"] = json!("dir");
    assert!(entry_of(&folder).is_none());
    assert!(entry_of(&change("LocalChangeDetected", "touched", "a.txt")).is_none());
    assert!(entry_of(&change("StateChanged", "added", "a.txt")).is_none());
    assert!(entry_of(&json!({ "type": "LocalChangeDetected" })).is_none());
}

#[test]
fn a_failed_item_is_an_entry_and_a_successful_one_is_not() {
    let failed = json!({
        "type": "ItemFinished",
        "time": "2026-10-16T10:00:00Z",
        "data": { "folder": "f-1", "item": "big.iso", "error": "no space" },
    });
    assert_eq!(entry_of(&failed).unwrap().action, "failed");
    let mut done = failed.clone();
    done["data"]["error"] = Value::Null;
    assert!(entry_of(&done).is_none());
}

fn entry(index: usize, folder: &str) -> Entry {
    Entry {
        time: format!("t{index}"),
        folder: folder.into(),
        path: format!("{index}.txt"),
        action: "added".into(),
        direction: Direction::Upload,
        conflict: false,
    }
}

#[test]
fn the_history_keeps_the_newest_entries_within_its_limit() {
    let mut history = History::default();
    for index in 0..LIMIT + 5 {
        history.push(entry(index, "f-1"));
    }
    assert_eq!(history.all().len(), LIMIT);
    assert_eq!(history.latest(2)[0].path, format!("{}.txt", LIMIT + 4));
    let restored = History::from_entries(history.latest(3));
    assert_eq!(restored.all(), history.latest(3));
    history.push(entry(0, "f-2"));
    history.retain_folders(&["f-2".to_owned()]);
    assert_eq!(history.all().len(), 1);
    history.clear();
    assert!(history.all().is_empty());
}
