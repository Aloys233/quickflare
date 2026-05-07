//! Cross-platform port scanner.
//!
//! We use `netstat2` to enumerate listening TCP sockets. The port table is
//! then joined with `sysinfo`'s process map to produce a row per port. The
//! actual classification (Vite vs Next.js, etc.) lives in
//! `process_detector.rs`.

use crate::error::{AppError, AppResult};
use crate::services::process_detector;
use crate::types::ListeningPort;
use netstat2::{
    AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info,
};
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Enumerate every TCP socket on the box that is currently in `LISTEN`.
///
/// Notes:
/// - Bound `[::]` / `0.0.0.0` listeners and explicit `127.0.0.1` listeners
///   are both returned — the UI filters them by the user's preference.
/// - Some processes can't be inspected without root (e.g. systemd services
///   on Linux). We surface them with `process: None` instead of failing.
pub fn scan() -> AppResult<Vec<ListeningPort>> {
    let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto = ProtocolFlags::TCP;

    let sockets = get_sockets_info(af, proto)
        .map_err(|e| AppError::Scan(e.to_string()))?;

    // Refresh sysinfo once up-front rather than per-process — much cheaper.
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new().with_cmd(sysinfo::UpdateKind::Always),
    );

    // Cache: port -> ListeningPort. We dedupe so a single process listening
    // on both IPv4 and IPv6 only shows up once.
    let mut by_port: HashMap<u16, ListeningPort> = HashMap::new();

    for socket in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Listen {
            continue;
        }

        // Prefer IPv4 (`127.0.0.1`/`0.0.0.0`) over IPv6 representations of
        // the same listener so the UI stays terse.
        if let Some(existing) = by_port.get(&tcp.local_port) {
            if existing.address.contains('.') {
                continue;
            }
        }

        let pid = socket.associated_pids.first().copied();
        let proc = pid.and_then(|pid| sys.process(sysinfo::Pid::from(pid as usize)));
        let process_name = proc.map(|p| p.name().to_string_lossy().into_owned());
        let command_line = proc.map(|p| {
            p.cmd()
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        });

        let service = process_detector::classify(
            tcp.local_port,
            process_name.as_deref(),
            command_line.as_deref(),
        );

        by_port.insert(
            tcp.local_port,
            ListeningPort {
                port: tcp.local_port,
                address: tcp.local_addr.to_string(),
                pid,
                process: process_name,
                command: command_line,
                service,
            },
        );
    }

    let mut ports: Vec<ListeningPort> = by_port.into_values().collect();
    ports.sort_by_key(|p| (priority_rank(p.port), p.port));
    Ok(ports)
}

/// 常见开发/调试端口排序权重 —— 数值越小越靠前。
/// 命中表内端口按表中顺序排列，未命中的统一排在后面再按端口号升序。
fn priority_rank(port: u16) -> u32 {
    const COMMON: &[u16] = &[
        1420,  // Tauri
        5173,  // Vite
        3000,  // Next.js / Node 默认
        4321,  // Astro
        8080,  // 常见 HTTP 替代
        8000,  // Django / Python
        5000,  // Flask / .NET
        4200,  // Angular
        8888,  // Jupyter
        9000,  // PHP-FPM / 通用
        3001, 8001, 8443, 80, 443,
    ];
    COMMON
        .iter()
        .position(|&p| p == port)
        .map(|i| i as u32)
        .unwrap_or(u32::MAX)
}
