mod client;
mod commands;
mod discovery;
mod engine;
mod error;
mod history;
mod i18n;
mod nas;
mod notices;
mod secret;
mod settings;
mod storage;
mod system_command;
mod titlebar;
mod tls;
mod tray;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

use client::{Client, UNREACHABLE_NOTICE};
use commands::AppState;
use std::{sync::Arc, time::Duration};
use tauri::{App, AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt as _;

const TICK: Duration = Duration::from_secs(5);
const TICK_HIDDEN: Duration = Duration::from_secs(20);
const STATE_EVENT: &str = "client-state";

fn notify(app: &AppHandle, notice: notices::Notice) {
    let _ = app
        .notification()
        .builder()
        .title(notice.title)
        .body(notice.body)
        .show();
}

/// Reads the state, updates the tray and notifies when the NAS stays unreachable.
async fn watch(app: AppHandle, client: Arc<Client>) {
    let mut told_unreachable = false;
    loop {
        client.tick().await;
        let snapshot = client.snapshot().await;
        let settings = client.settings().await;
        if let Some(tray) = app.try_state::<tray::Tray<tauri::Wry>>() {
            tray.update(&snapshot, &settings.appearance.locale);
        }
        let _ = app.emit(STATE_EVENT, ());
        let unreachable = client
            .unreachable_for()
            .await
            .is_some_and(|elapsed| elapsed >= UNREACHABLE_NOTICE);
        if unreachable && !told_unreachable && settings.notifications {
            notify(&app, notices::unreachable(&settings.appearance.locale));
        }
        told_unreachable = unreachable;
        let shown = [tray::MAIN, tray::MINI].into_iter().any(|label| {
            app.get_webview_window(label)
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false)
        });
        tokio::select! {
            () = tokio::time::sleep(if shown { TICK } else { TICK_HIDDEN }) => {},
            () = client.changed.notified() => {},
        }
    }
}

/// Follows the events of the engine into the history.
async fn follow(app: AppHandle, client: Arc<Client>) {
    let mut since = 0;
    loop {
        match client.follow_events(since).await {
            Ok((last, entries)) => {
                since = last;
                let entries = client.record(entries);
                let settings = client.settings().await;
                if settings.notifications {
                    if let Some(notice) = notices::of_entries(&entries, &settings.appearance.locale)
                    {
                        notify(&app, notice);
                    }
                }
            }
            Err(_) => {
                since = 0;
                tokio::time::sleep(TICK).await;
            }
        }
    }
}

/// Gives the engine its configuration at every start.
async fn configure(client: Arc<Client>) {
    let Some(engine) = client.engine.clone() else {
        return;
    };
    loop {
        engine.started.notified().await;
        client.engine_started().await;
        client.changed.notify_one();
    }
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let config = app.path().app_config_dir()?;
    let data = app.path().app_data_dir()?;
    let engine = engine::Engine::new(engine::program(), data.join("engine"));
    let client = Arc::new(Client::new(
        config,
        Arc::new(secret::Keyring),
        engine.clone(),
    ));
    app.manage(AppState::new(client.clone()));
    let tray = tray::create(app.handle(), |app| {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let client = state.client.clone();
        tauri::async_runtime::spawn(async move {
            let paused = client.settings().await.paused;
            let _ = client.set_paused(!paused).await;
        });
    })?;
    app.manage(tray);

    let handle = app.handle().clone();
    tauri::async_runtime::spawn({
        let client = client.clone();
        async move {
            commands::apply_autostart(&handle, client.settings().await.autostart);
        }
    });
    if let Some(engine) = engine {
        tauri::async_runtime::spawn(engine.supervise());
    }
    tauri::async_runtime::spawn(configure(client.clone()));
    tauri::async_runtime::spawn(watch(app.handle().clone(), client.clone()));
    tauri::async_runtime::spawn(follow(app.handle().clone(), client));

    if !std::env::args().any(|argument| argument == tray::MINIMIZED) {
        tray::show_main(app.handle());
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![tray::MINIMIZED]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(setup)
        .on_window_event(tray::window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_history,
            commands::default_name,
            commands::discover_nas,
            commands::probe_nas,
            commands::connect_nas,
            commands::move_nas,
            commands::disconnect_nas,
            commands::nas_folders,
            commands::pick_folder,
            commands::create_sync,
            commands::update_sync,
            commands::remove_sync,
            commands::set_paused,
            commands::save_preferences,
            commands::set_appearance,
            commands::open_main,
            commands::hide_mini,
            commands::quit_app,
            titlebar::minimize_window,
            titlebar::toggle_maximize_window,
            titlebar::close_window,
            titlebar::start_window_dragging,
        ])
        .build(tauri::generate_context!())
        .expect("error while building NAS Sync")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(engine) = app
                    .try_state::<AppState>()
                    .and_then(|state| state.client.engine.clone())
                {
                    engine.stop();
                }
            }
        });
}
