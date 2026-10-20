use super::{NoticeLabels, TrayLabels};
use crate::client::Status;

pub(super) const TRAY: TrayLabels = TrayLabels {
    summary: "Summary",
    open: "Open NAS Sync",
    pause: "Pause sync",
    resume: "Resume sync",
    quit: "Quit",
    status,
};

pub(super) const NOTICES: NoticeLabels = NoticeLabels {
    conflict: "Sync conflict",
    failed: "File not synced",
    unreachable: "NAS unreachable",
    resume_sync: "Sync will resume when the NAS will be available again",
    more_files: |first, count| format!("{first} and {count} more files"),
};

fn status(status: Status) -> &'static str {
    match status {
        Status::Unconfigured => "NAS not connected",
        Status::Paused => "Sync paused",
        Status::Disconnected => "NAS unreachable",
        Status::Syncing => "Syncing",
        Status::Synced => "Synced",
        Status::Error => "Sync errors",
    }
}
