/**
 * Pinia store: settings.
 *
 * Settings are persisted in `tauri-plugin-store`'s JSON file under the
 * platform config dir.
 */

import { defineStore } from "pinia";
import { api } from "@/composables/useTauriBridge";
import type { CloudflaredStatus, Settings, TunnelTokenStatus } from "@/types";

interface State {
  settings: Settings;
  cloudflared: CloudflaredStatus | null;
  tunnelToken: TunnelTokenStatus;
  hydrated: boolean;
}

const DEFAULTS: Settings = {
  autoRestart: true,
  launchAtLogin: false,
  closeToTray: true,
  cloudflaredPath: null,
  customHostname: null,
  scanIntervalSeconds: 5,
};

export const useSettingsStore = defineStore("settings", {
  state: (): State => ({
    settings: { ...DEFAULTS },
    cloudflared: null,
    tunnelToken: { saved: false },
    hydrated: false,
  }),

  actions: {
    async hydrate(): Promise<void> {
      const [settings, cloudflared, tunnelToken] = await Promise.all([
        api.getSettings(),
        api.cloudflaredStatus(),
        api.tunnelTokenStatus(),
      ]);
      this.settings = settings;
      this.cloudflared = cloudflared;
      this.tunnelToken = tunnelToken;
      this.hydrated = true;
    },

    async update(patch: Partial<Settings>): Promise<void> {
      const next = { ...this.settings, ...patch };
      await api.saveSettings(next);
      this.settings = next;
      // The cloudflared override may have changed — re-probe.
      this.cloudflared = await api.cloudflaredStatus();
    },

    async downloadCloudflared(mirror?: string): Promise<void> {
      this.cloudflared = await api.downloadCloudflared(mirror);
    },

    async saveTunnelToken(token: string): Promise<void> {
      this.tunnelToken = await api.saveTunnelToken(token);
    },

    async clearTunnelToken(): Promise<void> {
      this.tunnelToken = await api.clearTunnelToken();
    },
  },
});
