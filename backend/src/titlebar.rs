use serde::Serialize;
use tauri::WebviewWindow;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowError {
    Unavailable,
}

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) -> Result<(), WindowError> {
    window.minimize().map_err(|_| WindowError::Unavailable)
}

#[tauri::command]
pub fn toggle_maximize_window(window: WebviewWindow) -> Result<bool, WindowError> {
    let is_maximized = window
        .is_maximized()
        .map_err(|_| WindowError::Unavailable)?;
    let result = if is_maximized {
        window.unmaximize()
    } else {
        window.maximize()
    };
    result.map_err(|_| WindowError::Unavailable)?;

    Ok(!is_maximized)
}

/// Hides the main window while sync continues through the tray.
#[tauri::command]
pub fn close_window(window: WebviewWindow) -> Result<(), WindowError> {
    window.hide().map_err(|_| WindowError::Unavailable)
}

#[tauri::command]
pub fn start_window_dragging(window: WebviewWindow) -> Result<(), WindowError> {
    window
        .start_dragging()
        .map_err(|_| WindowError::Unavailable)
}
