//! Provider trait and registry.
//!
//! A `TunnelProvider` is a pure-data description of how to launch a tunnel
//! for one tool — *not* the runtime state of an active tunnel. The
//! `tunnel_manager` is the thing that actually spawns and supervises the
//! child process.

use crate::error::AppResult;
use crate::types::TunnelProviderKind;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;

/// A regex-style URL extractor: when called against a single line of the
/// child's stdout, it returns the public URL if the line announces one.
pub type UrlExtractor = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub trait TunnelProvider: Send + Sync {
    /// Stable identifier — see `TunnelProviderKind`.
    fn kind(&self) -> TunnelProviderKind;

    /// Human label for the UI. Reserved for the future provider-picker
    /// (today we only ship cloudflared).
    #[allow(dead_code)]
    fn display_name(&self) -> &'static str;

    /// Resolve the executable on disk. The resolver is responsible for
    /// trying `$PATH`, well-known install locations and the user's
    /// override from settings.
    fn resolve_binary(
        &self,
        app: Option<&AppHandle>,
        override_path: Option<&str>,
    ) -> AppResult<std::path::PathBuf>;

    /// Build the argv used to launch a tunnel for `target`. Excludes the
    /// program name itself — callers prepend that.
    fn build_args(
        &self,
        target: &str,
        protocol: Option<&str>,
        tunnel_token: Option<&str>,
    ) -> Vec<String>;

    /// Stateless URL extractor — see `UrlExtractor`.
    fn url_extractor(&self) -> UrlExtractor;
}

/// Lookup table of available providers. The cloudflared provider is
/// registered by default in `lib.rs`; future providers slot in alongside
/// it without touching the manager.
#[derive(Default)]
pub struct ProviderRegistry {
    inner: HashMap<TunnelProviderKind, Arc<dyn TunnelProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<P: TunnelProvider + 'static>(&mut self, provider: P) {
        self.inner.insert(provider.kind(), Arc::new(provider));
    }

    pub fn get(&self, kind: TunnelProviderKind) -> Option<Arc<dyn TunnelProvider>> {
        self.inner.get(&kind).cloned()
    }

    /// Reserved for the upcoming provider-picker UI.
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<TunnelProviderKind> {
        self.inner.keys().copied().collect()
    }
}
