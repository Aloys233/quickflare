//! Top-level error type for Quickflare.
//!
//! We funnel everything through one error so Tauri commands can return
//! `Result<T, AppError>` directly — Tauri serializes the `Display` impl
//! into the JS-side error.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// All errors surfaced from the Rust side.
///
/// `Serialize` is implemented manually so the frontend receives a plain
/// string — JS-side `try/catch` then sees a friendly message instead of a
/// structured object full of debug noise.
#[allow(dead_code)] // some variants are reserved for future validation paths
#[derive(Debug, Error)]
pub enum AppError {
    #[error("cloudflared binary not found in PATH — install it first")]
    CloudflaredMissing,

    #[error("tunnel `{0}` is not running")]
    TunnelNotFound(String),

    #[error("tunnel `{0}` is already running")]
    TunnelAlreadyRunning(String),

    #[error("invalid local target: {0}")]
    InvalidTarget(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("port scan failed: {0}")]
    Scan(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("secret store error: {0}")]
    SecretStore(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

/// Convenient `Result` alias for command handlers.
pub type AppResult<T> = std::result::Result<T, AppError>;
