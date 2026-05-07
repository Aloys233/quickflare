//! System-level commands — cloudflared availability check, settings IO,
//! and the recent-history readers used by the tray menu.

use crate::error::AppResult;
use crate::providers::ProviderRegistry;
use crate::secrets;
use crate::store::{RecentTunnel, StoreHandle};
use crate::types::{Settings, TunnelProviderKind};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflaredStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub override_used: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelTokenStatus {
    pub saved: bool,
}

#[tauri::command]
pub fn cloudflared_status(
    registry: State<'_, Arc<ProviderRegistry>>,
    app: AppHandle,
) -> CloudflaredStatus {
    let store = StoreHandle::open(&app).ok();
    let override_path = store.as_ref().and_then(|s| s.settings().cloudflared_path);
    let provider = registry
        .get(TunnelProviderKind::Cloudflared)
        .expect("cloudflared registered at startup");
    match provider.resolve_binary(override_path.as_deref()) {
        Ok(p) => CloudflaredStatus {
            installed: true,
            path: Some(p.display().to_string()),
            override_used: override_path.is_some(),
        },
        Err(_) => CloudflaredStatus {
            installed: false,
            path: None,
            override_used: override_path.is_some(),
        },
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<Settings> {
    Ok(StoreHandle::open(&app)?.settings())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> AppResult<()> {
    StoreHandle::open(&app)?.save_settings(&settings)
}

#[tauri::command]
pub fn tunnel_token_status() -> AppResult<TunnelTokenStatus> {
    Ok(TunnelTokenStatus {
        saved: secrets::tunnel_token()?.is_some(),
    })
}

#[tauri::command]
pub fn save_tunnel_token(token: String) -> AppResult<TunnelTokenStatus> {
    secrets::save_tunnel_token(&token)?;
    Ok(TunnelTokenStatus { saved: true })
}

#[tauri::command]
pub fn clear_tunnel_token() -> AppResult<TunnelTokenStatus> {
    secrets::clear_tunnel_token()?;
    Ok(TunnelTokenStatus { saved: false })
}

#[tauri::command]
pub fn get_recent_ports(app: AppHandle) -> AppResult<Vec<u16>> {
    Ok(StoreHandle::open(&app)?.recent_ports())
}

#[tauri::command]
pub fn get_recent_urls(app: AppHandle) -> AppResult<Vec<RecentTunnel>> {
    Ok(StoreHandle::open(&app)?.recent_urls())
}
