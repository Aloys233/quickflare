//! Thin wrapper around `tauri-plugin-store` for typed access to settings
//! and recent-port history.

use crate::error::{AppError, AppResult};
use crate::types::Settings;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::{Store, StoreExt};

/// Filename of the on-disk JSON store. Tauri places it under the
/// platform's config dir — e.g. `~/.config/Quickflare/quickflare.json`
/// on Linux.
pub const STORE_FILE: &str = "quickflare.json";

/// Top-level keys persisted in the store.
mod keys {
    pub const SETTINGS: &str = "settings";
    pub const RECENT_PORTS: &str = "recentPorts";
    pub const RECENT_URLS: &str = "recentUrls";
}

/// History entry — what the user shared and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTunnel {
    pub port: u16,
    pub url: String,
    pub at: chrono::DateTime<chrono::Utc>,
}

pub struct StoreHandle(pub Arc<Store<Wry>>);

impl StoreHandle {
    pub fn open(app: &AppHandle) -> AppResult<Self> {
        let store = app
            .store(STORE_FILE)
            .map_err(|e| AppError::Store(e.to_string()))?;
        Ok(Self(store))
    }

    // --- Settings ---------------------------------------------------------

    pub fn settings(&self) -> Settings {
        self.0
            .get(keys::SETTINGS)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> AppResult<()> {
        let v = serde_json::to_value(settings)
            .map_err(|e| AppError::Store(e.to_string()))?;
        self.0.set(keys::SETTINGS, v);
        self.0
            .save()
            .map_err(|e| AppError::Store(e.to_string()))?;
        Ok(())
    }

    // --- Recent ports -----------------------------------------------------

    pub fn recent_ports(&self) -> Vec<u16> {
        self.0
            .get(keys::RECENT_PORTS)
            .and_then(|v| serde_json::from_value::<Vec<u16>>(v).ok())
            .unwrap_or_default()
    }

    pub fn push_recent_port(&self, port: u16) -> AppResult<()> {
        let mut existing = self.recent_ports();
        existing.retain(|p| *p != port);
        existing.insert(0, port);
        existing.truncate(8);
        let v = serde_json::to_value(existing)
            .map_err(|e| AppError::Store(e.to_string()))?;
        self.0.set(keys::RECENT_PORTS, v);
        self.0
            .save()
            .map_err(|e| AppError::Store(e.to_string()))?;
        Ok(())
    }

    // --- Recent tunnel URLs ----------------------------------------------

    pub fn push_recent_url(&self, entry: RecentTunnel) -> AppResult<()> {
        let mut existing: Vec<RecentTunnel> = self
            .0
            .get(keys::RECENT_URLS)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        existing.retain(|e| e.url != entry.url);
        existing.insert(0, entry);
        existing.truncate(20);
        let v = serde_json::to_value(existing)
            .map_err(|e| AppError::Store(e.to_string()))?;
        self.0.set(keys::RECENT_URLS, v);
        self.0
            .save()
            .map_err(|e| AppError::Store(e.to_string()))?;
        Ok(())
    }

    pub fn recent_urls(&self) -> Vec<RecentTunnel> {
        self.0
            .get(keys::RECENT_URLS)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }
}
