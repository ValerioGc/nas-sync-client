//! Persistent connection, certificate, and preferences in `client.json`. Private keys use
//! `secret.rs`.

use crate::{error::ClientError, storage};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// The NAS the computer is paired with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub name: String,
    pub host: String,
    pub port: u16,
    /// Trusted NAS certificate authority in PEM format.
    pub authority: String,
    /// The certificate of this computer, in PEM.
    pub certificate: String,
    /// The identity of the sync engine of the NAS and the port it listens on.
    pub nas_id: String,
    pub sync_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub link: Option<Link>,
    pub paused: bool,
    pub notifications: bool,
    pub autostart: bool,
    /// Bandwidth limits in KiB/s, 0 for none.
    pub max_send_kbps: u32,
    pub max_recv_kbps: u32,
    pub appearance: Appearance,
    /// Locally confirmed computer folders indexed by sync ID.
    /// Only these paths are sent to the engine. Legacy settings adopt the first NAS list
    /// when explicit folder choices have not yet been initialized.
    pub chosen: Option<BTreeMap<String, String>>,
}

/// The language and the theme of every window of the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub locale: String,
    pub theme: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            locale: "it".to_owned(),
            theme: "dark".to_owned(),
        }
    }
}

impl Appearance {
    pub fn valid(&self) -> bool {
        matches!(self.locale.as_str(), "it" | "en")
            && matches!(self.theme.as_str(), "dark" | "light")
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            link: None,
            paused: false,
            notifications: true,
            autostart: false,
            max_send_kbps: 0,
            max_recv_kbps: 0,
            appearance: Appearance::default(),
            chosen: None,
        }
    }
}

/// The preferences edited from the options dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub notifications: bool,
    pub autostart: bool,
    pub max_send_kbps: u32,
    pub max_recv_kbps: u32,
}

/// The highest bandwidth limit accepted, about 1 GiB/s.
pub const MAX_KBPS: u32 = 1_048_576;

impl Settings {
    pub fn preferences(&self) -> Preferences {
        Preferences {
            notifications: self.notifications,
            autostart: self.autostart,
            max_send_kbps: self.max_send_kbps,
            max_recv_kbps: self.max_recv_kbps,
        }
    }

    pub fn apply(&mut self, preferences: Preferences) -> Result<(), ClientError> {
        if preferences.max_send_kbps > MAX_KBPS || preferences.max_recv_kbps > MAX_KBPS {
            return Err(ClientError::InvalidSync);
        }
        self.notifications = preferences.notifications;
        self.autostart = preferences.autostart;
        self.max_send_kbps = preferences.max_send_kbps;
        self.max_recv_kbps = preferences.max_recv_kbps;
        Ok(())
    }
}

pub fn path(directory: &Path) -> PathBuf {
    directory.join("client.json")
}

pub fn load(directory: &Path) -> Result<Settings, ClientError> {
    Ok(storage::read(&path(directory))?.unwrap_or_default())
}

pub fn save(directory: &Path, settings: &Settings) -> Result<(), ClientError> {
    storage::write(&path(directory), settings)
}

#[cfg(test)]
#[path = "../tests/settings/mod.rs"]
mod tests;
