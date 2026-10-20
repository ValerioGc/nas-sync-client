//! Device private key stored in Windows Credential Manager or Linux Secret Service.

use crate::error::ClientError;

pub trait Secrets: Send + Sync {
    fn save(&self, key: &str) -> Result<(), ClientError>;
    fn load(&self) -> Result<Option<String>, ClientError>;
    fn forget(&self) -> Result<(), ClientError>;
}

const SERVICE: &str = "nas-sync-client";
const ACCOUNT: &str = "device-key";

pub struct Keyring;

impl Keyring {
    fn entry() -> Result<keyring::Entry, ClientError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|_| ClientError::SecretUnavailable)
    }
}

impl Secrets for Keyring {
    fn save(&self, key: &str) -> Result<(), ClientError> {
        Self::entry()?
            .set_password(key)
            .map_err(|_| ClientError::SecretUnavailable)
    }

    fn load(&self) -> Result<Option<String>, ClientError> {
        match Self::entry()?.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(ClientError::SecretUnavailable),
        }
    }

    fn forget(&self) -> Result<(), ClientError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(ClientError::SecretUnavailable),
        }
    }
}

/// In-memory credential store restricted to tests.
#[cfg(test)]
#[derive(Default)]
pub struct Memory(std::sync::Mutex<Option<String>>);

#[cfg(test)]
impl Secrets for Memory {
    fn save(&self, key: &str) -> Result<(), ClientError> {
        *self.0.lock().map_err(|_| ClientError::SecretUnavailable)? = Some(key.to_owned());
        Ok(())
    }

    fn load(&self) -> Result<Option<String>, ClientError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ClientError::SecretUnavailable)?
            .clone())
    }

    fn forget(&self) -> Result<(), ClientError> {
        *self.0.lock().map_err(|_| ClientError::SecretUnavailable)? = None;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/secret/mod.rs"]
mod tests;
