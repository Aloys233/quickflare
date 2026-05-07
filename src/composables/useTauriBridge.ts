/**
 * Typed wrappers around `@tauri-apps/api/core::invoke`.
 *
 * Every backend command lives here so the rest of the app talks to a
 * pure TypeScript module — easier to mock in tests, easier to refactor.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  CloudflaredStatus,
  CreateTunnelInput,
  ListeningPort,
  RecentTunnel,
  Settings,
  TunnelTokenStatus,
  TunnelSnapshot,
} from "@/types";

export const api = {
  // Scanner --------------------------------------------------------------
  scanPorts: () => invoke<ListeningPort[]>("scan_ports"),

  // Tunnels --------------------------------------------------------------
  listTunnels: () => invoke<TunnelSnapshot[]>("list_tunnels"),
  createTunnel: (input: CreateTunnelInput) =>
    invoke<TunnelSnapshot>("create_tunnel", { input }),
  stopTunnel: (id: string) => invoke<void>("stop_tunnel", { id }),
  removeTunnel: (id: string) => invoke<void>("remove_tunnel", { id }),
  restartTunnel: (id: string) =>
    invoke<TunnelSnapshot>("restart_tunnel", { id }),

  // System ---------------------------------------------------------------
  cloudflaredStatus: () => invoke<CloudflaredStatus>("cloudflared_status"),
  downloadCloudflared: (mirror?: string) =>
    invoke<CloudflaredStatus>("download_cloudflared", {
      input: mirror ? { mirror } : null,
    }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),
  tunnelTokenStatus: () =>
    invoke<TunnelTokenStatus>("tunnel_token_status"),
  saveTunnelToken: (token: string) =>
    invoke<TunnelTokenStatus>("save_tunnel_token", { token }),
  clearTunnelToken: () =>
    invoke<TunnelTokenStatus>("clear_tunnel_token"),
  getRecentPorts: () => invoke<number[]>("get_recent_ports"),
  getRecentUrls: () => invoke<RecentTunnel[]>("get_recent_urls"),
};

export type Api = typeof api;
