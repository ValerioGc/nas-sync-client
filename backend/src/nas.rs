//! The gateway of the NAS, over HTTPS. The account password is used once to obtain the
//! certificate of this computer, and the sync routes are reached with that certificate only.

use crate::{error::ClientError, tls};
use reqwest::{header, Client, Method, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(20);
const SESSION_COOKIE: &str = "nas_session";

/// HTTPS gateway address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    pub fn new(host: &str, port: u16) -> Result<Self, ClientError> {
        let host = host.trim().trim_start_matches('[').trim_end_matches(']');
        let valid = !host.is_empty()
            && host.len() <= 253
            && port != 0
            && (host.parse::<std::net::IpAddr>().is_ok()
                || host.split('.').all(|label| {
                    (1..=63).contains(&label.len())
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                }));
        valid
            .then(|| Self {
                host: host.to_owned(),
                port,
            })
            .ok_or(ClientError::InvalidAddress)
    }

    pub fn url(&self, path: &str) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("https://{host}:{}{path}", self.port)
    }

    /// The address of the sync engine of the NAS, on its port.
    pub fn sync_address(&self, port: u16) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        format!("tcp://{host}:{port}")
    }
}

fn client(config: rustls::ClientConfig) -> Result<Client, ClientError> {
    Client::builder()
        .use_preconfigured_tls(config)
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| ClientError::NetworkUnavailable)
}

fn unreachable(_: reqwest::Error) -> ClientError {
    ClientError::NasUnreachable
}

/// NAS authority and fingerprint collected during initial connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    #[serde(skip)]
    pub authority: String,
    pub fingerprint: String,
}

/// Reaches the gateway, learns its authority and checks that its certificate comes from it.
pub async fn probe(endpoint: &Endpoint) -> Result<Probe, ClientError> {
    let (config, capture) = tls::capturing()?;
    let client = client(config)?;
    let health = client
        .get(endpoint.url("/health"))
        .send()
        .await
        .map_err(unreachable)?;
    if !health.status().is_success() || health.text().await.map_err(unreachable)?.trim() != "ok" {
        return Err(ClientError::NotANas);
    }
    let authority = client
        .get(endpoint.url("/auth/ca.crt"))
        .send()
        .await
        .map_err(unreachable)?;
    if !authority.status().is_success() {
        return Err(ClientError::NotANas);
    }
    let authority = authority.text().await.map_err(unreachable)?;
    let fingerprint = tls::authority_fingerprint(&authority).ok_or(ClientError::NotANas)?;
    if !tls::issued_by(&authority, &capture.chain()) {
        return Err(ClientError::NotANas);
    }
    Ok(Probe {
        authority,
        fingerprint,
    })
}

/// Device certificate, private key, and NAS authority returned in PEM format.
#[derive(Debug, Clone, Deserialize)]
pub struct Issued {
    pub certificate: String,
    pub key: String,
    pub authority: String,
}

/// The error of a sign-in or of a confirmation, from what the gateway answered.
fn refused(response: &Response) -> ClientError {
    let html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    match response.status() {
        StatusCode::FORBIDDEN if html => ClientError::SecurityRequired,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST => {
            ClientError::InvalidCredentials
        }
        StatusCode::TOO_MANY_REQUESTS => ClientError::TooManyAttempts,
        _ => ClientError::NasUnreachable,
    }
}

/// The value of the session cookie in what the sign-in answered.
fn session_of(response: &Response) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .find(|pair| pair.starts_with(&format!("{SESSION_COOKIE}=")))
        .map(str::to_owned)
}

/// Signs in, confirms the password, obtains the certificate of `name` and signs out.
pub async fn enroll(
    endpoint: &Endpoint,
    authority: &str,
    username: &str,
    password: &str,
    name: &str,
    sync_id: &str,
) -> Result<Issued, ClientError> {
    let client = client(tls::pinned(authority, None)?)?;
    let login = client
        .post(endpoint.url("/auth/login"))
        .json(&json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(unreachable)?;
    if !login.status().is_success() {
        return Err(refused(&login));
    }
    let cookie = session_of(&login).ok_or(ClientError::NotANas)?;
    let result = async {
        let confirm = client
            .post(endpoint.url("/auth/confirm"))
            .header(header::COOKIE, &cookie)
            .json(&json!({ "password": password }))
            .send()
            .await
            .map_err(unreachable)?;
        if !confirm.status().is_success() {
            return Err(refused(&confirm));
        }
        let code = confirm
            .json::<serde_json::Value>()
            .await
            .map_err(|_| ClientError::NotANas)?["code"]
            .as_str()
            .map(str::to_owned)
            .ok_or(ClientError::NotANas)?;
        let issued = client
            .post(endpoint.url("/auth/clients"))
            .header(header::COOKIE, &cookie)
            .header("x-nas-confirmation", code)
            .json(&json!({ "name": name, "syncId": sync_id }))
            .send()
            .await
            .map_err(unreachable)?;
        match issued.status() {
            StatusCode::CREATED => issued
                .json::<Issued>()
                .await
                .map_err(|_| ClientError::NotANas),
            StatusCode::BAD_REQUEST => Err(ClientError::InvalidSync),
            _ => Err(refused(&issued)),
        }
    }
    .await;
    let _ = client
        .post(endpoint.url("/auth/logout"))
        .header(header::COOKIE, &cookie)
        .send()
        .await;
    result
}

/// NAS engine identity and port returned during pairing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pairing {
    pub device_id: String,
    pub port: u16,
}

/// A folder of the data disks of the NAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub name: String,
    pub path: String,
}

/// A sync as the NAS lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSync {
    pub id: String,
    pub name: String,
    pub nas_path: String,
    pub pc_path: String,
    pub mode: String,
    pub paused: bool,
    pub revoked: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub last_sync: Option<String>,
}

/// A new sync requested by the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewSync {
    pub name: String,
    pub nas_path: String,
    pub pc_path: String,
    pub mode: String,
}

/// Partial sync update preserving omitted fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
}

/// The sync routes of the NAS, reached with the certificate of this computer.
#[derive(Clone)]
pub struct Gateway {
    client: Client,
    endpoint: Endpoint,
}

/// The error of a sync route, from what the gateway or the sync application answered.
fn failure(response: &Response) -> ClientError {
    let html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    match response.status() {
        StatusCode::FORBIDDEN if html => ClientError::SecurityRequired,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ClientError::NotPaired,
        StatusCode::NOT_FOUND => ClientError::AppMissing,
        StatusCode::CONFLICT => ClientError::FolderTaken,
        StatusCode::BAD_REQUEST | StatusCode::PAYLOAD_TOO_LARGE => ClientError::InvalidSync,
        _ => ClientError::NasUnreachable,
    }
}

impl Gateway {
    pub fn new(
        endpoint: Endpoint,
        authority: &str,
        certificate: &str,
        key: &str,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            client: client(tls::pinned(authority, Some((certificate, key)))?)?,
            endpoint,
        })
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, ClientError> {
        let mut request = self
            .client
            .request(method, self.endpoint.url(&format!("/client/sync/{path}")));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(unreachable)?;
        if !response.status().is_success() {
            return Err(failure(&response));
        }
        if response.status() == StatusCode::NO_CONTENT {
            return serde_json::from_value(serde_json::Value::Null)
                .map_err(|_| ClientError::NotANas);
        }
        response.json().await.map_err(|_| ClientError::NotANas)
    }

    pub async fn pair(&self, device_id: &str, name: &str) -> Result<Pairing, ClientError> {
        self.call(
            Method::POST,
            "pair",
            Some(json!({
                "deviceId": device_id,
                "name": name,
                "version": env!("CARGO_PKG_VERSION"),
            })),
        )
        .await
    }

    pub async fn folders(&self, path: &str) -> Result<Vec<Choice>, ClientError> {
        let encoded: String = path
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || b"-_./~".contains(&byte) {
                    char::from(byte).to_string()
                } else {
                    format!("%{byte:02X}")
                }
            })
            .collect();
        self.call(Method::GET, &format!("folders?path={encoded}"), None)
            .await
    }

    pub async fn syncs(&self) -> Result<Vec<RemoteSync>, ClientError> {
        self.call(Method::GET, "syncs", None).await
    }

    pub async fn create(&self, sync: &NewSync) -> Result<RemoteSync, ClientError> {
        self.call(Method::POST, "syncs", Some(json!(sync))).await
    }

    pub async fn change(&self, id: &str, change: &SyncChange) -> Result<RemoteSync, ClientError> {
        self.call(Method::PUT, &format!("syncs/{id}"), Some(json!(change)))
            .await
    }

    pub async fn remove(&self, id: &str) -> Result<(), ClientError> {
        self.call(Method::DELETE, &format!("syncs/{id}"), None)
            .await
    }
}

#[cfg(test)]
#[path = "../tests/nas/mod.rs"]
mod tests;
