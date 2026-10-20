use super::{NoticeLabels, TrayLabels};
use crate::client::Status;

pub(super) const TRAY: TrayLabels = TrayLabels {
    summary: "Riepilogo",
    open: "Apri NAS Sync",
    pause: "Sospendi sincronizzazione",
    resume: "Riprendi sincronizzazione",
    quit: "Esci",
    status,
};

pub(super) const NOTICES: NoticeLabels = NoticeLabels {
    conflict: "Conflitto di sincronizzazione",
    failed: "File non sincronizzato",
    unreachable: "NAS non raggiungibile",
    resume_sync: "La sincronizzazione riprenderà quando il NAS tornerà disponibile",
    more_files: |first, count| format!("{first} e altri {count} file"),
};

fn status(status: Status) -> &'static str {
    match status {
        Status::Unconfigured => "NAS non collegato",
        Status::Paused => "Sincronizzazione sospesa",
        Status::Disconnected => "NAS non raggiungibile",
        Status::Syncing => "Sincronizzazione in corso",
        Status::Synced => "Sincronizzato",
        Status::Error => "Errore di sincronizzazione",
    }
}
