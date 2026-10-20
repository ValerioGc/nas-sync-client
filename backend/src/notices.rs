//! Deduplicated system notifications for conflicts, sync failures, and prolonged NAS
//! unavailability.

use crate::{history::Entry, i18n};

/// Localized notification title and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: &'static str,
    pub body: String,
}

/// Notification for conflicting or failed file updates.
pub fn of_entries(entries: &[Entry], locale: &str) -> Option<Notice> {
    let labels = i18n::notice_labels(locale);
    let conflicts: Vec<&Entry> = entries.iter().filter(|entry| entry.conflict).collect();
    let failed: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.action == "failed")
        .collect();
    let (title, list) = if !conflicts.is_empty() {
        (labels.conflict, conflicts)
    } else if !failed.is_empty() {
        (labels.failed, failed)
    } else {
        return None;
    };
    let first = list[0]
        .path
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned();
    let body = match list.len() {
        1 => first,
        count => (labels.more_files)(&first, count - 1),
    };
    Some(Notice { title, body })
}

/// The notification of a NAS out of reach.
pub fn unreachable(locale: &str) -> Notice {
    let labels = i18n::notice_labels(locale);
    Notice {
        title: labels.unreachable,
        body: labels.resume_sync.to_owned(),
    }
}

#[cfg(test)]
#[path = "../tests/notices/mod.rs"]
mod tests;
