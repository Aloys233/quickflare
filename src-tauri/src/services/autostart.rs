//! Launch-at-login integration.
//!
//! Implemented natively rather than through a plugin: every platform only
//! needs one small, well-documented artefact, and doing it ourselves keeps
//! the dependency surface (and the release binary) small.
//!
//! - **Linux** — an XDG autostart entry under
//!   `$XDG_CONFIG_HOME/autostart/app.quickflare.desktop` (read by KDE,
//!   GNOME, XFCE and friends).
//! - **macOS** — a per-user LaunchAgent plist under
//!   `~/Library/LaunchAgents/`.
//! - **Windows** — the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//!   registry value, written through `reg.exe` so we don't pull in a
//!   registry crate.

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

const APP_ID: &str = "app.quickflare.desktop";
const APP_NAME: &str = "Quickflare";

/// Turn launch-at-login on or off. Idempotent — re-applying the current
/// state simply rewrites the same artefact.
pub fn set_enabled(enabled: bool) -> AppResult<()> {
    if enabled {
        enable()
    } else {
        disable()
    }
}

#[allow(dead_code)] // unused on platforms without an autostart backend
fn current_exe() -> AppResult<PathBuf> {
    std::env::current_exe().map_err(AppError::Io)
}

#[allow(dead_code)] // unused on platforms without an autostart backend
fn home_dir() -> AppResult<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::Internal("HOME is not set".into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Linux — XDG autostart
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn entry_path() -> AppResult<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .map(Ok)
        .unwrap_or_else(|| home_dir().map(|h| h.join(".config")))?;
    Ok(config_dir.join("autostart").join(format!("{APP_ID}.desktop")))
}

#[cfg(target_os = "linux")]
fn enable() -> AppResult<()> {
    let exe = current_exe()?;
    let path = entry_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // `Exec` values are parsed by the desktop-entry spec, so a path with
    // spaces needs to be quoted.
    let body = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         Comment=Local Cloudflare Tunnel manager\n\
         Exec=\"{}\"\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    std::fs::write(&path, body)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable() -> AppResult<()> {
    let path = entry_path()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Io(e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// macOS — LaunchAgent
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn entry_path() -> AppResult<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{APP_ID}.plist")))
}

#[cfg(target_os = "macos")]
fn enable() -> AppResult<()> {
    let exe = current_exe()?;
    let path = entry_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Minimal LaunchAgent: run once at login, no keep-alive.
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{APP_ID}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
        exe.display()
    );
    std::fs::write(&path, body)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn disable() -> AppResult<()> {
    let path = entry_path()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Io(e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows — HKCU\...\Run
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
fn reg(args: &[&str]) -> std::io::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW — `reg.exe` would otherwise flash a console window
    // every time the setting is toggled.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("reg.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

#[cfg(target_os = "windows")]
fn enable() -> AppResult<()> {
    let exe = current_exe()?;
    // The stored command line is what Windows will execute; quote it so a
    // path containing spaces stays a single token.
    let value = format!("\"{}\"", exe.display());
    let out = reg(&[
        "add",
        RUN_KEY,
        "/v",
        APP_NAME,
        "/t",
        "REG_SZ",
        "/d",
        value.as_str(),
        "/f",
    ])
    .map_err(AppError::Io)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(AppError::Internal(format!(
            "reg add failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

#[cfg(target_os = "windows")]
fn disable() -> AppResult<()> {
    // `reg delete` exits non-zero when the value is absent; the end state we
    // want is "not present", so treat that as success too.
    let _ = reg(&["delete", RUN_KEY, "/v", APP_NAME, "/f"]).map_err(AppError::Io)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Everything else — no-op with an honest error
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn enable() -> AppResult<()> {
    Err(AppError::Internal(
        "launch at login is not implemented on this platform".into(),
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn disable() -> AppResult<()> {
    Ok(())
}
