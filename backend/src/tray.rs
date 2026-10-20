//! System-tray state, controls, and summary window placement.
//! Windows opens the summary on a click and the main window on a double click.
//! Linux exposes the summary through the tray menu.

use crate::{
    client::{Snapshot, Status},
    i18n::tray_labels as labels,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, PhysicalPosition, Rect, Runtime, WebviewWindow, Window, WindowEvent,
};

pub const MAIN: &str = "main";
pub const MINI: &str = "mini";

pub const MINIMIZED: &str = "--minimized";
const REOPEN_AFTER_MS: u64 = 400;
const MARGIN: f64 = 8.0;

/// State-specific tray icon with a colored status indicator.
fn icon(status: Status) -> Option<Image<'static>> {
    let bytes: &[u8] = match status {
        Status::Synced => include_bytes!("../icons/tray/synced.png"),
        Status::Syncing => include_bytes!("../icons/tray/syncing.png"),
        Status::Paused | Status::Unconfigured => include_bytes!("../icons/tray/paused.png"),
        Status::Disconnected | Status::Error => include_bytes!("../icons/tray/offline.png"),
    };
    Image::from_bytes(bytes).ok().map(Image::to_owned)
}

/// Tray icon, mutable menu entries, and last displayed state.
pub struct Tray<R: Runtime> {
    icon: TrayIcon<R>,
    summary: MenuItem<R>,
    open: MenuItem<R>,
    pause: MenuItem<R>,
    quit: MenuItem<R>,
    shown: Mutex<Option<(Status, bool, String)>>,
    blurred: AtomicU64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

pub fn create<R: Runtime>(
    app: &AppHandle<R>,
    on_pause: impl Fn(&AppHandle<R>) + Send + Sync + 'static,
) -> tauri::Result<Tray<R>> {
    let texts = labels("it");
    let summary = MenuItem::with_id(app, "summary", texts.summary, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", texts.open, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", texts.pause, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &summary,
            &open,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("NAS Sync")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "summary" => show_mini(app, None),
            "open" => show_main(app),
            "pause" => on_pause(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } => toggle_mini(tray.app_handle(), rect),
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main(tray.app_handle()),
            _ => {}
        });
    if let Some(image) = icon(Status::Unconfigured) {
        builder = builder.icon(image);
    }
    Ok(Tray {
        icon: builder.build(app)?,
        summary,
        open,
        pause,
        quit,
        shown: Mutex::new(None),
        blurred: AtomicU64::new(0),
    })
}

impl<R: Runtime> Tray<R> {
    pub fn update(&self, snapshot: &Snapshot, locale: &str) {
        let current = (snapshot.status, snapshot.paused, locale.to_owned());
        let Ok(mut shown) = self.shown.lock() else {
            return;
        };
        if shown.as_ref() == Some(&current) {
            return;
        }
        let texts = labels(locale);
        let _ = self.icon.set_icon(icon(snapshot.status));
        let _ = self.icon.set_tooltip(Some(format!(
            "NAS Sync: {}",
            (texts.status)(snapshot.status)
        )));
        let _ = self.summary.set_text(texts.summary);
        let _ = self.open.set_text(texts.open);
        let _ = self.quit.set_text(texts.quit);
        let _ = self.pause.set_text(if snapshot.paused {
            texts.resume
        } else {
            texts.pause
        });
        let _ = self.pause.set_enabled(snapshot.link.is_some());
        *shown = Some(current);
    }
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MINI) {
        let _ = window.hide();
    }
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_mini<R: Runtime>(app: &AppHandle<R>, anchor: Rect) {
    let Some(window) = app.get_webview_window(MINI) else {
        return;
    };
    let just_closed = app.try_state::<Tray<R>>().is_some_and(|tray| {
        now_ms().saturating_sub(tray.blurred.load(Ordering::Relaxed)) < REOPEN_AFTER_MS
    });
    if window.is_visible().unwrap_or(false) || just_closed {
        let _ = window.hide();
    } else {
        show_mini(app, Some(anchor));
    }
}

/// Places the summary beside the tray icon within screen bounds, with a bottom-right fallback.
pub fn mini_position(
    screen: (f64, f64, f64, f64),
    size: (f64, f64),
    anchor: Option<(f64, f64, f64, f64)>,
) -> (f64, f64) {
    let (left, top, width, height) = screen;
    let (x, y) = match anchor {
        Some((x, y, anchor_width, anchor_height)) => {
            let x = x + anchor_width / 2.0 - size.0 / 2.0;
            if y > top + height / 2.0 {
                (x, y - size.1 - MARGIN)
            } else {
                (x, y + anchor_height + MARGIN)
            }
        }
        None => (
            left + width - size.0 - MARGIN,
            top + height - size.1 - MARGIN,
        ),
    };
    (
        x.clamp(
            left + MARGIN,
            (left + width - size.0 - MARGIN).max(left + MARGIN),
        ),
        y.clamp(
            top + MARGIN,
            (top + height - size.1 - MARGIN).max(top + MARGIN),
        ),
    )
}

pub fn show_mini<R: Runtime>(app: &AppHandle<R>, anchor: Option<Rect>) {
    let Some(window) = app.get_webview_window(MINI) else {
        return;
    };
    place(&window, anchor);
    let _ = window.show();
    let _ = window.set_focus();
}

fn place<R: Runtime>(window: &WebviewWindow<R>, anchor: Option<Rect>) {
    let Ok(size) = window.outer_size() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let anchor = anchor.map(|rect| {
        let position = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        (position.x, position.y, size.width, size.height)
    });
    let monitor = anchor
        .and_then(|(x, y, _, _)| window.monitor_from_point(x, y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let area = monitor.work_area();
    let (x, y) = mini_position(
        (
            f64::from(area.position.x),
            f64::from(area.position.y),
            f64::from(area.size.width),
            f64::from(area.size.height),
        ),
        (f64::from(size.width), f64::from(size.height)),
        anchor,
    );
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The main window hides instead of closing, and the summary hides when it loses the focus.
pub fn window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    match (window.label(), event) {
        (MAIN, WindowEvent::CloseRequested { api, .. }) => {
            api.prevent_close();
            let _ = window.hide();
        }
        (MINI, WindowEvent::Focused(false)) => {
            if let Some(tray) = window.app_handle().try_state::<Tray<R>>() {
                tray.blurred.store(now_ms(), Ordering::Relaxed);
            }
            let _ = window.hide();
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "../tests/tray/mod.rs"]
mod tests;
