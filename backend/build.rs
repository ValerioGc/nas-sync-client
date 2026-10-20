fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_state",
            "get_history",
            "default_name",
            "discover_nas",
            "probe_nas",
            "connect_nas",
            "move_nas",
            "disconnect_nas",
            "nas_folders",
            "pick_folder",
            "create_sync",
            "update_sync",
            "remove_sync",
            "set_paused",
            "save_preferences",
            "set_appearance",
            "open_main",
            "hide_mini",
            "quit_app",
            "minimize_window",
            "toggle_maximize_window",
            "close_window",
            "start_window_dragging",
        ]));
    tauri_build::try_build(attributes).expect("failed to build Tauri permissions");
}
