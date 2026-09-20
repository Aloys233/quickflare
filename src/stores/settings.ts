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
      // The three probes are independent: a platform keyring that isn't
      // available must not wipe out the settings, which previously made the
      // whole preferences page look unconfigured and suppressed the
      // "cloudflared is missing" prompt.
      const [settings, cloudflared, tunnelToken] = await Promise.allSettled([
        api.getSettings(),
        api.cloudflaredStatus(),
        api.tunnelTokenStatus(),
      ]);

      if (settings.status === "fulfilled") {
        this.settings = settings.value;
      } else {
        console.error("[settings] get_settings failed:", settings.reason);
      }
      if (cloudflared.status === "fulfilled") {
        this.cloudflared = cloudflared.value;
      } else {
        console.error(
          "[settings] cloudflared_status failed:",
          cloudflared.reason,
        );
      }
      if (tunnelToken.status === "fulfilled") {
        this.tunnelToken = tunnelToken.value;
      } else {
        console.error(
          "[settings] tunnel_token_status failed:",
          tunnelToken.reason,
        );
      }

      this.hydrated = true;
    },

    async update(patch: Partial<Settings>): Promise<void> {
      const next = { ...this.settings, ...patch };
      await api.saveSettings(next);
      this.settings = next;
      // The cloudflared override may have changed — re-probe. A probe
      // failure must not make a successful save look like it failed.
      try {
        this.cloudflared = await api.cloudflaredStatus();
      } catch (e) {
        console.error("[settings] cloudflared re-probe failed:", e);
      }
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
