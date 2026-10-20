use serde::Serialize;

/// IPC error codes translated by the interface, excluding machine-specific details.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientError {
    InvalidAddress,
    NetworkUnavailable,
    NasUnreachable,
    NotANas,
    TrustChanged,
    InvalidCredentials,
    TooManyAttempts,
    SecurityRequired,
    NotPaired,
    AppMissing,
    EngineUnavailable,
    StorageUnavailable,
    SecretUnavailable,
    InvalidSync,
    FolderTaken,
    NotConnected,
    Busy,
}
