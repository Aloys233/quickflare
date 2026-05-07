/**
 * Pinia store: scanner.
 *
 * Owns the live list of listening ports and the polling cadence.
 * Polling lives in the renderer for now — it's cheap, and it keeps the
 * Rust side stateless.
 */

import { defineStore } from "pinia";
import { api } from "@/composables/useTauriBridge";
import type { ListeningPort } from "@/types";

interface State {
  ports: ListeningPort[];
  loading: boolean;
  lastScannedAt: string | null;
  pollHandle: number | null;
  intervalSeconds: number;
}

export const useScannerStore = defineStore("scanner", {
  state: (): State => ({
    ports: [],
    loading: false,
    lastScannedAt: null,
    pollHandle: null,
    intervalSeconds: 5,
  }),

  getters: {
    /** Filter out things the user almost never wants to tunnel (the editor's
     *  hot-reload server, system services, etc.). */
    userFacing: (state) =>
      state.ports.filter((p) => p.port >= 1024 && p.service !== "ssh"),
    byService: (state) => {
      const map: Record<string, ListeningPort[]> = {};
      for (const p of state.ports) {
        (map[p.service] ??= []).push(p);
      }
      return map;
    },
  },

  actions: {
    async scan(): Promise<void> {
      this.loading = true;
      try {
        this.ports = await api.scanPorts();
        this.lastScannedAt = new Date().toISOString();
      } finally {
        this.loading = false;
      }
    },

    startPolling(intervalSeconds?: number): void {
      this.stopPolling();
      const nextInterval = intervalSeconds ?? this.intervalSeconds;
      this.intervalSeconds = nextInterval;
      // Kick off an immediate scan — the user shouldn't have to wait the
      // full interval to see results on first paint.
      void this.scan();
      this.pollHandle = window.setInterval(
        () => this.scan(),
        nextInterval * 1000,
      );
    },

    stopPolling(): void {
      if (this.pollHandle != null) {
        window.clearInterval(this.pollHandle);
        this.pollHandle = null;
      }
    },
  },
});
