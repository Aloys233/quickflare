//! System tray icon, menu and event wiring.
//!
//! Linux: relies on the `libayatana-appindicator` package — present on
//! KDE Plasma + GNOME with a small extra packaging step (declared in
//! `tauri.conf.json -> bundle.linux.deb.depends`). On Wayland the
//! StatusNotifierItem protocol works across KDE Plasma and most modern
//! desktops; if a desktop lacks an indicator host (sway / hyprland out
//! of the box) the user can install `waybar` / `swaync` / a similar
//! SNI-aware host.
//!
//! macOS: native NSStatusItem in template-icon mode.
//! Windows: native shell-tray with a 16×16 icon.

use crate::services::tunnel_manager::TunnelManager;
use crate::types::TunnelStatus;
use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Menu item identifiers — kept short, matched in the click handler.
mod ids {
    pub const OPEN: &str = "tray:open";
    pub const CREATE: &str = "tray:create";
    pub const STOP_ALL: &str = "tray:stop-all";
    pub const COPY_URL_PREFIX: &str = "tray:copy-url:";
    pub const RECENT_PORT_PREFIX: &str = "tray:recent-port:";
    pub const QUIT: &str = "tray:quit";
}

/// Build the tray on startup. We register a *static* skeleton menu now —
/// the dynamic sections (active tunnels, recent ports) are rebuilt on
/// every right-click via `rebuild_menu()`.
pub fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &TrayMenuModel::empty())?;

    let _tray = TrayIconBuilder::with_id("quickflare-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Quickflare")
        .on_tray_icon_event(|tray, event| {
            // Left-click → toggle the dashboard window. Right-click is
            // owned by the menu (Linux/Win) or treated the same on macOS.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| handle_menu_event(app, event))
        .build(app)?;

    Ok(())
}

/// Rebuild and swap the tray menu — call this whenever tunnels change.
#[allow(dead_code)]
pub fn rebuild_menu(app: &AppHandle) {
    let model = collect_model(app);
    if let Ok(menu) = build_menu(app, &model)
        && let Some(tray) = app.tray_by_id("quickflare-tray")
    {
        let _ = tray.set_menu(Some(menu));
    }
}

#[derive(Default)]
struct TrayMenuModel {
    active: Vec<ActiveItem>,
    recent_ports: Vec<u16>,
}

struct ActiveItem {
    id: String,
    label: String,
    url: Option<String>,
    is_live: bool,
}

impl TrayMenuModel {
    fn empty() -> Self {
        Self::default()
    }
}

#[allow(dead_code)]
fn collect_model(app: &AppHandle) -> TrayMenuModel {
    let manager = app.state::<TunnelManager>();
    let active = manager
        .list()
        .into_iter()
        .map(|t| ActiveItem {
            id: t.id,
            label: format!(
                "{}:{}（{}）",
                t.label.as_deref().unwrap_or("隧道"),
                t.local_port,
                match t.status {
                    TunnelStatus::Live => "已连接",
                    TunnelStatus::Starting => "启动中",
                    TunnelStatus::Stopping => "停止中",
                    TunnelStatus::Stopped => "已停止",
                    TunnelStatus::Crashed => "已崩溃",
                }
            ),
            url: t.public_url,
            is_live: matches!(t.status, TunnelStatus::Live),
        })
        .collect();

    let recent_ports = crate::store::StoreHandle::open(app)
        .map(|s| s.recent_ports())
        .unwrap_or_default();

    TrayMenuModel {
        active,
        recent_ports,
    }
}

fn build_menu(app: &AppHandle, model: &TrayMenuModel) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        ids::OPEN,
        "打开主界面",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        ids::CREATE,
        "创建隧道…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    if model.active.is_empty() {
        let item = MenuItem::with_id(
            app,
            "tray:no-active",
            "暂无活跃隧道",
            false,
            None::<&str>,
        )?;
        menu.append(&item)?;
    } else {
        for active in &model.active {
            let id = format!("{}{}", ids::COPY_URL_PREFIX, active.id);
            let label = match (&active.url, active.is_live) {
                (Some(url), true) => format!("复制 {}", short_url(url)),
                _ => active.label.clone(),
            };
            menu.append(&MenuItem::with_id(
                app,
                id,
                label,
                active.url.is_some(),
                None::<&str>,
            )?)?;
        }
        menu.append(&MenuItem::with_id(
            app,
            ids::STOP_ALL,
            "停止全部隧道",
            true,
            None::<&str>,
        )?)?;
    }

    if !model.recent_ports.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        let recent = Submenu::new(app, "最近使用的端口", true)?;
        for port in &model.recent_ports {
            let id = format!("{}{}", ids::RECENT_PORT_PREFIX, port);
            recent.append(&MenuItem::with_id(
                app,
                id,
                format!("打通 localhost:{port}"),
                true,
                None::<&str>,
            )?)?;
        }
        menu.append(&recent)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        ids::QUIT,
        "退出 Quickflare",
        true,
        Some("CmdOrCtrl+Q"),
    )?)?;

    Ok(menu)
}

fn short_url(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .chars()
        .take(28)
        .collect::<String>()
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().0.as_str();
    log::debug!("[tray] menu event: {id}");

    match id {
        ids::OPEN => show_main_window(app),
        ids::CREATE => {
            show_main_window(app);
            // Tell the frontend router to navigate to the create page.
            let _ = app.emit_to("main", "tray://navigate", "/create");
        }
        ids::STOP_ALL => {
            let manager = app.state::<TunnelManager>();
            manager.stop_all(app);
        }
        ids::QUIT => {
            app.exit(0);
        }
        other if other.starts_with(ids::COPY_URL_PREFIX) => {
            let tunnel_id = &other[ids::COPY_URL_PREFIX.len()..];
            let manager = app.state::<TunnelManager>();
            if let Some(snap) = manager.get(tunnel_id)
                && let Some(url) = snap.public_url
            {
                use tauri_plugin_clipboard_manager::ClipboardExt;
                let _ = app.clipboard().write_text(url);
            }
        }
        other if other.starts_with(ids::RECENT_PORT_PREFIX) => {
            let port_str = &other[ids::RECENT_PORT_PREFIX.len()..];
            if let Ok(port) = port_str.parse::<u16>() {
                show_main_window(app);
                let _ = app.emit_to(
                    "main",
                    "tray://quick-create",
                    serde_json::json!({ "port": port }),
                );
            }
        }
        _ => {}
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// Imports placed at the bottom so the module documentation stays at the top.
use tauri::Emitter;
