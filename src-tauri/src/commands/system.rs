//! System-level commands — cloudflared availability check, settings IO,
//! and the recent-history readers used by the tray menu.

use crate::error::{AppError, AppResult};
use crate::providers::{ProviderRegistry, cloudflared};
use crate::secrets;
use crate::store::{RecentTunnel, StoreHandle};
use crate::types::{Settings, TunnelProviderKind, events};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflaredStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub override_used: bool,
    /// Whether the "download cloudflared" flow can work on this platform.
    pub can_download: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelTokenStatus {
    pub saved: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCloudflaredInput {
    pub mirror: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflaredDownloadProgress {
    pub phase: CloudflaredDownloadPhase,
    pub url: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub path: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloudflaredDownloadPhase {
    Starting,
    Downloading,
    Finished,
    Failed,
}

const CLOUDFLARED_WINDOWS_AMD64_URL: &str = "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe";

const GITHUB_MIRRORS: [&str; 4] = [
    "https://hk.gh-proxy.org",
    "https://gh-proxy.org",
    "https://cdn.gh-proxy.org",
    "https://edgeone.gh-proxy.org",
];

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
    // `override_used` should only be true when the override is what actually
    // won — a stale path that no longer exists falls through to PATH.
    let override_matched = |resolved: &std::path::Path| {
        override_path
            .as_deref()
            .is_some_and(|o| std::path::Path::new(o) == resolved)
    };
    match provider.resolve_binary(Some(&app), override_path.as_deref()) {
        Ok(p) => CloudflaredStatus {
            installed: true,
            override_used: override_matched(&p),
            path: Some(p.display().to_string()),
            can_download: cloudflared::download_supported(),
        },
        Err(_) => CloudflaredStatus {
            installed: false,
            path: None,
            override_used: false,
            can_download: cloudflared::download_supported(),
        },
    }
}

#[tauri::command]
pub async fn download_cloudflared(
    app: AppHandle,
    input: Option<DownloadCloudflaredInput>,
) -> AppResult<CloudflaredStatus> {
    let target = cloudflared::download_target_path(&app)?;
    if target.exists() {
        return Ok(CloudflaredStatus {
            installed: true,
            path: Some(target.display().to_string()),
            override_used: false,
            can_download: cloudflared::download_supported(),
        });
    }

    let urls = cloudflared_download_urls(input.and_then(|i| i.mirror));
    let mut last_error = None;
    for url in urls {
        match download_cloudflared_from_url(&app, &url, &target).await {
            Ok(()) => {
                return Ok(CloudflaredStatus {
                    installed: true,
                    path: Some(target.display().to_string()),
                    override_used: false,
                    can_download: cloudflared::download_supported(),
                });
            }
            Err(e) => {
                last_error = Some(e.to_string());
                emit_download_progress(
                    &app,
                    CloudflaredDownloadProgress {
                        phase: CloudflaredDownloadPhase::Failed,
                        url,
                        downloaded: 0,
                        total: None,
                        path: None,
                        message: last_error.clone(),
                    },
                );
            }
        }
    }

    Err(AppError::Internal(
        last_error.unwrap_or_else(|| "cloudflared download failed".into()),
    ))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<Settings> {
    Ok(StoreHandle::open(&app)?.settings())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> AppResult<()> {
    StoreHandle::open(&app)?.save_settings(&settings)?;
    // Mirror the launch-at-login preference onto the OS. A failure here
    // (read-only HOME, unsupported platform, …) must not lose the rest of
    // the user's settings, so it is logged rather than returned.
    if let Err(e) = crate::services::autostart::set_enabled(settings.launch_at_login) {
        log::warn!("[autostart] could not update launch-at-login: {e}");
    }
    Ok(())
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

fn cloudflared_download_urls(selected_mirror: Option<String>) -> Vec<String> {
    let normalized = selected_mirror
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut mirrors = Vec::new();
    if let Some(mirror) = normalized {
        mirrors.push(mirror.trim_end_matches('/').to_string());
    }
    for mirror in GITHUB_MIRRORS {
        let mirror = mirror.to_string();
        if !mirrors.iter().any(|m| m == &mirror) {
            mirrors.push(mirror);
        }
    }

    mirrors
        .into_iter()
        .map(|mirror| format!("{mirror}/{CLOUDFLARED_WINDOWS_AMD64_URL}"))
        .chain(std::iter::once(CLOUDFLARED_WINDOWS_AMD64_URL.to_string()))
        .collect()
}

async fn download_cloudflared_from_url(
    app: &AppHandle,
    url: &str,
    target: &PathBuf,
) -> AppResult<()> {
    emit_download_progress(
        app,
        CloudflaredDownloadProgress {
            phase: CloudflaredDownloadPhase::Starting,
            url: url.to_string(),
            downloaded: 0,
            total: None,
            path: None,
            message: None,
        },
    );

    let response = reqwest::get(url)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let total = response.content_length();

    let parent = target
        .parent()
        .ok_or_else(|| AppError::Internal("cloudflared target has no parent".into()))?;
    tokio::fs::create_dir_all(parent).await?;

    let partial = target.with_extension("exe.download");
    let mut file = tokio::fs::File::create(&partial).await?;
    let mut downloaded = 0u64;
    let mut response = response;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        emit_download_progress(
            app,
            CloudflaredDownloadProgress {
                phase: CloudflaredDownloadPhase::Downloading,
                url: url.to_string(),
                downloaded,
                total,
                path: None,
                message: None,
            },
        );
    }

    file.flush().await?;
    drop(file);
    tokio::fs::rename(&partial, target).await?;

    emit_download_progress(
        app,
        CloudflaredDownloadProgress {
            phase: CloudflaredDownloadPhase::Finished,
            url: url.to_string(),
            downloaded,
            total,
            path: Some(target.display().to_string()),
            message: None,
        },
    );

    Ok(())
}

fn emit_download_progress(app: &AppHandle, payload: CloudflaredDownloadProgress) {
    let _ = app.emit(events::CLOUDFLARED_DOWNLOAD_PROGRESS, payload);
}
