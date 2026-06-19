//! Tray / menu-bar panel — the popover shown when the user clicks the tray icon
//! or presses the global shortcut.
//!
//! - **macOS** uses a native `NSPanel` (via `tauri-nspanel`) so the popover can
//!   float above the menu bar without activating the app or stealing focus.
//! - **Windows & Linux** have no `NSPanel` equivalent, so the panel is a regular
//!   borderless, transparent, always-on-top `WebviewWindow` that we position
//!   next to the tray icon and hide when it loses focus.
//!
//! Both implementations expose the same API, so `tray.rs` and `lib.rs` never
//! need platform `cfg`s:
//!   `init`, `show_panel`, `hide_panel`, `toggle_panel`, `handle_tray_click`,
//!   `position_panel_at_tray_icon`.

#[cfg(target_os = "macos")]
pub use mac::*;
#[cfg(not(target_os = "macos"))]
pub use other::*;

// ---------------------------------------------------------------------------
// macOS — NSPanel implementation (unchanged behavior from upstream).
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod mac {
    use tauri::{AppHandle, Manager, Position, Size};
    use tauri_nspanel::{
        CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt, tauri_panel,
    };

    fn monitor_contains_physical_point(
        origin_x: f64,
        origin_y: f64,
        width: f64,
        height: f64,
        point_x: f64,
        point_y: f64,
    ) -> bool {
        point_x >= origin_x
            && point_x < origin_x + width
            && point_y >= origin_y
            && point_y < origin_y + height
    }

    unsafe fn set_panel_frame_top_left(panel: &tauri_nspanel::NSPanel, x: f64, y: f64) {
        let point = tauri_nspanel::NSPoint::new(x, y);
        let _: () = objc2::msg_send![panel, setFrameTopLeftPoint: point];
    }

    fn set_panel_top_left_immediately(
        window: &tauri::WebviewWindow,
        app_handle: &AppHandle,
        panel_x: f64,
        panel_y: f64,
        primary_logical_h: f64,
    ) {
        let Ok(panel_handle) = app_handle.get_webview_panel("main") else {
            return;
        };

        let target_x = panel_x;
        let target_y = primary_logical_h - panel_y;

        if objc2_foundation::MainThreadMarker::new().is_some() {
            unsafe {
                set_panel_frame_top_left(panel_handle.as_panel(), target_x, target_y);
            }
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let panel_handle = panel_handle.clone();

        if let Err(error) = window.run_on_main_thread(move || {
            unsafe {
                set_panel_frame_top_left(panel_handle.as_panel(), target_x, target_y);
            }
            let _ = tx.send(());
        }) {
            log::warn!("Failed to position panel on main thread: {}", error);
            return;
        }

        if rx.recv().is_err() {
            log::warn!("Failed waiting for panel position on main thread");
        }
    }

    /// Get the existing panel or initialize it. `Some` if available, `None` on error.
    macro_rules! get_or_init_panel {
        ($app_handle:expr) => {
            match $app_handle.get_webview_panel("main") {
                Ok(panel) => Some(panel),
                Err(_) => {
                    if let Err(err) = init($app_handle) {
                        log::error!("Failed to init panel: {}", err);
                        None
                    } else {
                        match $app_handle.get_webview_panel("main") {
                            Ok(panel) => Some(panel),
                            Err(err) => {
                                log::error!("Panel missing after init: {:?}", err);
                                None
                            }
                        }
                    }
                }
            }
        };
    }

    /// Retrieve the tray icon rect and position the panel beneath it.
    fn position_panel_from_tray(app_handle: &AppHandle) {
        let Some(tray) = app_handle.tray_by_id("tray") else {
            log::debug!("position_panel_from_tray: tray icon not found");
            return;
        };
        match tray.rect() {
            Ok(Some(rect)) => {
                position_panel_at_tray_icon(app_handle, rect.position, rect.size);
            }
            Ok(None) => {
                log::debug!("position_panel_from_tray: tray rect not available yet");
            }
            Err(e) => {
                log::warn!("position_panel_from_tray: failed to get tray rect: {}", e);
            }
        }
    }

    /// Show the panel (initializing if needed), positioned under the tray icon.
    pub fn show_panel(app_handle: &AppHandle) {
        if let Some(panel) = get_or_init_panel!(app_handle) {
            panel.show_and_make_key();
            position_panel_from_tray(app_handle);
        }
    }

    /// Hide the panel if it exists.
    pub fn hide_panel(app_handle: &AppHandle) {
        if let Ok(panel) = app_handle.get_webview_panel("main") {
            panel.hide();
        }
    }

    /// Toggle panel visibility. Used by the global shortcut handler.
    pub fn toggle_panel(app_handle: &AppHandle) {
        let Some(panel) = get_or_init_panel!(app_handle) else {
            return;
        };

        if panel.is_visible() {
            log::debug!("toggle_panel: hiding panel");
            panel.hide();
        } else {
            log::debug!("toggle_panel: showing panel");
            panel.show_and_make_key();
            position_panel_from_tray(app_handle);
        }
    }

    /// Handle a left-click on the tray icon: toggle the panel, positioning it at
    /// the clicked icon rect when showing.
    pub fn handle_tray_click(app_handle: &AppHandle, position: Position, size: Size) {
        let Some(panel) = get_or_init_panel!(app_handle) else {
            return;
        };

        if panel.is_visible() {
            log::debug!("tray click: hiding panel");
            panel.hide();
            return;
        }
        log::debug!("tray click: showing panel");

        // macOS quirk: must show window before positioning to another monitor
        panel.show_and_make_key();
        position_panel_at_tray_icon(app_handle, position, size);
    }

    // Define our panel class and event handler together
    tauri_panel! {
        panel!(OpenUsagePanel {
            config: {
                can_become_key_window: true,
                is_floating_panel: true
            }
        })

        panel_event!(OpenUsagePanelEventHandler {
            window_did_resign_key(notification: &NSNotification) -> ()
        })
    }

    pub fn init(app_handle: &tauri::AppHandle) -> tauri::Result<()> {
        if app_handle.get_webview_panel("main").is_ok() {
            return Ok(());
        }

        let window = app_handle.get_webview_window("main").unwrap();

        let panel = window.to_panel::<OpenUsagePanel>()?;

        // Disable native shadow - it causes gray border on transparent windows
        // Let CSS handle shadow via shadow-xl class
        panel.set_has_shadow(false);
        panel.set_opaque(false);

        // Configure panel behavior
        panel.set_level(PanelLevel::MainMenu.value() + 1);

        panel.set_collection_behavior(
            CollectionBehavior::new()
                .move_to_active_space()
                .full_screen_auxiliary()
                .value(),
        );

        panel.set_style_mask(StyleMask::empty().nonactivating_panel().value());

        // Set up event handler to hide panel when it loses focus
        let event_handler = OpenUsagePanelEventHandler::new();

        let handle = app_handle.clone();
        event_handler.window_did_resign_key(move |_notification| {
            if let Ok(panel) = handle.get_webview_panel("main") {
                panel.hide();
            }
        });

        panel.set_event_handler(Some(event_handler.as_ref()));

        Ok(())
    }

    pub fn position_panel_at_tray_icon(
        app_handle: &tauri::AppHandle,
        icon_position: Position,
        icon_size: Size,
    ) {
        let window = app_handle.get_webview_window("main").unwrap();

        let (icon_phys_x, icon_phys_y) = match &icon_position {
            Position::Physical(pos) => (pos.x as f64, pos.y as f64),
            Position::Logical(pos) => (pos.x, pos.y),
        };
        let (icon_phys_w, icon_phys_h) = match &icon_size {
            Size::Physical(s) => (s.width as f64, s.height as f64),
            Size::Logical(s) => (s.width, s.height),
        };

        let monitors = window.available_monitors().expect("failed to get monitors");
        let primary_logical_h = window
            .primary_monitor()
            .ok()
            .flatten()
            .map(|m| m.size().height as f64 / m.scale_factor())
            .unwrap_or(0.0);

        let icon_center_x = icon_phys_x + (icon_phys_w / 2.0);
        let icon_center_y = icon_phys_y + (icon_phys_h / 2.0);

        let found_monitor = monitors.iter().find(|monitor| {
            let origin = monitor.position();
            let size = monitor.size();
            monitor_contains_physical_point(
                origin.x as f64,
                origin.y as f64,
                size.width as f64,
                size.height as f64,
                icon_center_x,
                icon_center_y,
            )
        });

        let monitor = match found_monitor {
            Some(m) => m.clone(),
            None => {
                log::warn!(
                    "No monitor found for tray rect center at ({:.0}, {:.0}), using primary",
                    icon_center_x,
                    icon_center_y
                );
                match window.primary_monitor() {
                    Ok(Some(m)) => m,
                    _ => return,
                }
            }
        };

        let target_scale = monitor.scale_factor();
        let mon_phys_x = monitor.position().x as f64;
        let mon_phys_y = monitor.position().y as f64;
        let mon_logical_x = mon_phys_x / target_scale;
        let mon_logical_y = mon_phys_y / target_scale;

        let icon_logical_x = mon_logical_x + (icon_phys_x - mon_phys_x) / target_scale;
        let icon_logical_y = mon_logical_y + (icon_phys_y - mon_phys_y) / target_scale;
        let icon_logical_w = icon_phys_w / target_scale;
        let icon_logical_h = icon_phys_h / target_scale;

        // Read panel width from the window, converted to logical points.
        let panel_width = match (window.outer_size(), window.scale_factor()) {
            (Ok(s), Ok(win_scale)) => s.width as f64 / win_scale,
            _ => {
                let conf: serde_json::Value =
                    serde_json::from_str(include_str!("../tauri.conf.json"))
                        .expect("tauri.conf.json must be valid JSON");
                conf["app"]["windows"][0]["width"]
                    .as_f64()
                    .expect("width must be set in tauri.conf.json")
            }
        };

        let icon_center_x = icon_logical_x + (icon_logical_w / 2.0);
        let panel_x = icon_center_x - (panel_width / 2.0);
        let nudge_up: f64 = 6.0;
        // Clamp to the monitor's top edge: when the menu bar is set to auto-hide,
        // the tray rect sits above the visible screen, which would otherwise push
        // the panel's top edge off-screen and clip it.
        let panel_y = (icon_logical_y + icon_logical_h - nudge_up).max(mon_logical_y);

        set_panel_top_left_immediately(&window, app_handle, panel_x, panel_y, primary_logical_h);
    }
}

// ---------------------------------------------------------------------------
// Windows & Linux — borderless always-on-top WebviewWindow popover.
// ---------------------------------------------------------------------------
#[cfg(not(target_os = "macos"))]
mod other {
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
    use tauri::{AppHandle, LogicalPosition, Manager, Position, Size};

    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    static VISIBLE: AtomicBool = AtomicBool::new(false);
    static LAST_AUTO_HIDE_MS: AtomicI64 = AtomicI64::new(0);

    /// When the panel is open and the user clicks the tray icon, the click first
    /// moves focus away from the panel — firing a blur that hides it — and only
    /// then delivers the tray click. Without a guard the tray handler would see a
    /// now-hidden panel and immediately reopen it. Swallow tray clicks that land
    /// within this window after a focus-loss hide.
    const REOPEN_GUARD_MS: i64 = 350;
    /// Gap in logical pixels between the tray icon and the panel.
    const PANEL_GAP: f64 = 8.0;

    fn now_ms() -> i64 {
        // Monotonic clock for the reopen/drag guards: a backward wall-clock jump
        // (NTP step, manual change, VM/sleep resume) must not wedge the tray
        // toggle by making the guard window appear arbitrarily long.
        use std::sync::OnceLock;
        use std::time::Instant;
        static EPOCH: OnceLock<Instant> = OnceLock::new();
        EPOCH.get_or_init(Instant::now).elapsed().as_millis() as i64
    }

    /// The window is fixed-size and non-resizable, so its logical size always
    /// equals the configured size — read it straight from the embedded config.
    fn panel_dims() -> (f64, f64) {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json must be valid JSON");
        let win = &conf["app"]["windows"][0];
        (
            win["width"].as_f64().unwrap_or(400.0),
            win["height"].as_f64().unwrap_or(500.0),
        )
    }

    fn icon_xy(position: &Position) -> (f64, f64) {
        match position {
            Position::Physical(p) => (p.x as f64, p.y as f64),
            Position::Logical(p) => (p.x, p.y),
        }
    }

    fn icon_wh(size: &Size) -> (f64, f64) {
        match size {
            Size::Physical(s) => (s.width as f64, s.height as f64),
            Size::Logical(s) => (s.width, s.height),
        }
    }

    fn is_visible(app_handle: &AppHandle) -> bool {
        let _ = app_handle;
        VISIBLE.load(Ordering::SeqCst)
    }

    fn show(app_handle: &AppHandle) {
        if let Some(window) = app_handle.get_webview_window("main") {
            // The caller positioned the window first; reveal and focus it. The
            // first show is also what boots the WebView2 page (a window that has
            // never been on-screen doesn't run its scripts on Windows), so the
            // frontend bootstraps + probes here.
            let _ = window.show();
            let _ = window.set_focus();
            VISIBLE.store(true, Ordering::SeqCst);
        }
    }

    fn position_from_tray(app_handle: &AppHandle) {
        let Some(tray) = app_handle.tray_by_id("tray") else {
            log::debug!("position_from_tray: tray icon not found");
            return;
        };
        match tray.rect() {
            Ok(Some(rect)) => position_panel_at_tray_icon(app_handle, rect.position, rect.size),
            Ok(None) => log::debug!("position_from_tray: tray rect not available yet"),
            Err(e) => log::warn!("position_from_tray: failed to get tray rect: {}", e),
        }
    }

    fn show_panel_at_tray(app_handle: &AppHandle, position: Position, size: Size) {
        let _ = init(app_handle);
        // Position before showing so the panel never flashes at its old location.
        position_panel_at_tray_icon(app_handle, position, size);
        show(app_handle);
    }

    pub fn init(app_handle: &AppHandle) -> tauri::Result<()> {
        if INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let Some(window) = app_handle.get_webview_window("main") else {
            log::warn!("panel init: main window not found");
            return Ok(());
        };

        // Popover behavior: floats above other windows, stays out of the taskbar
        // and Alt+Tab (skip_taskbar sets WS_EX_TOOLWINDOW on Windows).
        let _ = window.set_always_on_top(true);
        let _ = window.set_skip_taskbar(true);

        // Dismiss when the user clicks away — the analog of NSPanel resign-key.
        let handle = app_handle.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                LAST_AUTO_HIDE_MS.store(now_ms(), Ordering::SeqCst);
                hide_panel(&handle);
            }
        });

        Ok(())
    }

    /// Show the panel (initializing if needed), positioned at the tray icon.
    pub fn show_panel(app_handle: &AppHandle) {
        let _ = init(app_handle);
        position_from_tray(app_handle);
        show(app_handle);
    }

    /// Hide the panel.
    pub fn hide_panel(app_handle: &AppHandle) {
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.hide();
        }
        VISIBLE.store(false, Ordering::SeqCst);
    }

    /// Toggle panel visibility. Used by the global shortcut handler.
    pub fn toggle_panel(app_handle: &AppHandle) {
        if is_visible(app_handle) {
            log::debug!("toggle_panel: hiding panel");
            hide_panel(app_handle);
        } else {
            log::debug!("toggle_panel: showing panel");
            show_panel(app_handle);
        }
    }

    /// Handle a left-click on the tray icon: toggle the panel, positioning it at
    /// the clicked icon rect when showing.
    pub fn handle_tray_click(app_handle: &AppHandle, position: Position, size: Size) {
        if now_ms() - LAST_AUTO_HIDE_MS.load(Ordering::SeqCst) < REOPEN_GUARD_MS {
            // The same click just closed the panel via focus-loss; leave it closed.
            log::debug!("tray click: ignored (just auto-hid on blur)");
            return;
        }
        if is_visible(app_handle) {
            log::debug!("tray click: hiding panel");
            hide_panel(app_handle);
        } else {
            log::debug!("tray click: showing panel");
            show_panel_at_tray(app_handle, position, size);
        }
    }

    /// Position the panel relative to the tray icon. The Windows/Linux tray sits
    /// at the bottom of the screen, so the panel opens upward from just above the
    /// icon. Tauri windows use a top-left origin (no Y-flip, unlike NSPanel).
    pub fn position_panel_at_tray_icon(
        app_handle: &AppHandle,
        icon_position: Position,
        icon_size: Size,
    ) {
        let Some(window) = app_handle.get_webview_window("main") else {
            return;
        };

        let (icon_phys_x, icon_phys_y) = icon_xy(&icon_position);
        let (icon_phys_w, icon_phys_h) = icon_wh(&icon_size);

        let monitors = match window.available_monitors() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("position_panel: failed to get monitors: {}", e);
                return;
            }
        };

        let icon_center_x = icon_phys_x + (icon_phys_w / 2.0);
        let icon_center_y = icon_phys_y + (icon_phys_h / 2.0);

        let monitor = monitors
            .iter()
            .find(|m| {
                let o = m.position();
                let s = m.size();
                icon_center_x >= o.x as f64
                    && icon_center_x < (o.x as f64 + s.width as f64)
                    && icon_center_y >= o.y as f64
                    && icon_center_y < (o.y as f64 + s.height as f64)
            })
            .cloned()
            .or_else(|| window.primary_monitor().ok().flatten());

        let Some(monitor) = monitor else {
            log::warn!("position_panel: no monitor available");
            return;
        };

        let scale = monitor.scale_factor();
        let mon_phys_x = monitor.position().x as f64;
        let mon_phys_y = monitor.position().y as f64;
        let mon_logical_x = mon_phys_x / scale;
        let mon_logical_y = mon_phys_y / scale;
        let mon_logical_w = monitor.size().width as f64 / scale;
        let mon_logical_h = monitor.size().height as f64 / scale;

        let icon_logical_x = mon_logical_x + (icon_phys_x - mon_phys_x) / scale;
        let icon_logical_y = mon_logical_y + (icon_phys_y - mon_phys_y) / scale;
        let icon_logical_w = icon_phys_w / scale;

        let (panel_w, panel_h) = panel_dims();
        let icon_center_logical_x = icon_logical_x + (icon_logical_w / 2.0);

        let mut x = icon_center_logical_x - (panel_w / 2.0);
        let mut y = icon_logical_y - panel_h - PANEL_GAP;

        // Clamp to the monitor so the panel never spills off-screen.
        x = x.max(mon_logical_x).min(mon_logical_x + mon_logical_w - panel_w);
        y = y.max(mon_logical_y).min(mon_logical_y + mon_logical_h - panel_h);

        if let Err(e) = window.set_position(LogicalPosition::new(x, y)) {
            log::warn!("position_panel: set_position failed: {}", e);
        }
    }
}
