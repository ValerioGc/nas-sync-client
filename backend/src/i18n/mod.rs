//! Localized text for native tray menus and system notifications.

mod en;
mod it;

use crate::client::Status;

pub struct TrayLabels {
    pub summary: &'static str,
    pub open: &'static str,
    pub pause: &'static str,
    pub resume: &'static str,
    pub quit: &'static str,
    pub status: fn(Status) -> &'static str,
}

pub struct NoticeLabels {
    pub conflict: &'static str,
    pub failed: &'static str,
    pub unreachable: &'static str,
    pub resume_sync: &'static str,
    pub more_files: fn(&str, usize) -> String,
}

pub fn tray_labels(locale: &str) -> &'static TrayLabels {
    if locale == "en" {
        &en::TRAY
    } else {
        &it::TRAY
    }
}

pub fn notice_labels(locale: &str) -> &'static NoticeLabels {
    if locale == "en" {
        &en::NOTICES
    } else {
        &it::NOTICES
    }
}
