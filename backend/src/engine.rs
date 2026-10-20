//! Supervises the bundled Syncthing process and its authenticated loopback API.
//! The API key is stored locally and passed through the environment.
//! Global discovery, relays, NAT traversal, telemetry, and automatic upgrades are disabled.
//! Before starting a replacement, the client requests shutdown through the saved API port.
//! Engine output is written to `engine.log` for each run.

use crate::tls::sha256_hex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    process::{Child, Command},
    sync::Notify,
};

const TIMEOUT: Duration = Duration::from_secs(30);
const RESTART: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(500);

pub const EVENTS_WAIT: u64 = 20;
pub const VERSIONS_DAYS: u32 = 30;
const LEFT_RUNNING_WAIT: Duration = Duration::from_secs(10);

/// A folder of the sync on the computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub kind: &'static str,
    pub device: String,
    pub paused: bool,
}

/// Folder progress reported by the engine.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatus {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub global_bytes: u64,
    #[serde(default)]
    pub need_bytes: u64,
    #[serde(default)]
    pub need_files: u64,
    #[serde(default)]
    pub error: String,
}

/// Local folder direction corresponding to the opposite NAS sync direction.
pub fn folder_kind(mode: &str) -> &'static str {
    match mode {
        "pc-to-nas" => "sendonly",
        "nas-to-pc" => "receiveonly",
        _ => "sendreceive",
    }
}

/// The program next to the application, or `NAS_SYNC_ENGINE` in development.
pub fn program() -> PathBuf {
    if let Some(path) = std::env::var_os("NAS_SYNC_ENGINE").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    let name = if cfg!(windows) {
        "syncthing.exe"
    } else {
        "syncthing"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|directory| directory.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// A port free on the loopback right now.
fn free_port() -> Option<u16> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .ok()
}

/// A random key of 32 hexadecimal characters.
pub fn random_key() -> Option<String> {
    let mut bytes = [0u8; 32];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes).ok()?;
    Some(sha256_hex(&bytes)[..32].to_owned())
}

fn restrict(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// The key of the API, made once and kept in the home of the engine.
pub fn key_in(home: &Path) -> Option<String> {
    fs::create_dir_all(home).ok()?;
    restrict(home);
    let file = home.join("api-key");
    if let Ok(key) = fs::read_to_string(&file) {
        let key = key.trim().to_owned();
        if key.len() >= 32 && key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Some(key);
        }
    }
    let key = random_key()?;
    fs::write(&file, &key).ok()?;
    restrict(&file);
    Some(key)
}

/// The first configuration, written before the first start, so the engine is local-only before
/// its API even answers.
pub fn first_config(listen_port: u16) -> String {
    format!(
        r#"<configuration version="37">
<gui enabled="true" tls="false"><address>127.0.0.1:0</address></gui>
<options>
<listenAddress>tcp://127.0.0.1:{listen_port}</listenAddress>
<globalAnnounceEnabled>false</globalAnnounceEnabled><localAnnounceEnabled>false</localAnnounceEnabled>
<relaysEnabled>false</relaysEnabled><natEnabled>false</natEnabled>
<urAccepted>-1</urAccepted><crashReportingEnabled>false</crashReportingEnabled>
<autoUpgradeIntervalH>0</autoUpgradeIntervalH><startBrowser>false</startBrowser>
<limitBandwidthInLan>true</limitBandwidthInLan>
</options></configuration>
"#
    )
}

#[derive(Clone)]
pub struct Engine {
    program: PathBuf,
    home: PathBuf,
    key: String,
    port: Arc<AtomicU16>,
    http: reqwest::Client,
    running: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
    /// The running process, killed at once when the application closes.
    child: Arc<Mutex<Option<Child>>>,
    /// Signals successful engine startup so configuration can be reapplied.
    pub started: Arc<Notify>,
}

impl Engine {
    pub fn new(program: PathBuf, home: PathBuf) -> Option<Self> {
        let key = key_in(&home)?;
        let config = home.join("config.xml");
        if !config.exists() {
            fs::write(&config, first_config(free_port()?)).ok()?;
            restrict(&config);
        }
        Some(Self {
            program,
            home,
            key,
            port: Arc::default(),
            http: reqwest::Client::builder().timeout(TIMEOUT).build().ok()?,
            running: Arc::default(),
            stopping: Arc::default(),
            child: Arc::default(),
            started: Arc::default(),
        })
    }

    /// Dedicated engine-home directory excluded from sync folders.
    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn is_installed(&self) -> bool {
        self.program.is_file()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Captures engine output in engine.log when the file can be opened.
    fn output(&self) -> (Stdio, Stdio) {
        let log = self.home.join("engine.log");
        match fs::File::create(&log).and_then(|file| Ok((file.try_clone()?, file))) {
            Ok((out, err)) => {
                restrict(&log);
                (Stdio::from(out), Stdio::from(err))
            }
            Err(_) => (Stdio::null(), Stdio::null()),
        }
    }

    /// Shuts down an engine left running by a killed application, since it still locks the home.
    async fn stop_left_running(&self) {
        let file = self.home.join("api-port");
        let Some(port) = fs::read_to_string(&file)
            .ok()
            .and_then(|text| text.trim().parse::<u16>().ok())
        else {
            return;
        };
        let _ = fs::remove_file(&file);
        let at = |path: &str| format!("http://127.0.0.1:{port}{path}");
        let shut = self
            .http
            .post(at("/rest/system/shutdown"))
            .header("X-API-Key", &self.key)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success());
        if !shut {
            return;
        }
        let deadline = tokio::time::Instant::now() + LEFT_RUNNING_WAIT;
        while tokio::time::Instant::now() < deadline {
            let answering = self
                .http
                .get(at("/rest/system/ping"))
                .header("X-API-Key", &self.key)
                .send()
                .await
                .is_ok_and(|response| response.status().is_success());
            if !answering {
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    }

    fn command(&self, port: u16) -> Command {
        let mut command = Command::new(&self.program);
        let (out, err) = self.output();
        command
            .arg("serve")
            .arg(format!("--home={}", self.home.display()))
            .arg("--no-browser")
            .arg("--no-restart")
            .arg(format!("--gui-address=http://127.0.0.1:{port}"))
            .env("STGUIAPIKEY", &self.key)
            .env("STNODEFAULTFOLDER", "1")
            .env("STNOUPGRADE", "1")
            // Keeps a single supervised process without child restarts managed by Syncthing.
            .env("STMONITORED", "1")
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    }

    /// Whether the process is gone, by itself or killed.
    fn exited(&self) -> bool {
        self.child
            .lock()
            .map(|mut child| {
                child
                    .as_mut()
                    .is_none_or(|child| !matches!(child.try_wait(), Ok(None)))
            })
            .unwrap_or(true)
    }

    async fn wait_exit(&self) {
        while !self.exited() {
            tokio::time::sleep(POLL).await;
        }
    }

    /// Runs the engine until `stop`, starting it again when it stops by itself.
    pub async fn supervise(self) {
        while !self.stopping.load(Ordering::Relaxed) {
            let Some(port) = free_port().filter(|_| self.is_installed()) else {
                tokio::time::sleep(RESTART * 6).await;
                continue;
            };
            self.stop_left_running().await;
            self.port.store(port, Ordering::Relaxed);
            if let (Ok(child), Ok(mut slot)) = (self.command(port).spawn(), self.child.lock()) {
                *slot = Some(child);
            }
            let ready = tokio::select! {
                ready = self.wait_ready() => ready,
                () = self.wait_exit() => false,
            };
            if ready && !self.stopping.load(Ordering::Relaxed) {
                let file = self.home.join("api-port");
                if fs::write(&file, port.to_string()).is_ok() {
                    restrict(&file);
                }
                self.running.store(true, Ordering::Relaxed);
                self.started.notify_one();
                self.wait_exit().await;
            }
            self.running.store(false, Ordering::Relaxed);
            self.kill();
            if !self.stopping.load(Ordering::Relaxed) {
                tokio::time::sleep(RESTART).await;
            }
        }
    }

    fn kill(&self) {
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.start_kill();
                let _ = child.try_wait();
            }
        }
    }

    /// Stops the engine immediately during application shutdown.
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.running.store(false, Ordering::Relaxed);
        self.kill();
        let _ = fs::remove_file(self.home.join("api-port"));
    }

    async fn wait_ready(&self) -> bool {
        for _ in 0..120 {
            if self.get("/rest/system/ping").await.is_ok() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        false
    }

    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, String> {
        let port = self.port.load(Ordering::Relaxed);
        let mut request = self
            .http
            .request(method, format!("http://127.0.0.1:{port}{path}"))
            .header("X-API-Key", &self.key);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("the engine answered {status}: {}", text.trim()));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|error| error.to_string())
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        self.call(reqwest::Method::GET, path, None).await
    }

    /// The identity of this computer in the sync.
    pub async fn device_id(&self) -> Result<String, String> {
        self.get("/rest/system/status")
            .await?
            .get("myID")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| "the engine did not say its identity".to_owned())
    }

    /// Applies the local-only network options and the bandwidth limits of the settings.
    pub async fn configure(&self, send_kbps: u32, receive_kbps: u32) -> Result<(), String> {
        let options = json!({
            "globalAnnounceEnabled": false,
            "localAnnounceEnabled": false,
            "relaysEnabled": false,
            "natEnabled": false,
            "urAccepted": -1,
            "crashReportingEnabled": false,
            "autoUpgradeIntervalH": 0,
            "startBrowser": false,
            "maxSendKbps": send_kbps,
            "maxRecvKbps": receive_kbps,
            "limitBandwidthInLan": true,
        });
        self.call(
            reqwest::Method::PATCH,
            "/rest/config/options",
            Some(&options),
        )
        .await
        .map(drop)
    }

    /// Registers the NAS at its address only. A paused device exchanges no data.
    pub async fn set_nas(&self, id: &str, address: &str, paused: bool) -> Result<(), String> {
        let device = json!({
            "deviceID": id,
            "name": "NAS",
            "addresses": [address],
            "compression": "metadata",
            "autoAcceptFolders": false,
            "introducer": false,
            "paused": paused,
        });
        self.call(
            reqwest::Method::PUT,
            &format!("/rest/config/devices/{id}"),
            Some(&device),
        )
        .await
        .map(drop)
    }

    pub async fn remove_device(&self, id: &str) -> Result<(), String> {
        self.call(
            reqwest::Method::DELETE,
            &format!("/rest/config/devices/{id}"),
            None,
        )
        .await
        .map(drop)
    }

    /// The devices the engine knows, by identity.
    pub async fn devices(&self) -> Result<Vec<String>, String> {
        Ok(ids(&self.get("/rest/config/devices").await?, "deviceID"))
    }

    pub async fn put_folder(&self, folder: &Folder) -> Result<(), String> {
        if folder.kind != "sendonly" {
            fs::create_dir_all(&folder.path).map_err(|error| error.to_string())?;
        }
        let body = json!({
            "id": folder.id,
            "label": folder.label,
            "path": folder.path,
            "type": folder.kind,
            "devices": [{ "deviceID": folder.device }],
            "rescanIntervalS": 3600,
            "fsWatcherEnabled": true,
            "paused": folder.paused,
            // Retains files replaced or deleted by the NAS for 30 days in `.stversions`.
            "versioning": {
                "type": "trashcan",
                "params": { "cleanoutDays": VERSIONS_DAYS.to_string() },
            },
        });
        self.call(
            reqwest::Method::PUT,
            &format!("/rest/config/folders/{}", folder.id),
            Some(&body),
        )
        .await
        .map(drop)
    }

    pub async fn remove_folder(&self, id: &str) -> Result<(), String> {
        self.call(
            reqwest::Method::DELETE,
            &format!("/rest/config/folders/{id}"),
            None,
        )
        .await
        .map(drop)
    }

    /// The folders the engine knows, by id.
    pub async fn folders(&self) -> Result<Vec<String>, String> {
        Ok(ids(&self.get("/rest/config/folders").await?, "id"))
    }

    pub async fn folder_status(&self, id: &str) -> Result<FolderStatus, String> {
        serde_json::from_value(self.get(&format!("/rest/db/status?folder={id}")).await?)
            .map_err(|error| error.to_string())
    }

    /// Whether the engine is connected to the NAS now.
    pub async fn connected(&self, id: &str) -> bool {
        self.get("/rest/system/connections")
            .await
            .is_ok_and(|connections| connections["connections"][id]["connected"] == true)
    }

    /// The events after `since`, waiting for one at most `EVENTS_WAIT` seconds.
    pub async fn events(&self, since: u64) -> Result<Vec<Value>, String> {
        let events = self
            .get(&format!(
                "/rest/events?events=LocalChangeDetected,RemoteChangeDetected,ItemFinished\
                 &since={since}&timeout={EVENTS_WAIT}"
            ))
            .await?;
        Ok(events.as_array().cloned().unwrap_or_default())
    }
}

#[cfg(test)]
impl Engine {
    /// An engine already answering on `port`, as after a start.
    pub fn answering(home: PathBuf, port: u16) -> Self {
        let engine = Self::new(PathBuf::from("missing-engine"), home).expect("an engine");
        engine.port.store(port, Ordering::Relaxed);
        engine.running.store(true, Ordering::Relaxed);
        engine
    }
}

fn ids(list: &Value, field: &str) -> Vec<String> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item[field].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../tests/engine/mod.rs"]
mod tests;
