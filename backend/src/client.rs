//! NAS connection, sync reconciliation, and engine state shared by windows and the tray.

use crate::{
    engine::{folder_kind, Engine, Folder, FolderStatus},
    error::ClientError,
    history::{self, Entry, History},
    nas::{self, Choice, Endpoint, Gateway, NewSync, Probe, RemoteSync, SyncChange},
    secret::Secrets,
    settings::{self, Appearance, Link, Preferences, Settings},
    storage, tls,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Notify};

/// NAS polling interval while sync state remains unchanged.
const REFRESH_EVERY: Duration = Duration::from_secs(30);

/// Maximum pairing wait for the engine to become ready.
const ENGINE_WAIT: Duration = Duration::from_secs(20);

/// Unreachable duration before an offline notification is shown.
pub const UNREACHABLE_NOTICE: Duration = Duration::from_secs(300);

/// Maximum number of history entries returned to a window.
const HISTORY_SHOWN: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Unconfigured,
    Paused,
    Disconnected,
    Syncing,
    Synced,
    Error,
}

/// Window-visible connection metadata excluding certificates and private keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkView {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
}

/// A sync as the windows show it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncView {
    pub id: String,
    pub name: String,
    pub nas_path: String,
    /// The folder of the NAS with the name of its disk.
    pub nas_label: String,
    pub pc_path: String,
    pub mode: String,
    /// `synced`, `syncing`, `paused`, `offline`, `error` or `revoked`.
    pub state: String,
    pub progress: Option<f64>,
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: Status,
    /// Why the client is not in sync with the NAS, when it is not.
    pub problem: Option<ClientError>,
    pub link: Option<LinkView>,
    pub paused: bool,
    pub syncs: Vec<SyncView>,
    pub preferences: Preferences,
    pub appearance: Appearance,
}

#[derive(Default)]
struct Inner {
    settings: Settings,
    gateway: Option<Gateway>,
    syncs: Vec<RemoteSync>,
    volumes: Vec<Choice>,
    /// Most recent NAS request failure.
    problem: Option<ClientError>,
    unreachable_since: Option<Instant>,
    refreshed: Option<Instant>,
    connected: bool,
    local: HashMap<String, FolderStatus>,
    /// Last engine configuration, reapplied after changes or engine restart.
    applied: Option<Vec<Folder>>,
}

pub struct Client {
    directory: PathBuf,
    secrets: Arc<dyn Secrets>,
    pub engine: Option<Engine>,
    inner: Mutex<Inner>,
    history: std::sync::Mutex<History>,
    /// Woken when something asks the state to be read again at once.
    pub changed: Notify,
}

/// The name of this computer, as the system calls it.
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|name| name.trim().chars().take(60).collect::<String>())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Computer".to_owned())
}

/// Whether one folder holds the other, or is the other.
pub fn overlaps(first: &Path, second: &Path) -> bool {
    first.starts_with(second) || second.starts_with(first)
}

/// System directories excluded from sync destinations.
const SYSTEM_FOLDERS: [&str; 14] = [
    "/bin", "/boot", "/dev", "/etc", "/lib", "/lib64", "/opt", "/proc", "/root", "/run", "/sbin",
    "/sys", "/usr", "/var",
];

/// Rejects disk roots, the complete home directory, system paths, and application data
/// directories.
pub fn forbidden(folder: &Path, own: &[&Path]) -> bool {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let windows = [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ]
    .into_iter()
    .filter_map(std::env::var_os);
    folder.parent().is_none()
        || home.is_some_and(|home| folder == Path::new(&home))
        || SYSTEM_FOLDERS
            .iter()
            .any(|system| folder.starts_with(system))
        || windows
            .into_iter()
            .any(|system| folder.starts_with(Path::new(&system)))
        || own.iter().any(|own| overlaps(folder, own))
}

/// Whether a sync of the NAS names the folder chosen for it on this computer.
pub fn chosen_here(chosen: Option<&BTreeMap<String, String>>, sync: &RemoteSync) -> bool {
    chosen.is_none_or(|chosen| chosen.get(&sync.id) == Some(&sync.pc_path))
}

/// The folder of the NAS with the name of its disk in place of its identity.
pub fn nas_label(path: &str, volumes: &[Choice]) -> String {
    let (disk, rest) = path.split_once('/').unwrap_or((path, ""));
    let disk = volumes
        .iter()
        .find(|volume| volume.path == disk)
        .map_or(disk, |volume| volume.name.as_str());
    if rest.is_empty() {
        disk.to_owned()
    } else {
        format!("{disk}/{rest}")
    }
}

/// Combines NAS metadata and local engine state for a sync.
pub fn view_of(
    sync: &RemoteSync,
    local: Option<&FolderStatus>,
    connected: bool,
    paused: bool,
    volumes: &[Choice],
    chosen: bool,
) -> SyncView {
    let busy = |status: &FolderStatus| {
        status.state.starts_with("sync")
            || status.state.starts_with("scan")
            || status.need_bytes > 0
            || status.need_files > 0
    };
    let (state, progress) = if sync.revoked {
        ("revoked", None)
    } else if !chosen {
        ("unchosen", None)
    } else if paused || sync.paused {
        ("paused", None)
    } else if local.is_some_and(|status| !status.error.is_empty() || status.state == "error") {
        ("error", None)
    } else if !connected {
        ("offline", None)
    } else if let Some(status) = local.filter(|status| busy(status)) {
        let progress = (status.global_bytes > 0).then(|| {
            status.global_bytes.saturating_sub(status.need_bytes) as f64 * 100.0
                / status.global_bytes as f64
        });
        ("syncing", progress)
    } else if matches!(sync.state.as_str(), "syncing" | "scanning") {
        ("syncing", sync.progress)
    } else if sync.state == "error" {
        ("error", None)
    } else {
        ("synced", Some(100.0))
    };
    SyncView {
        id: sync.id.clone(),
        name: sync.name.clone(),
        nas_label: nas_label(&sync.nas_path, volumes),
        nas_path: sync.nas_path.clone(),
        pc_path: sync.pc_path.clone(),
        mode: sync.mode.clone(),
        state: state.to_owned(),
        progress,
        last_sync: sync.last_sync.clone(),
    }
}

/// The state of the whole client, and why it is not in sync when it is not.
pub fn overall(
    settings: &Settings,
    engine_running: bool,
    problem: Option<ClientError>,
    connected: bool,
    syncs: &[SyncView],
) -> (Status, Option<ClientError>) {
    if settings.link.is_none() {
        return (Status::Unconfigured, None);
    }
    if settings.paused {
        return (Status::Paused, None);
    }
    if !engine_running {
        return (Status::Disconnected, Some(ClientError::EngineUnavailable));
    }
    if let Some(problem) = problem {
        return (Status::Disconnected, Some(problem));
    }
    let active = syncs
        .iter()
        .any(|sync| !matches!(sync.state.as_str(), "revoked" | "unchosen"));
    if active && !connected {
        return (Status::Disconnected, Some(ClientError::NotConnected));
    }
    if syncs.iter().any(|sync| sync.state == "syncing") {
        return (Status::Syncing, None);
    }
    if syncs.iter().any(|sync| sync.state == "error") {
        return (Status::Error, None);
    }
    (Status::Synced, None)
}

fn history_path(directory: &Path) -> PathBuf {
    directory.join("history.json")
}

impl Client {
    pub fn new(directory: PathBuf, secrets: Arc<dyn Secrets>, engine: Option<Engine>) -> Self {
        let settings = settings::load(&directory).unwrap_or_default();
        let history = storage::read::<Vec<Entry>>(&history_path(&directory))
            .ok()
            .flatten()
            .map(History::from_entries)
            .unwrap_or_default();
        let gateway = settings
            .link
            .as_ref()
            .and_then(|link| gateway_of(link, secrets.as_ref()).ok());
        Self {
            directory,
            secrets,
            engine,
            inner: Mutex::new(Inner {
                settings,
                gateway,
                ..Inner::default()
            }),
            history: std::sync::Mutex::new(history),
            changed: Notify::new(),
        }
    }

    pub async fn settings(&self) -> Settings {
        self.inner.lock().await.settings.clone()
    }

    fn engine(&self) -> Result<&Engine, ClientError> {
        self.engine
            .as_ref()
            .filter(|engine| engine.is_running())
            .ok_or(ClientError::EngineUnavailable)
    }

    /// Waits up to `ENGINE_WAIT` for the engine before pairing.
    async fn ready_engine(&self) -> Result<&Engine, ClientError> {
        let deadline = Instant::now() + ENGINE_WAIT;
        loop {
            match self.engine() {
                Ok(engine) => return Ok(engine),
                Err(error) if self.engine.is_none() || Instant::now() >= deadline => {
                    return Err(error)
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
    }

    /// Runs blocking credential-store operations outside async executor threads.
    async fn secrets<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn Secrets) -> Result<T, ClientError> + Send + 'static,
    ) -> Result<T, ClientError> {
        let secrets = self.secrets.clone();
        tokio::task::spawn_blocking(move || work(secrets.as_ref()))
            .await
            .map_err(|_| ClientError::SecretUnavailable)?
    }

    fn save(&self, settings: &Settings) -> Result<(), ClientError> {
        settings::save(&self.directory, settings)
    }

    /// Current state exposed to application windows.
    pub async fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock().await;
        let paused = inner.settings.paused;
        let syncs: Vec<SyncView> = inner
            .syncs
            .iter()
            .map(|sync| {
                view_of(
                    sync,
                    inner.local.get(&sync.id),
                    inner.connected,
                    paused,
                    &inner.volumes,
                    chosen_here(inner.settings.chosen.as_ref(), sync),
                )
            })
            .collect();
        let running = self.engine.as_ref().is_some_and(Engine::is_running);
        let (status, problem) = overall(
            &inner.settings,
            running,
            inner.problem,
            inner.connected,
            &syncs,
        );
        Snapshot {
            status,
            problem,
            link: inner.settings.link.as_ref().map(|link| LinkView {
                name: link.name.clone(),
                host: link.host.clone(),
                port: link.port,
                fingerprint: tls::authority_fingerprint(&link.authority).unwrap_or_default(),
            }),
            paused,
            syncs,
            preferences: inner.settings.preferences(),
            appearance: inner.settings.appearance.clone(),
        }
    }

    /// Elapsed time since the NAS became unreachable.
    pub async fn unreachable_for(&self) -> Option<Duration> {
        self.inner
            .lock()
            .await
            .unreachable_since
            .map(|since| since.elapsed())
    }

    pub fn history(&self, count: usize) -> Vec<Entry> {
        self.history
            .lock()
            .map(|history| history.latest(count.min(HISTORY_SHOWN)))
            .unwrap_or_default()
    }

    /// Records the events of the engine that are about files, and keeps the history on disk.
    pub fn record(&self, entries: Vec<Entry>) -> Vec<Entry> {
        if entries.is_empty() {
            return entries;
        }
        if let Ok(mut history) = self.history.lock() {
            for entry in &entries {
                history.push(entry.clone());
            }
            let _ = storage::write(&history_path(&self.directory), &history.all());
        }
        entries
    }

    /// Fetches the NAS authority fingerprint for initial trust confirmation.
    pub async fn probe(&self, host: &str, port: u16) -> Result<Probe, ClientError> {
        nas::probe(&Endpoint::new(host, port)?).await
    }

    /// Pairs this computer with the NAS at `host` once its authority fingerprint is confirmed.
    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        fingerprint: &str,
        username: &str,
        password: &str,
        name: &str,
    ) -> Result<(), ClientError> {
        let endpoint = Endpoint::new(host, port)?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
            return Err(ClientError::InvalidSync);
        }
        let probe = nas::probe(&endpoint).await?;
        if probe.fingerprint != fingerprint {
            return Err(ClientError::TrustChanged);
        }
        let engine = self.ready_engine().await?;
        let device_id = engine
            .device_id()
            .await
            .map_err(|_| ClientError::EngineUnavailable)?;
        // The protected storage has to work before the NAS issues a certificate it would keep.
        self.secrets(|secrets| secrets.load().map(drop)).await?;
        let issued = nas::enroll(
            &endpoint,
            &probe.authority,
            username,
            password,
            name,
            &device_id,
        )
        .await?;
        let gateway = Gateway::new(
            endpoint.clone(),
            &issued.authority,
            &issued.certificate,
            &issued.key,
        )?;
        let key = issued.key.clone();
        self.secrets(move |secrets| secrets.save(&key)).await?;
        let pairing = gateway.pair(&device_id, name).await?;
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        settings.link = Some(Link {
            name: name.to_owned(),
            host: endpoint.host,
            port: endpoint.port,
            authority: issued.authority,
            certificate: issued.certificate,
            nas_id: pairing.device_id,
            sync_port: pairing.port,
        });
        settings.paused = false;
        settings.chosen = Some(BTreeMap::new());
        self.save(&settings)?;
        inner.settings = settings;
        inner.gateway = Some(gateway);
        inner.problem = None;
        inner.applied = None;
        inner.refreshed = None;
        drop(inner);
        self.changed.notify_one();
        Ok(())
    }

    /// Changes the NAS address while retaining trust in the existing certificate authority.
    pub async fn move_link(&self, host: &str, port: u16) -> Result<(), ClientError> {
        let endpoint = Endpoint::new(host, port)?;
        let probe = nas::probe(&endpoint).await?;
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        let link = settings.link.as_mut().ok_or(ClientError::NotPaired)?;
        if tls::authority_fingerprint(&link.authority).as_deref() != Some(&probe.fingerprint) {
            return Err(ClientError::TrustChanged);
        }
        link.host = endpoint.host;
        link.port = endpoint.port;
        let key = self
            .secrets(|secrets| secrets.load())
            .await?
            .ok_or(ClientError::NotPaired)?;
        let gateway = gateway_with(link, &key)?;
        self.save(&settings)?;
        inner.settings = settings;
        inner.gateway = Some(gateway);
        inner.problem = None;
        inner.applied = None;
        inner.refreshed = None;
        drop(inner);
        self.changed.notify_one();
        Ok(())
    }

    /// Unpairs this computer and removes its NAS sync rules while preserving files on both
    /// devices.
    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let mut inner = self.inner.lock().await;
        if let Some(gateway) = inner.gateway.take() {
            for sync in &inner.syncs {
                let _ = gateway.remove(&sync.id).await;
            }
        }
        let mut settings = inner.settings.clone();
        settings.link = None;
        settings.paused = false;
        settings.chosen = Some(BTreeMap::new());
        self.save(&settings)?;
        self.secrets(|secrets| secrets.forget()).await?;
        inner.settings = settings;
        inner.syncs.clear();
        inner.volumes.clear();
        inner.local.clear();
        inner.problem = None;
        inner.unreachable_since = None;
        inner.applied = None;
        if let Ok(mut history) = self.history.lock() {
            history.clear();
            let _ = storage::write(&history_path(&self.directory), &history.all());
        }
        drop(inner);
        let _ = self.reconcile().await;
        self.changed.notify_one();
        Ok(())
    }

    fn gateway(inner: &Inner) -> Result<Gateway, ClientError> {
        inner.gateway.clone().ok_or(ClientError::NotPaired)
    }

    /// The folders of the data disks of the NAS under `path`, the disks at its root.
    pub async fn folders(&self, path: &str) -> Result<Vec<Choice>, ClientError> {
        let gateway = Self::gateway(&*self.inner.lock().await)?;
        let choices = gateway.folders(path).await?;
        if path.trim_matches('/').is_empty() {
            self.inner.lock().await.volumes = choices.clone();
        }
        Ok(choices)
    }

    /// Application settings and engine-home directories excluded from sync destinations.
    fn own_folders(&self) -> Vec<&Path> {
        let mut own = vec![self.directory.as_path()];
        if let Some(engine) = &self.engine {
            own.push(engine.home());
        }
        own
    }

    /// Notes the folder of the computer chosen here for a sync, or forgets the sync.
    async fn choose(&self, id: &str, pc_path: Option<&str>) -> Result<(), ClientError> {
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        let chosen = settings.chosen.get_or_insert_with(BTreeMap::new);
        match pc_path {
            Some(path) => chosen.insert(id.to_owned(), path.to_owned()),
            None => chosen.remove(id),
        };
        self.save(&settings)?;
        inner.settings = settings;
        inner.applied = None;
        Ok(())
    }

    async fn check_local(
        &self,
        path: &str,
        mode: &str,
        except: Option<&str>,
    ) -> Result<(), ClientError> {
        let folder = PathBuf::from(path.trim());
        if !folder.is_absolute()
            || (mode == "pc-to-nas" && !folder.is_dir())
            || forbidden(&folder, &self.own_folders())
        {
            return Err(ClientError::InvalidSync);
        }
        let inner = self.inner.lock().await;
        if inner
            .syncs
            .iter()
            .filter(|sync| !sync.revoked && Some(sync.id.as_str()) != except)
            .any(|sync| overlaps(&folder, Path::new(&sync.pc_path)))
        {
            return Err(ClientError::FolderTaken);
        }
        Ok(())
    }

    pub async fn create(&self, sync: NewSync) -> Result<(), ClientError> {
        self.check_local(&sync.pc_path, &sync.mode, None).await?;
        let gateway = Self::gateway(&*self.inner.lock().await)?;
        let created = gateway.create(&sync).await?;
        self.choose(&created.id, Some(&created.pc_path)).await?;
        self.inner.lock().await.syncs.push(created);
        self.after_change().await
    }

    pub async fn change(&self, id: &str, change: SyncChange) -> Result<(), ClientError> {
        let mode = {
            let inner = self.inner.lock().await;
            let known = inner
                .syncs
                .iter()
                .find(|sync| sync.id == id)
                .ok_or(ClientError::InvalidSync)?;
            change.mode.clone().unwrap_or_else(|| known.mode.clone())
        };
        if let Some(path) = &change.pc_path {
            self.check_local(path, &mode, Some(id)).await?;
        }
        let gateway = Self::gateway(&*self.inner.lock().await)?;
        let changed = gateway.change(id, &change).await?;
        if change.pc_path.is_some() {
            self.choose(id, Some(&changed.pc_path)).await?;
        }
        let mut inner = self.inner.lock().await;
        if let Some(sync) = inner.syncs.iter_mut().find(|sync| sync.id == id) {
            *sync = changed;
        }
        drop(inner);
        self.after_change().await
    }

    pub async fn remove(&self, id: &str) -> Result<(), ClientError> {
        let gateway = Self::gateway(&*self.inner.lock().await)?;
        match gateway.remove(id).await {
            Ok(()) | Err(ClientError::AppMissing) => {}
            Err(error) => return Err(error),
        }
        self.choose(id, None).await?;
        self.inner.lock().await.syncs.retain(|sync| sync.id != id);
        self.after_change().await
    }

    /// Stops or starts every sync of this computer.
    pub async fn set_paused(&self, paused: bool) -> Result<(), ClientError> {
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        settings.paused = paused;
        self.save(&settings)?;
        inner.settings = settings;
        inner.applied = None;
        drop(inner);
        self.after_change().await
    }

    pub async fn save_preferences(&self, preferences: Preferences) -> Result<(), ClientError> {
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        settings.apply(preferences)?;
        self.save(&settings)?;
        inner.settings = settings;
        inner.applied = None;
        drop(inner);
        self.after_change().await
    }

    pub async fn set_appearance(&self, appearance: Appearance) -> Result<(), ClientError> {
        if !appearance.valid() {
            return Err(ClientError::InvalidSync);
        }
        let mut inner = self.inner.lock().await;
        let mut settings = inner.settings.clone();
        settings.appearance = appearance;
        self.save(&settings)?;
        inner.settings = settings;
        drop(inner);
        self.changed.notify_one();
        Ok(())
    }

    async fn after_change(&self) -> Result<(), ClientError> {
        let _ = self.reconcile().await;
        self.changed.notify_one();
        Ok(())
    }

    /// Invalidates cached engine configuration after restart.
    pub async fn engine_started(&self) {
        self.inner.lock().await.applied = None;
        let _ = self.reconcile().await;
    }

    /// Reconciles engine preferences, devices, and sync folders, removing obsolete entries.
    pub async fn reconcile(&self) -> Result<(), String> {
        let engine = match self.engine.as_ref().filter(|engine| engine.is_running()) {
            Some(engine) => engine,
            None => return Ok(()),
        };
        let (settings, desired, applied) = {
            let inner = self.inner.lock().await;
            let desired: Vec<Folder> = match &inner.settings.link {
                Some(link) => inner
                    .syncs
                    .iter()
                    .filter(|sync| !sync.revoked)
                    // Only a folder chosen on this computer, never one the NAS names by itself.
                    .filter(|sync| chosen_here(inner.settings.chosen.as_ref(), sync))
                    .filter(|sync| !forbidden(Path::new(&sync.pc_path), &self.own_folders()))
                    .map(|sync| Folder {
                        id: sync.id.clone(),
                        label: sync.name.clone(),
                        path: PathBuf::from(&sync.pc_path),
                        kind: folder_kind(&sync.mode),
                        device: link.nas_id.clone(),
                        paused: sync.paused,
                    })
                    .collect(),
                None => Vec::new(),
            };
            (inner.settings.clone(), desired, inner.applied.clone())
        };
        if applied.as_ref() == Some(&desired) {
            return Ok(());
        }
        engine
            .configure(settings.max_send_kbps, settings.max_recv_kbps)
            .await?;
        let nas_id = settings.link.as_ref().map(|link| link.nas_id.clone());
        if let Some(link) = &settings.link {
            let endpoint = Endpoint::new(&link.host, link.port).map_err(|_| "invalid address")?;
            engine
                .set_nas(
                    &link.nas_id,
                    &endpoint.sync_address(link.sync_port),
                    settings.paused,
                )
                .await?;
        }
        for id in engine.folders().await? {
            if !desired.iter().any(|folder| folder.id == id) {
                engine.remove_folder(&id).await?;
            }
        }
        let own = engine.device_id().await?;
        for device in engine.devices().await? {
            if device != own && Some(&device) != nas_id.as_ref() {
                engine.remove_device(&device).await?;
            }
        }
        for folder in &desired {
            engine.put_folder(folder).await?;
        }
        let folders: Vec<String> = desired.iter().map(|folder| folder.id.clone()).collect();
        if let Ok(mut history) = self.history.lock() {
            history.retain_folders(&folders);
        }
        self.inner.lock().await.applied = Some(desired);
        Ok(())
    }

    /// Asks the NAS for the syncs when it is time, then the engine for their state.
    pub async fn tick(&self) {
        let due = {
            let inner = self.inner.lock().await;
            inner.settings.link.is_some()
                && inner
                    .refreshed
                    .is_none_or(|refreshed| refreshed.elapsed() >= REFRESH_EVERY)
        };
        if due {
            self.refresh().await;
        }
        self.read_engine().await;
    }

    /// Reads the syncs from the NAS now.
    pub async fn refresh(&self) {
        let Ok(gateway) = Self::gateway(&*self.inner.lock().await) else {
            return;
        };
        let answer = gateway.syncs().await;
        let volumes = if answer.is_ok() && self.inner.lock().await.volumes.is_empty() {
            gateway.folders("").await.ok()
        } else {
            None
        };
        let mut inner = self.inner.lock().await;
        inner.refreshed = Some(Instant::now());
        match answer {
            Ok(syncs) => {
                // Preserves folder choices from syncs created before explicit local
                // confirmation was introduced.
                if inner.settings.chosen.is_none() {
                    let mut settings = inner.settings.clone();
                    settings.chosen = Some(
                        syncs
                            .iter()
                            .map(|sync| (sync.id.clone(), sync.pc_path.clone()))
                            .collect(),
                    );
                    if self.save(&settings).is_ok() {
                        inner.settings = settings;
                    }
                }
                inner.syncs = syncs;
                inner.problem = None;
                inner.unreachable_since = None;
                if let Some(volumes) = volumes {
                    inner.volumes = volumes;
                }
            }
            Err(problem) => {
                inner.problem = Some(problem);
                if problem == ClientError::NasUnreachable {
                    inner.unreachable_since.get_or_insert_with(Instant::now);
                }
            }
        }
        drop(inner);
        let _ = self.reconcile().await;
    }

    async fn read_engine(&self) {
        let Some(engine) = self.engine.as_ref().filter(|engine| engine.is_running()) else {
            return;
        };
        let (nas_id, ids) = {
            let inner = self.inner.lock().await;
            (
                inner.settings.link.as_ref().map(|link| link.nas_id.clone()),
                inner
                    .syncs
                    .iter()
                    .filter(|sync| !sync.revoked)
                    .map(|sync| sync.id.clone())
                    .collect::<Vec<_>>(),
            )
        };
        let connected = match &nas_id {
            Some(id) => engine.connected(id).await,
            None => false,
        };
        let mut local = HashMap::new();
        for id in ids {
            if let Ok(status) = engine.folder_status(&id).await {
                local.insert(id, status);
            }
        }
        let mut inner = self.inner.lock().await;
        inner.connected = connected;
        inner.local = local;
    }

    /// The entries of the history in the events of the engine after `since`.
    pub async fn follow_events(&self, since: u64) -> Result<(u64, Vec<Entry>), String> {
        let engine = self
            .engine
            .as_ref()
            .filter(|engine| engine.is_running())
            .ok_or("the engine is not running")?;
        let events = engine.events(since).await?;
        let last = events
            .iter()
            .filter_map(|event| event["id"].as_u64())
            .max()
            .unwrap_or(since);
        let entries = events.iter().filter_map(history::entry_of).collect();
        Ok((last, entries))
    }
}

fn gateway_of(link: &Link, secrets: &dyn Secrets) -> Result<Gateway, ClientError> {
    gateway_with(link, &secrets.load()?.ok_or(ClientError::NotPaired)?)
}

fn gateway_with(link: &Link, key: &str) -> Result<Gateway, ClientError> {
    Gateway::new(
        Endpoint::new(&link.host, link.port)?,
        &link.authority,
        &link.certificate,
        key,
    )
}

#[cfg(test)]
#[path = "../tests/client/mod.rs"]
mod tests;
