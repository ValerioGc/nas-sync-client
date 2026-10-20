use super::*;
use crate::history::Direction;

fn entry(path: &str, action: &str, conflict: bool) -> Entry {
    Entry {
        time: "t".into(),
        folder: "f-1".into(),
        path: path.into(),
        action: action.into(),
        direction: Direction::Download,
        conflict,
    }
}

#[test]
fn conflicts_come_before_failures_and_plain_changes_tell_nothing() {
    assert_eq!(of_entries(&[entry("a.txt", "added", false)], "it"), None);
    let notice = of_entries(
        &[
            entry("docs/b.txt", "failed", false),
            entry("docs/a.sync-conflict-1.txt", "added", true),
        ],
        "it",
    )
    .unwrap();
    assert_eq!(notice.title, "Conflitto di sincronizzazione");
    assert_eq!(notice.body, "a.sync-conflict-1.txt");
    let failed = of_entries(
        &[
            entry("x/1.iso", "failed", false),
            entry("2.iso", "failed", false),
        ],
        "en",
    )
    .unwrap();
    assert_eq!(failed.title, "File not synced");
    assert_eq!(failed.body, "1.iso and 1 more files");
    let italian = of_entries(
        &[
            entry("1.iso", "failed", false),
            entry("2.iso", "failed", false),
        ],
        "it",
    )
    .unwrap();
    assert_eq!(italian.body, "1.iso e altri 1 file");
}

#[test]
fn an_unreachable_nas_is_told_in_the_language_of_the_client() {
    assert_eq!(unreachable("it").title, "NAS non raggiungibile");
    assert_eq!(unreachable("en").title, "NAS unreachable");
}
