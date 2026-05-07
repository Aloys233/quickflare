//! Tunnel-management commands. Thin glue between the JS layer and
//! `services::tunnel_manager::TunnelManager`.

use crate::error::AppResult;
use crate::services::tunnel_manager::TunnelManager;
use crate::types::{CreateTunnelInput, TunnelSnapshot};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_tunnels(manager: State<'_, TunnelManager>) -> Vec<TunnelSnapshot> {
    manager.list()
}

#[tauri::command]
pub async fn create_tunnel(
    app: AppHandle,
    manager: State<'_, TunnelManager>,
    input: CreateTunnelInput,
) -> AppResult<TunnelSnapshot> {
    manager.create(app, input).await
}

#[tauri::command]
pub fn stop_tunnel(
    app: AppHandle,
    manager: State<'_, TunnelManager>,
    id: String,
) -> AppResult<()> {
    manager.stop(&app, &id)
}

#[tauri::command]
pub fn remove_tunnel(
    app: AppHandle,
    manager: State<'_, TunnelManager>,
    id: String,
) -> AppResult<()> {
    manager.remove(&app, &id)
}

#[tauri::command]
pub async fn restart_tunnel(
    app: AppHandle,
    manager: State<'_, TunnelManager>,
    id: String,
) -> AppResult<TunnelSnapshot> {
    manager.restart(app, &id).await
}
