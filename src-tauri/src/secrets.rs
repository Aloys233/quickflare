//! OS-backed secret storage.
//!
//! `tauri-plugin-store` is fine for preferences, but tunnel tokens are
//! credentials. Store them in the platform keyring instead:
//! - Linux: Secret Service / libsecret-compatible keyring
//! - macOS: Keychain
//! - Windows: Credential Manager

use crate::error::{AppError, AppResult};
use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "app.quickflare.desktop";
const CLOUDFLARE_TUNNEL_TOKEN: &str = "cloudflare-tunnel-token";

fn token_entry() -> AppResult<Entry> {
    Entry::new(SERVICE, CLOUDFLARE_TUNNEL_TOKEN)
        .map_err(|e| AppError::SecretStore(e.to_string()))
}

pub fn tunnel_token() -> AppResult<Option<String>> {
    match token_entry()?.get_password() {
        Ok(token) if token.trim().is_empty() => Ok(None),
        Ok(token) => Ok(Some(token)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(AppError::SecretStore(e.to_string())),
    }
}

pub fn save_tunnel_token(token: &str) -> AppResult<()> {
    let token = token.trim();
    if token.is_empty() {
        return clear_tunnel_token();
    }
    token_entry()?
        .set_password(token)
        .map_err(|e| AppError::SecretStore(e.to_string()))
}

pub fn clear_tunnel_token() -> AppResult<()> {
    match token_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(AppError::SecretStore(e.to_string())),
    }
}
