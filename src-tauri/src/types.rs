//! Shared data types crossing the Rust ⇄ JS boundary.
//!
//! Every type uses `#[serde(rename_all = "camelCase")]` so the frontend
//! consumes idiomatic JS while Rust keeps its snake_case fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Port scanner
// ─────────────────────────────────────────────────────────────────────────────

/// A single locally-listening TCP port.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningPort {
    /// 1–65535
    pub port: u16,
    /// Bound address, e.g. `127.0.0.1` or `0.0.0.0`.
    pub address: String,
    /// Owning process ID, when discoverable.
    pub pid: Option<u32>,
    /// Process executable basename (`node`, `cloudflared`, …).
    pub process: Option<String>,
    /// Full command line — useful for distinguishing `node vite` vs `node next dev`.
    pub command: Option<String>,
    /// Heuristic service guess: `vite`, `nextjs`, `springboot`, etc.
    pub service: ServiceKind,
}

/// Coarse service classification driven by `process_detector::classify()`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceKind {
    Vite,
    NextJs,
    Nuxt,
    SpringBoot,
    NodeJs,
    Python,
    Docker,
    Ssh,
    Minecraft,
    Postgres,
    Mysql,
    Redis,
    Mongo,
    Elastic,
    Http,
    Unknown,
}

impl ServiceKind {
    /// Display label shown in the scanner table.
    ///
    /// The JS side has its own copy of this mapping (in
    /// `src/components/PortRow.vue`) so the Rust-side function is kept
    /// for command-line tooling and future server-side rendering.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            ServiceKind::Vite => "Vite",
            ServiceKind::NextJs => "Next.js",
            ServiceKind::Nuxt => "Nuxt",
            ServiceKind::SpringBoot => "Spring Boot",
            ServiceKind::NodeJs => "Node.js",
            ServiceKind::Python => "Python",
            ServiceKind::Docker => "Docker",
            ServiceKind::Ssh => "SSH",
            ServiceKind::Minecraft => "Minecraft",
            ServiceKind::Postgres => "Postgres",
            ServiceKind::Mysql => "MySQL",
            ServiceKind::Redis => "Redis",
            ServiceKind::Mongo => "MongoDB",
            ServiceKind::Elastic => "Elasticsearch",
            ServiceKind::Http => "HTTP",
            ServiceKind::Unknown => "Unknown",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tunnel state
// ─────────────────────────────────────────────────────────────────────────────

/// Lifecycle of a tunnel as observed by the manager.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TunnelStatus {
    /// Process spawned, no public URL yet.
    Starting,
    /// Public URL captured from stdout.
    Live,
    /// User asked us to stop; child still draining.
    Stopping,
    /// Cleanly stopped.
    Stopped,
    /// Process exited unexpectedly.
    Crashed,
}

/// Which underlying tunnelling provider is in use.
///
/// The architecture is intentionally pluggable — see `providers/traits.rs`.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TunnelProviderKind {
    Cloudflared,
    /// Reserved for future adapters.
    Ngrok,
    Pinggy,
    TailscaleFunnel,
}

/// User-visible snapshot of a tunnel — the source of truth shown in the UI
/// and the tray menu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelSnapshot {
    pub id: String,
    pub provider: TunnelProviderKind,
    pub local_port: u16,
    pub local_target: String,
    pub status: TunnelStatus,
    pub public_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Last 200 lines of stdout/stderr — drained from a ring buffer.
    pub recent_logs: Vec<String>,
    /// Optional human label set by the user.
    pub label: Option<String>,
}

/// Payload accepted by `commands::tunnel::create`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTunnelInput {
    pub provider: TunnelProviderKind,
    pub local_port: u16,
    /// Defaults to `http://localhost:{port}` if not provided.
    pub local_target: Option<String>,
    pub label: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Events emitted to the frontend
// ─────────────────────────────────────────────────────────────────────────────

/// Event names — kept as constants to avoid typo drift between Rust and TS.
pub mod events {
    pub const TUNNEL_UPDATED: &str = "tunnel://updated";
    pub const TUNNEL_LOG: &str = "tunnel://log";
    pub const CLOUDFLARED_DOWNLOAD_PROGRESS: &str = "cloudflared://download-progress";
    #[allow(dead_code)]
    pub const TUNNEL_REMOVED: &str = "tunnel://removed";
    /// Reserved for future scanner change events — emitted from a watcher.
    #[allow(dead_code)]
    pub const SCANNER_UPDATED: &str = "scanner://updated";
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelLogEvent {
    pub tunnel_id: String,
    pub line: String,
    pub stream: LogStream,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings persisted in the store
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    /// Auto-relaunch a tunnel that exits unexpectedly.
    pub auto_restart: bool,
    /// Launch Quickflare on system startup.
    pub launch_at_login: bool,
    /// Hide to tray on close instead of quitting.
    pub close_to_tray: bool,
    /// Override path to the cloudflared binary (otherwise resolved from PATH).
    pub cloudflared_path: Option<String>,
    /// Public hostname already routed to the token-backed tunnel.
    pub custom_hostname: Option<String>,
    /// Polling interval for the port scanner, in seconds.
    pub scan_interval_seconds: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_restart: true,
            launch_at_login: false,
            close_to_tray: true,
            cloudflared_path: None,
            custom_hostname: None,
            scan_interval_seconds: 5,
        }
    }
}
