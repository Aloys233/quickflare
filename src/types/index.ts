/**
 * Shared TypeScript shapes — mirror `src-tauri/src/types.rs`.
 *
 * Keeping the types hand-mirrored (rather than auto-generated) makes the
 * Rust ⇄ TS contract explicit at code-review time. If you add a field on
 * either side, update the other.
 */

export type ServiceKind =
  | "vite"
  | "next-js"
  | "nuxt"
  | "spring-boot"
  | "node-js"
  | "python"
  | "docker"
  | "ssh"
  | "minecraft"
  | "postgres"
  | "mysql"
  | "redis"
  | "mongo"
  | "elastic"
  | "http"
  | "unknown";

export interface ListeningPort {
  port: number;
  address: string;
  pid: number | null;
  process: string | null;
  command: string | null;
  service: ServiceKind;
}

export type TunnelStatus =
  | "starting"
  | "live"
  | "stopping"
  | "stopped"
  | "crashed";

export type TunnelProviderKind =
  | "cloudflared"
  | "ngrok"
  | "pinggy"
  | "tailscale-funnel";

export interface TunnelSnapshot {
  id: string;
  provider: TunnelProviderKind;
  localPort: number;
  localTarget: string;
  status: TunnelStatus;
  publicUrl: string | null;
  createdAt: string;
  updatedAt: string;
  recentLogs: string[];
  label: string | null;
}

export interface CreateTunnelInput {
  provider: TunnelProviderKind;
  localPort: number;
  localTarget?: string | null;
  label?: string | null;
}

export interface TunnelLogEvent {
  tunnelId: string;
  line: string;
  stream: "stdout" | "stderr";
  at: string;
}

export interface CloudflaredStatus {
  installed: boolean;
  path: string | null;
  overrideUsed: boolean;
}

export type CloudflaredDownloadPhase =
  | "starting"
  | "downloading"
  | "finished"
  | "failed";

export interface CloudflaredDownloadProgress {
  phase: CloudflaredDownloadPhase;
  url: string;
  downloaded: number;
  total: number | null;
  path: string | null;
  message: string | null;
}

export interface Settings {
  autoRestart: boolean;
  launchAtLogin: boolean;
  closeToTray: boolean;
  cloudflaredPath: string | null;
  customHostname: string | null;
  scanIntervalSeconds: number;
}

export interface TunnelTokenStatus {
  saved: boolean;
}

export interface RecentTunnel {
  port: number;
  url: string;
  at: string;
}

/** Event names shared with Rust — kept as a const-object to avoid typos. */
export const Events = {
  TunnelUpdated: "tunnel://updated",
  TunnelLog: "tunnel://log",
  TunnelRemoved: "tunnel://removed",
  ScannerUpdated: "scanner://updated",
  CloudflaredDownloadProgress: "cloudflared://download-progress",
  TrayNavigate: "tray://navigate",
  TrayQuickCreate: "tray://quick-create",
  AppHiddenToTray: "app://hidden-to-tray",
} as const;
