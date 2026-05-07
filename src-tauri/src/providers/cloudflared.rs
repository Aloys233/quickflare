//! Cloudflare Tunnel provider — wraps the `cloudflared` CLI.
//!
//! We intentionally use the *Quick Tunnel* mode (`cloudflared tunnel
//! --url http://localhost:PORT`) so users can ship a public URL without
//! a Cloudflare account. Authenticated `named` tunnels are a future
//! extension and slot into the same trait.

use crate::error::{AppError, AppResult};
use crate::providers::traits::{TunnelProvider, UrlExtractor};
use crate::types::TunnelProviderKind;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;

/// Matches the line cloudflared prints once a Quick Tunnel is ready, e.g.
///   `2024-01-01T12:00:00Z INF |  https://abcd-ef.trycloudflare.com  |`
/// or just `https://something.trycloudflare.com` in `--no-tls-verify` mode.
static URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[a-zA-Z0-9.-]+\.trycloudflare\.com")
        .expect("static cloudflared url regex")
});

pub struct CloudflaredProvider;

impl TunnelProvider for CloudflaredProvider {
    fn kind(&self) -> TunnelProviderKind {
        TunnelProviderKind::Cloudflared
    }

    fn display_name(&self) -> &'static str {
        "Cloudflare Tunnel"
    }

    fn resolve_binary(&self, override_path: Option<&str>) -> AppResult<PathBuf> {
        if let Some(p) = override_path {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        // `which` searches PATH cross-platform, including AppImage / brew /
        // scoop / chocolatey install locations.
        which::which("cloudflared").map_err(|_| AppError::CloudflaredMissing)
    }

    fn build_args(
        &self,
        target: &str,
        protocol: Option<&str>,
        tunnel_token: Option<&str>,
    ) -> Vec<String> {
        // cloudflared 的全局 flag 必须放在子命令之前，否则会报 unknown
        // flag。我们尽量保持参数最小，让 cloudflared 用默认的 stderr 行
        // 缓冲日志输出 —— 这样 quick-tunnel 的 URL 横幅能稳定地落到我
        // 们的 reader 里。
        //
        // 注意：之前版本里加过 `--logfile /dev/null`，那会让部分版本
        // 的 cloudflared 把所有日志重定向到文件，导致我们永远收不到
        // URL 公告（也就是「一直卡在打开隧道」的根本原因）。
        let mut args = vec![
            "--no-autoupdate".to_string(),
            "tunnel".to_string(),
        ];
        if let Some(protocol) = protocol {
            args.push("--protocol".to_string());
            args.push(protocol.to_string());
        }
        if let Some(token) = tunnel_token {
            args.extend([
                "run".to_string(),
                "--token".to_string(),
                token.to_string(),
                "--url".to_string(),
                target.to_string(),
            ]);
        } else {
            args.extend(["--url".to_string(), target.to_string()]);
        }
        args
    }

    fn url_extractor(&self) -> UrlExtractor {
        Arc::new(|line: &str| URL_RE.find(line).map(|m| m.as_str().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quick_tunnel_url() {
        let extractor = CloudflaredProvider.url_extractor();
        let line = "2024-01-01T12:00:00Z INF |  https://abc-def.trycloudflare.com  |";
        assert_eq!(
            extractor(line),
            Some("https://abc-def.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn ignores_unrelated_lines() {
        let extractor = CloudflaredProvider.url_extractor();
        assert!(extractor("registering connection").is_none());
    }

    #[test]
    fn omits_protocol_by_default() {
        let args = CloudflaredProvider.build_args("http://localhost:1420", None, None);
        assert!(!args.iter().any(|arg| arg == "--protocol"));
    }

    #[test]
    fn accepts_protocol_override() {
        let args = CloudflaredProvider.build_args("http://localhost:1420", Some("http2"), None);
        assert!(args.windows(2).any(|pair| pair == ["--protocol", "http2"]));
    }

    #[test]
    fn accepts_tunnel_token() {
        let args = CloudflaredProvider.build_args(
            "http://localhost:1420",
            None,
            Some("secret-token"),
        );
        assert!(args.windows(2).any(|pair| pair == ["--token", "secret-token"]));
        assert!(args.windows(2).any(|pair| pair == ["--url", "http://localhost:1420"]));
    }
}
