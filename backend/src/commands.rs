//! Commands invoked by the windows. Each command validates its input and fails with an error code.
//! Network and engine work runs outside the window thread.

use crate::{
    client::{computer_name, Client, Snapshot},
    discovery::{self, NasDevice},
    error::ClientError,
    history::Entry,
    nas::{Choice, NewSync, Probe, SyncChange},
    settings::{Appearance, Preferences},
    tray,
};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt as _;

pub struct AppState {
    pub client: Arc<Client>,
    /// Serializes state changes requested by multiple windows.
    operation: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            operation: tokio::sync::Mutex::new(()),
        }
    }
}

type Answer<T> = Result<T, ClientError>;

async fn exclusive<T>(
    state: &AppState,
    work: impl std::future::Future<Output = Answer<T>>,
) -> Answer<T> {
    let _guard = state.operation.try_lock().map_err(|_| ClientError::Busy)?;
    work.await
}

#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Answer<Snapshot> {
    Ok(state.client.snapshot().await)
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>, limit: usize) -> Vec<Entry> {
    state.client.history(limit)
}

#[tauri::command]
pub fn default_name() -> String {
    computer_name()
}

#[tauri::command]
pub async fn discover_nas(target: Option<String>) -> Answer<Vec<NasDevice>> {
    let target = discovery::parse_target(target.as_deref().unwrap_or_default())?;
    tauri::async_runtime::spawn_blocking(move || discovery::discover(target))
        .await
        .map_err(|_| ClientError::NetworkUnavailable)?
}

#[tauri::command]
pub async fn probe_nas(state: State<'_, AppState>, host: String, port: u16) -> Answer<Probe> {
    state.client.probe(&host, port).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn connect_nas(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    fingerprint: String,
    username: String,
    password: String,
    name: String,
) -> Answer<()> {
    exclusive(
        &state,
        state
            .client
            .connect(&host, port, &fingerprint, &username, &password, &name),
    )
    .await
}

#[tauri::command]
pub async fn move_nas(state: State<'_, AppState>, host: String, port: u16) -> Answer<()> {
    exclusive(&state, state.client.move_link(&host, port)).await
}

#[tauri::command]
pub async fn disconnect_nas(state: State<'_, AppState>) -> Answer<()> {
    exclusive(&state, state.client.disconnect()).await
}

#[tauri::command]
pub async fn nas_folders(state: State<'_, AppState>, path: String) -> Answer<Vec<Choice>> {
    state.client.folders(&path).await
}

/// Opens the folder dialog of the system and returns the chosen path, or nothing when cancelled.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Answer<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|folder| folder.into_path().ok())
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| ClientError::Busy)
}

#[tauri::command]
pub async fn create_sync(state: State<'_, AppState>, sync: NewSync) -> Answer<()> {
    exclusive(&state, state.client.create(sync)).await
}

#[tauri::command]
pub async fn update_sync(state: State<'_, AppState>, id: String, change: SyncChange) -> Answer<()> {
    exclusive(&state, state.client.change(&id, change)).await
}

#[tauri::command]
pub async fn remove_sync(state: State<'_, AppState>, id: String) -> Answer<()> {
    exclusive(&state, state.client.remove(&id)).await
}

#[tauri::command]
pub async fn set_paused(state: State<'_, AppState>, paused: bool) -> Answer<()> {
    exclusive(&state, state.client.set_paused(paused)).await
}

#[tauri::command]
pub async fn save_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: Preferences,
) -> Answer<()> {
    exclusive(&state, state.client.save_preferences(preferences)).await?;
    apply_autostart(&app, preferences.autostart);
    Ok(())
}

/// Enables or disables the start of the client with the system, as the preferences require.
pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    if launcher.is_enabled().unwrap_or(false) != enabled {
        let _ = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
    }
}

#[tauri::command]
pub async fn set_appearance(state: State<'_, AppState>, appearance: Appearance) -> Answer<()> {
    state.client.set_appearance(appearance).await
}

#[tauri::command]
pub fn open_main(app: AppHandle) {
    tray::show_main(&app);
}

#[tauri::command]
pub fn hide_mini(app: AppHandle) {
    if let Some(window) = app.get_webview_window(tray::MINI) {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
