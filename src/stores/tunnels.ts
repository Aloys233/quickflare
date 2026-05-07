/**
 * Pinia store: tunnels.
 *
 * Mirrors the manager's view of the world by:
 *   1. priming with `list_tunnels` on app start
 *   2. patching individual entries on `tunnel://updated`
 *   3. handling `tunnel://log` to keep the per-tunnel ring buffer fresh
 */

import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/composables/useTauriBridge";
import { Events, type TunnelLogEvent, type TunnelSnapshot } from "@/types";

const TRY_CLOUDFLARE_URL_RE =
  /https?:\/\/[a-zA-Z0-9.-]+\.trycloudflare\.com/;

function lineContainsEdgeSuccess(line: string): boolean {
  return (
    line.includes("Registered tunnel connection") ||
    line.includes("Connection registered")
  );
}

interface State {
  tunnels: Record<string, TunnelSnapshot>;
  bound: boolean;
  unlisten: UnlistenFn[];
  pollHandle: number | null;
}

export const useTunnelsStore = defineStore("tunnels", {
  state: (): State => ({
    tunnels: {},
    bound: false,
    unlisten: [],
    pollHandle: null,
  }),

  getters: {
    list: (state): TunnelSnapshot[] =>
      Object.values(state.tunnels).sort(
        (a, b) =>
          new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime(),
      ),
    live: (state): TunnelSnapshot[] =>
      Object.values(state.tunnels).filter((t) => t.status === "live"),
    /** Most-recently created tunnel — used for the dashboard hero. */
    primary(): TunnelSnapshot | null {
      return this.list[0] ?? null;
    },
  },

  actions: {
    async refresh(): Promise<void> {
      const snapshot = await api.listTunnels();
      this.tunnels = Object.fromEntries(
        snapshot.map((t) => [t.id, this.withUrlFromLogs(t)]),
      );
    },

    async create(port: number, label?: string): Promise<TunnelSnapshot> {
      const snap = await api.createTunnel({
        provider: "cloudflared",
        localPort: port,
        label: label ?? null,
      });
      this.tunnels[snap.id] = snap;
      this.ensurePolling();
      return snap;
    },

    async stop(id: string): Promise<void> {
      const current = this.tunnels[id];
      if (current) {
        this.tunnels[id] = {
          ...current,
          status: "stopping",
          updatedAt: new Date().toISOString(),
          recentLogs: [...current.recentLogs, "[quickflare] stop requested"],
        };
      }
      this.ensurePolling();
      await api.stopTunnel(id);
    },

    async restart(id: string): Promise<TunnelSnapshot> {
      const snap = await api.restartTunnel(id);
      this.tunnels[snap.id] = snap;
      return snap;
    },

    async remove(id: string): Promise<void> {
      delete this.tunnels[id];
      this.ensurePolling();
      await api.removeTunnel(id);
    },

    ensurePolling(): void {
      if (this.pollHandle != null) return;
      this.pollHandle = window.setInterval(() => {
        void this.refresh().catch((e) => {
          console.error("[tunnels] poll refresh failed:", e);
        });
      }, 1000);
    },

    stopPolling(): void {
      if (this.pollHandle != null) {
        window.clearInterval(this.pollHandle);
        this.pollHandle = null;
      }
    },

    async bind(): Promise<void> {
      if (this.bound) return;

      // 先订阅事件再去 refresh —— 顺序很重要：如果 listen 失败我们直接抛，
      // 还没改动 bound 标志，下次能干净重试。任何一步成功即注册到 unlisten，
      // 析构时统一 off。
      try {
        this.unlisten.push(
          await listen<TunnelSnapshot>(Events.TunnelUpdated, (e) => {
            this.tunnels[e.payload.id] = this.withUrlFromLogs(e.payload);
          }),
        );
        this.unlisten.push(
          await listen<string>(Events.TunnelRemoved, (e) => {
            delete this.tunnels[e.payload];
          }),
        );
        this.unlisten.push(
          await listen<TunnelLogEvent>(Events.TunnelLog, (e) => {
            const t = this.tunnels[e.payload.tunnelId];
            if (!t) return;
            const next = [...t.recentLogs, e.payload.line];
            if (next.length > 200) next.splice(0, next.length - 200);
            const url = e.payload.line.match(TRY_CLOUDFLARE_URL_RE)?.[0];
            this.tunnels[e.payload.tunnelId] = {
              ...t,
              publicUrl: t.publicUrl ?? url ?? null,
              status: t.publicUrl || url ? "live" : t.status,
              recentLogs: next,
            };
          }),
        );
      } catch (e) {
        // 任意 listen 失败:回滚已经注册的,避免泄漏。
        for (const off of this.unlisten) off();
        this.unlisten = [];
        throw e;
      }

      this.bound = true;

      // refresh 失败不致命 —— UI 会以空状态启动,后续事件会补上。
      try {
        await this.refresh();
      } catch (e) {
        console.error("[tunnels] initial refresh failed:", e);
      }
      this.ensurePolling();
    },

    async dispose(): Promise<void> {
      for (const off of this.unlisten) off();
      this.unlisten = [];
      this.bound = false;
      this.stopPolling();
    },

    withUrlFromLogs(tunnel: TunnelSnapshot): TunnelSnapshot {
      if (tunnel.publicUrl) return tunnel;
      const url = tunnel.recentLogs
        .map((line) => line.match(TRY_CLOUDFLARE_URL_RE)?.[0])
        .find(Boolean);
      if (!url) return tunnel;
      const connected = tunnel.recentLogs.some(lineContainsEdgeSuccess);
      return {
        ...tunnel,
        publicUrl: url,
        status:
          connected && tunnel.status === "starting" ? "live" : tunnel.status,
      };
    },
  },
});
