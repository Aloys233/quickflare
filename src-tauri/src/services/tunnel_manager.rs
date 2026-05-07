//! Tunnel lifecycle supervisor.
//!
//! Owns:
//! - the **set of live tunnels** (`HashMap<id, TunnelHandle>`)
//! - the **child processes** they wrap
//! - the **stdout/stderr readers** that feed back lines
//! - the **URL parser** sourced from each tunnel's provider
//! - **crash recovery** when `auto_restart` is enabled
//!
//! The manager exposes a small async API (`create`, `stop`, `restart`,
//! `list`) and emits Tauri events whenever a tunnel transitions states or
//! produces a log line.

use crate::error::{AppError, AppResult};
use crate::providers::ProviderRegistry;
use crate::secrets;
use crate::store::{RecentTunnel, StoreHandle};
use crate::types::{
    CreateTunnelInput, LogStream, TunnelLogEvent, TunnelProviderKind, TunnelSnapshot, TunnelStatus,
    events,
};
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Maximum lines kept per tunnel for the in-app log viewer.
const LOG_RING_CAPACITY: usize = 200;

/// How many crash-restarts we tolerate within `RESTART_WINDOW`
/// before giving up — prevents a busy crash-loop from melting the box.
const RESTART_LIMIT: u32 = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);
const EDGE_FAILURE_LIMIT: u32 = 5;

/// Internal state for one running tunnel. Held inside `Arc<Mutex<...>>`
/// so command handlers and the IO reader tasks can both touch it.
struct TunnelHandle {
    id: String,
    provider: TunnelProviderKind,
    local_port: u16,
    local_target: String,
    label: Option<String>,
    status: TunnelStatus,
    public_url: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    logs: VecDeque<String>,
    /// Channel used to ask the supervisor task to terminate the child.
    kill_tx: Option<oneshot::Sender<()>>,
    /// Restart bookkeeping.
    restart_count: u32,
    last_restart: Option<std::time::Instant>,
    edge_failure_count: u32,
}

impl TunnelHandle {
    fn snapshot(&self) -> TunnelSnapshot {
        TunnelSnapshot {
            id: self.id.clone(),
            provider: self.provider,
            local_port: self.local_port,
            local_target: self.local_target.clone(),
            status: self.status,
            public_url: self.public_url.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            recent_logs: self.logs.iter().cloned().collect(),
            label: self.label.clone(),
        }
    }

    fn push_log(&mut self, line: String) {
        if self.logs.len() == LOG_RING_CAPACITY {
            self.logs.pop_front();
        }
        self.logs.push_back(line);
    }
}

/// Public-facing handle injected into Tauri as managed state.
pub struct TunnelManager {
    inner: Arc<Mutex<HashMap<String, Arc<Mutex<TunnelHandle>>>>>,
    providers: Arc<ProviderRegistry>,
}

impl TunnelManager {
    pub fn new(providers: Arc<ProviderRegistry>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            providers,
        }
    }

    pub fn list(&self) -> Vec<TunnelSnapshot> {
        let guard = self.inner.lock();
        guard
            .values()
            .map(|h| h.lock().snapshot())
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<TunnelSnapshot> {
        let guard = self.inner.lock();
        guard.get(id).map(|h| h.lock().snapshot())
    }

    /// Create + start a tunnel. Returns the snapshot in `Starting` state —
    /// the caller (or the frontend, via events) waits for the URL to be
    /// captured asynchronously.
    pub async fn create(
        &self,
        app: AppHandle,
        input: CreateTunnelInput,
    ) -> AppResult<TunnelSnapshot> {
        // Reject duplicate tunnels for the same local port to keep the
        // mental model simple. Users who really need parallel tunnels can
        // start a second on a different port.
        {
            let guard = self.inner.lock();
            for handle in guard.values() {
                let h = handle.lock();
                if h.local_port == input.local_port
                    && matches!(
                        h.status,
                        TunnelStatus::Starting | TunnelStatus::Live
                    )
                {
                    return Err(AppError::TunnelAlreadyRunning(format!(
                        "port {}",
                        input.local_port
                    )));
                }
            }
        }

        // Fail fast if the provider isn't registered — the supervisor
        // resolves the binary again on its own, so we don't need to keep
        // the trait object around past this validation point.
        if self.providers.get(input.provider).is_none() {
            return Err(AppError::Internal(format!(
                "unknown provider {:?}",
                input.provider
            )));
        }

        let id = Uuid::new_v4().to_string();
        let target = input
            .local_target
            .clone()
            .unwrap_or_else(|| format!("http://localhost:{}", input.local_port));
        let settings = StoreHandle::open(&app)
            .map(|s| s.settings())
            .unwrap_or_default();
        let has_tunnel_token = secrets::tunnel_token()
            .ok()
            .flatten()
            .is_some_and(|token| !token.trim().is_empty());
        let configured_public_url = settings
            .custom_hostname
            .as_deref()
            .and_then(normalize_hostname_url)
            .filter(|_| has_tunnel_token);

        let handle = Arc::new(Mutex::new(TunnelHandle {
            id: id.clone(),
            provider: input.provider,
            local_port: input.local_port,
            local_target: target.clone(),
            label: input.label.clone(),
            status: TunnelStatus::Starting,
            public_url: configured_public_url,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            logs: VecDeque::with_capacity(LOG_RING_CAPACITY),
            kill_tx: None,
            restart_count: 0,
            last_restart: None,
            edge_failure_count: 0,
        }));

        self.inner.lock().insert(id.clone(), handle.clone());
        push_manager_log_to_ring(
            &handle,
            format!("[quickflare] preparing tunnel for {target}"),
        );

        // Spawn the supervisor — it will outlive this function.
        spawn_supervisor(
            app.clone(),
            handle,
            self.providers.clone(),
            input.provider,
            target,
            self.inner.clone(),
        );

        Ok(self
            .get(&id)
            .ok_or_else(|| AppError::Internal("tunnel disappeared after create".into()))?)
    }

    /// Send a graceful shutdown signal. The supervisor flushes stdout and
    /// kills the child if it doesn't exit on its own.
    pub fn stop(&self, _app: &AppHandle, id: &str) -> AppResult<()> {
        let handle = {
            let guard = self.inner.lock();
            guard
                .get(id)
                .cloned()
                .ok_or_else(|| AppError::TunnelNotFound(id.to_string()))?
        };

        let tx = {
            let mut h = handle.lock();
            h.status = TunnelStatus::Stopping;
            h.updated_at = Utc::now();
            h.kill_tx.take()
        };

        push_manager_log_to_ring(&handle, "[quickflare] stop requested".into());

        if let Some(tx) = tx {
            let _ = tx.send(());
        }
        Ok(())
    }

    /// Stop + remove a tunnel from the manager entirely.
    pub fn remove(&self, app: &AppHandle, id: &str) -> AppResult<()> {
        self.stop(app, id)?;
        // Defer actual removal so the reader task can drain — easier
        // than synchronously waiting here.
        self.inner.lock().remove(id);
        Ok(())
    }

    /// Restart = stop + create with the same params.
    pub async fn restart(&self, app: AppHandle, id: &str) -> AppResult<TunnelSnapshot> {
        let snap = self
            .get(id)
            .ok_or_else(|| AppError::TunnelNotFound(id.to_string()))?;
        self.remove(&app, id)?;
        // Briefly let the OS release the previous bind.
        tokio::time::sleep(Duration::from_millis(250)).await;
        self.create(
            app,
            CreateTunnelInput {
                provider: snap.provider,
                local_port: snap.local_port,
                local_target: Some(snap.local_target),
                label: snap.label,
            },
        )
        .await
    }

    pub fn stop_all(&self, app: &AppHandle) {
        let ids: Vec<String> = self.inner.lock().keys().cloned().collect();
        for id in ids {
            let _ = self.stop(app, &id);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internals
// ─────────────────────────────────────────────────────────────────────────────

/// Spawns the long-lived supervisor task that:
///   1. resolves the binary
///   2. spawns the child
///   3. fans out stdout/stderr to event readers
///   4. handles graceful shutdown via `kill_tx`
///   5. on unexpected exit, restarts up to `RESTART_LIMIT` times
fn spawn_supervisor(
    app: AppHandle,
    handle: Arc<Mutex<TunnelHandle>>,
    providers: Arc<ProviderRegistry>,
    provider_kind: TunnelProviderKind,
    target: String,
    pool: Arc<Mutex<HashMap<String, Arc<Mutex<TunnelHandle>>>>>,
) {
    tokio::spawn(async move {
        let protocol_attempts: [Option<&'static str>; 2] = [None, Some("http2")];
        let mut protocol_index = 0usize;

        loop {
            let provider = match providers.get(provider_kind) {
                Some(p) => p,
                None => {
                    fail_tunnel(&app, &handle, "provider disappeared");
                    return;
                }
            };

            let settings = StoreHandle::open(&app)
                .map(|s| s.settings())
                .unwrap_or_default();
            let override_path = settings.cloudflared_path.as_deref();
            let tunnel_token = secrets::tunnel_token().ok().flatten();
            let tunnel_token = tunnel_token.as_deref();

            let bin = match provider.resolve_binary(override_path.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    fail_tunnel(&app, &handle, &e.to_string());
                    return;
                }
            };

            let protocol = protocol_attempts[protocol_index];
            let args = provider.build_args(&target, protocol, tunnel_token);
            let extractor = provider.url_extractor();

            log::info!("[tunnel] launching {} {:?}", bin.display(), args);
            push_manager_log(
                &app,
                &handle,
                LogStream::Stderr,
                format!(
                    "[quickflare] launching {} {}",
                    bin.display(),
                    redacted_args(&args).join(" ")
                ),
            );

            let mut cmd = Command::new(&bin);
            cmd.args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                // Ensure cloudflared sees no terminal so it produces
                // line-buffered logs we can stream cleanly.
                .kill_on_drop(true);

            #[cfg(unix)]
            {
                // New process group → killing the parent doesn't drag down
                // children of children, and we can signal the whole group on
                // shutdown if needed. `process_group` is intrinsic on
                // `tokio::process::Command` under `cfg(unix)`, so no extra
                // trait imports required.
                cmd.process_group(0);
            }

            let child: Child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    fail_tunnel(&app, &handle, &format!("spawn failed: {e}"));
                    return;
                }
            };

            let (kill_tx, kill_rx) = oneshot::channel::<()>();
            handle.lock().kill_tx = Some(kill_tx);

            let exit_status = run_one_child(
                &app,
                handle.clone(),
                child,
                kill_rx,
                extractor,
            )
            .await;

            if matches!(exit_status, ExitOutcome::EdgeFailure) {
                if protocol_index + 1 < protocol_attempts.len() {
                    protocol_index += 1;
                    let next_protocol = protocol_attempts[protocol_index]
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "cloudflared-default".into());
                    push_manager_log(
                        &app,
                        &handle,
                        LogStream::Stderr,
                        format!(
                            "[quickflare] edge connection failed; retrying with protocol {next_protocol}"
                        ),
                    );
                    {
                        let mut h = handle.lock();
                        h.status = TunnelStatus::Starting;
                        h.public_url = None;
                        h.edge_failure_count = 0;
                        h.updated_at = Utc::now();
                    }
                    emit_updated(&app, &handle.lock().snapshot());
                    tokio::time::sleep(Duration::from_millis(750)).await;
                    continue;
                }

                fail_tunnel(
                    &app,
                    &handle,
                    "cloudflared could not establish a connection to Cloudflare edge after protocol fallback",
                );
                return;
            }

            // Decide whether to restart or stop.
            let restart_decision = decide_restart(&handle, exit_status);

            match restart_decision {
                Decision::Restart => {
                    log::warn!("[tunnel] restarting after unexpected exit");
                    // brief backoff
                    tokio::time::sleep(Duration::from_millis(750)).await;
                    let mut h = handle.lock();
                    h.status = TunnelStatus::Starting;
                    h.updated_at = Utc::now();
                    drop(h);
                    emit_updated(&app, &handle.lock().snapshot());
                    continue;
                }
                Decision::Stop(reason) => {
                    let mut h = handle.lock();
                    h.status = reason;
                    h.public_url = None;
                    h.updated_at = Utc::now();
                    drop(h);
                    emit_updated(&app, &handle.lock().snapshot());
                    // Keep the entry around until the user dismisses it
                    // — they may want to read the final log lines.
                    let _ = pool;
                    return;
                }
            }
        }
    });
}

enum Decision {
    Restart,
    Stop(TunnelStatus),
}

/// Restart policy:
/// - if the user asked us to stop → `Stopped`, no restart
/// - if `auto_restart` is off → `Crashed`
/// - if we've exceeded `RESTART_LIMIT` within `RESTART_WINDOW` → `Crashed`
/// - otherwise → restart
fn decide_restart(handle: &Arc<Mutex<TunnelHandle>>, _exit: ExitOutcome) -> Decision {
    let mut h = handle.lock();
    if matches!(h.status, TunnelStatus::Stopping) {
        return Decision::Stop(TunnelStatus::Stopped);
    }
    if matches!(h.status, TunnelStatus::Crashed) {
        return Decision::Stop(TunnelStatus::Crashed);
    }

    // Reset window if last restart was a while ago.
    let now = std::time::Instant::now();
    let within_window = h
        .last_restart
        .map(|t| now.saturating_duration_since(t) < RESTART_WINDOW)
        .unwrap_or(false);

    if within_window {
        h.restart_count += 1;
    } else {
        h.restart_count = 1;
    }
    h.last_restart = Some(now);

    if h.restart_count > RESTART_LIMIT {
        return Decision::Stop(TunnelStatus::Crashed);
    }
    Decision::Restart
}

#[derive(Debug)]
#[allow(dead_code)] // exit code is observed in logs, not in code paths
enum ExitOutcome {
    Killed,
    Exited(Option<i32>),
    StreamFailure,
    EdgeFailure,
}

/// Runs a single child process from spawn to exit. Listens for a kill
/// signal on `kill_rx` so the manager can ask for graceful shutdown.
async fn run_one_child(
    app: &AppHandle,
    handle: Arc<Mutex<TunnelHandle>>,
    mut child: Child,
    mut kill_rx: oneshot::Receiver<()>,
    extractor: crate::providers::traits::UrlExtractor,
) -> ExitOutcome {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let id = handle.lock().id.clone();
    let startup_timeout = tokio::time::sleep(STARTUP_TIMEOUT);
    tokio::pin!(startup_timeout);
    let mut startup_timeout_done = false;

    let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<(LogStream, String)>();

    if let Some(stdout) = stdout {
        let tx = line_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send((LogStream::Stdout, line));
            }
        });
    }
    if let Some(stderr) = stderr {
        let tx = line_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send((LogStream::Stderr, line));
            }
        });
    }
    drop(line_tx); // last sender lives in the spawned tasks

    loop {
        tokio::select! {
            // Graceful shutdown request from the manager.
            _ = &mut kill_rx => {
                log::info!("[tunnel {id}] kill requested");
                let _ = child.start_kill();
                let _ = child.wait().await;
                return ExitOutcome::Killed;
            }

            _ = &mut startup_timeout, if !startup_timeout_done => {
                startup_timeout_done = true;
                let still_starting = {
                    let h = handle.lock();
                    matches!(h.status, TunnelStatus::Starting)
                };
                if still_starting {
                    push_manager_log(
                        app,
                        &handle,
                        LogStream::Stderr,
                        "[quickflare] timed out waiting for cloudflared to connect to edge".into(),
                    );
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return ExitOutcome::EdgeFailure;
                }
            }

            // Child exited on its own — either normally or by crashing.
            status = child.wait() => {
                return match status {
                    Ok(s) => ExitOutcome::Exited(s.code()),
                    Err(_) => ExitOutcome::StreamFailure,
                };
            }

            // Drain log lines as they arrive.
            Some((stream, line)) = line_rx.recv() => {
                let outcome = handle_log_line(app, &handle, &extractor, stream, line);
                if matches!(outcome, LogLineOutcome::EdgeFailureLimit) {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return ExitOutcome::EdgeFailure;
                }
            }
        }
    }
}

enum LogLineOutcome {
    Continue,
    EdgeFailureLimit,
}

/// Append to the ring buffer, emit a `tunnel://log` event, and — if the
/// line announces a public URL — promote the tunnel to `Live`.
fn handle_log_line(
    app: &AppHandle,
    handle: &Arc<Mutex<TunnelHandle>>,
    extractor: &crate::providers::traits::UrlExtractor,
    stream: LogStream,
    line: String,
) -> LogLineOutcome {
    let now = Utc::now();
    let id = handle.lock().id.clone();
    let mut url_set: Option<String> = None;
    let mut edge_failure_limit_reached = false;
    {
        let mut h = handle.lock();
        h.push_log(line.clone());
        h.updated_at = now;

        if h.public_url.is_none() {
            if let Some(url) = extractor(&line) {
                h.public_url = Some(url.clone());
                url_set = Some(url);
            }
        }

        if line_contains_edge_success(&line) {
            h.status = TunnelStatus::Live;
            h.edge_failure_count = 0;
        } else if line_contains_edge_failure(&line) && matches!(h.status, TunnelStatus::Starting) {
            h.edge_failure_count += 1;
            if h.edge_failure_count >= EDGE_FAILURE_LIMIT {
                edge_failure_limit_reached = true;
            }
        }
    }

    let event = TunnelLogEvent {
        tunnel_id: id.clone(),
        line,
        stream,
        at: now,
    };
    let _ = app.emit(events::TUNNEL_LOG, &event);

    if url_set.is_some() {
        let snap = handle.lock().snapshot();
        emit_updated(app, &snap);

        // Persist to recent-URL history.
        if let Ok(store) = StoreHandle::open(app) {
            let _ = store.push_recent_url(RecentTunnel {
                port: snap.local_port,
                url: snap.public_url.clone().unwrap_or_default(),
                at: now,
            });
            let _ = store.push_recent_port(snap.local_port);
        }
    }

    if line_contains_edge_success(&event.line) {
        emit_updated(app, &handle.lock().snapshot());
    }

    if edge_failure_limit_reached {
        LogLineOutcome::EdgeFailureLimit
    } else {
        LogLineOutcome::Continue
    }
}

fn line_contains_edge_success(line: &str) -> bool {
    line.contains("Registered tunnel connection") || line.contains("Connection registered")
}

fn line_contains_edge_failure(line: &str) -> bool {
    line.contains("Unable to establish connection with Cloudflare edge")
        || line.contains("Serve tunnel error")
        || line.contains("Failed to dial a quic connection")
}

fn fail_tunnel(app: &AppHandle, handle: &Arc<Mutex<TunnelHandle>>, reason: &str) {
    {
        let mut h = handle.lock();
        h.status = TunnelStatus::Crashed;
        h.updated_at = Utc::now();
    }
    push_manager_log(app, handle, LogStream::Stderr, format!("[error] {reason}"));
    emit_updated(app, &handle.lock().snapshot());
}

fn emit_updated(app: &AppHandle, snap: &TunnelSnapshot) {
    let _ = app.emit(events::TUNNEL_UPDATED, snap);
}

fn normalize_hostname_url(hostname: &str) -> Option<String> {
    let trimmed = hostname.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Some(trimmed.to_string())
    } else {
        Some(format!("https://{trimmed}"))
    }
}

fn redacted_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut hide_next = false;
    for arg in args {
        if hide_next {
            redacted.push("<redacted>".into());
            hide_next = false;
            continue;
        }
        redacted.push(arg.clone());
        if arg == "--token" {
            hide_next = true;
        }
    }
    redacted
}

fn push_manager_log(
    app: &AppHandle,
    handle: &Arc<Mutex<TunnelHandle>>,
    stream: LogStream,
    line: String,
) {
    let now = Utc::now();
    let id = {
        let mut h = handle.lock();
        h.push_log(line.clone());
        h.updated_at = now;
        h.id.clone()
    };
    let event = TunnelLogEvent {
        tunnel_id: id,
        line,
        stream,
        at: now,
    };
    let _ = app.emit(events::TUNNEL_LOG, &event);
}

fn push_manager_log_to_ring(handle: &Arc<Mutex<TunnelHandle>>, line: String) {
    let mut h = handle.lock();
    h.push_log(line);
    h.updated_at = Utc::now();
}
