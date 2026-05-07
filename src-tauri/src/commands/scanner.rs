//! Port scanner commands.

use crate::error::AppResult;
use crate::services::port_scanner;
use crate::types::ListeningPort;

/// Run a one-shot scan and return the listening TCP ports.
///
/// The frontend can poll this on a timer (the default cadence is in
/// `Settings::scan_interval_seconds`) or trigger it on demand from the
/// scanner page's "Refresh" button.
#[tauri::command]
pub async fn scan_ports() -> AppResult<Vec<ListeningPort>> {
    // Spawn off the blocking scan so we don't tie up Tauri's main thread.
    tokio::task::spawn_blocking(port_scanner::scan)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
}
