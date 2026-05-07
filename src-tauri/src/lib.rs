//! Quickflare desktop entry point.
//!
//! This file is the *single* place where the app composition root lives —
//! plugins are registered, providers are bootstrapped, the tunnel manager
//! is added to Tauri's typed state, the system tray is installed, and
//! every command is wired into the IPC bridge.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod providers;
mod secrets;
mod services;
mod store;
mod tray;
mod types;

use std::sync::Arc;

use providers::{ProviderRegistry, cloudflared::CloudflaredProvider};
use services::tunnel_manager::TunnelManager;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

#[cfg(target_os = "linux")]
fn configure_linux_webview_environment() {
    // WebKitGTK's DMABuf renderer can fail EGL initialization on some
    // Wayland/NVIDIA/driver combinations before the Tauri window exists.
    // Set the fallback before GTK/WebKit starts reading process env.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: `run` is entered at process startup, before Tauri creates
        // threads or initializes GTK/WebKit, so mutating the process
        // environment here cannot race other Rust code in this process.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    configure_linux_webview_environment();

    let app = tauri::Builder::default()
        // ── Plugins ──────────────────────────────────────────────────
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        // ── State ────────────────────────────────────────────────────
        .setup(|app| {
            // Build the provider registry. Future providers (ngrok,
            // pinggy, tailscale-funnel) call `register()` here.
            let mut registry = ProviderRegistry::new();
            registry.register(CloudflaredProvider);
            let registry = Arc::new(registry);
            app.manage(registry.clone());

            let manager = TunnelManager::new(registry);
            app.manage(manager);

            // Install the tray. Failures are logged but non-fatal — the
            // app still runs in window mode if the desktop has no tray.
            if let Err(e) = tray::install_tray(app.handle()) {
                log::warn!("[tray] install failed: {e:?} — continuing without tray");
            }

            Ok(())
        })
        // ── Commands ─────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            commands::scanner::scan_ports,
            commands::tunnel::list_tunnels,
            commands::tunnel::create_tunnel,
            commands::tunnel::stop_tunnel,
            commands::tunnel::remove_tunnel,
            commands::tunnel::restart_tunnel,
            commands::system::cloudflared_status,
            commands::system::get_settings,
            commands::system::save_settings,
            commands::system::tunnel_token_status,
            commands::system::save_tunnel_token,
            commands::system::clear_tunnel_token,
            commands::system::get_recent_ports,
            commands::system::get_recent_urls,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Tauri application");

    // We use `run` instead of `Builder::run` so we can intercept the
    // close-window event and translate it into "hide to tray" when the
    // user has that setting on.
    app.run(|handle, event| match event {
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            if label == "main" {
                let store = store::StoreHandle::open(handle).ok();
                let close_to_tray = store
                    .map(|s| s.settings().close_to_tray)
                    .unwrap_or(true);
                if close_to_tray
                    && let Some(window) = handle.get_webview_window("main")
                {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = handle.emit("app://hidden-to-tray", ());
                }
            }
        }
        RunEvent::ExitRequested { .. } => {
            // Best-effort: kill all child processes before we go. The
            // supervisor tasks will log their cleanup.
            let manager = handle.state::<TunnelManager>();
            manager.stop_all(handle);
        }
        _ => {}
    });
}
