/**
 * Pinia store: 全局日志缓冲。
 *
 * 订阅 `tunnel://log` 事件，把所有隧道的输出汇聚到一个有上限的环形
 * 缓冲。日志页直接消费它；其他页面（如 TunnelCard）只需要看
 * 该隧道的简略状态即可。
 */
import { defineStore } from "pinia";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/composables/useTauriBridge";
import { Events, type TunnelLogEvent, type TunnelSnapshot } from "@/types";

interface State {
  entries: TunnelLogEvent[];
  bound: boolean;
  unlisten: UnlistenFn[];
}

/** 最多保留多少行 — 旧行从头部丢弃。 */
const MAX_ENTRIES = 1000;

export const useLogsStore = defineStore("logs", {
  state: (): State => ({
    entries: [],
    bound: false,
    unlisten: [],
  }),

  actions: {
    async bind(): Promise<void> {
      if (this.bound) return;
      try {
        this.unlisten.push(
          await listen<TunnelLogEvent>(Events.TunnelLog, (e) => {
            this.push(e.payload);
          }),
        );
      } catch (e) {
        for (const off of this.unlisten) off();
        this.unlisten = [];
        throw e;
      }

      this.bound = true;

      try {
        const snapshots = await api.listTunnels();
        this.hydrateFromSnapshots(snapshots);
      } catch (e) {
        console.error("[logs] initial hydrate failed:", e);
      }
    },

    async dispose(): Promise<void> {
      for (const off of this.unlisten) off();
      this.unlisten = [];
      this.bound = false;
    },

    clear(): void {
      this.entries = [];
    },

    hydrateFromSnapshots(snapshots: TunnelSnapshot[]): void {
      const historical = snapshots.flatMap((t) =>
        t.recentLogs.map((line, index) => ({
          tunnelId: t.id,
          line,
          stream: "stdout" as const,
          at: new Date(
            new Date(t.updatedAt).getTime() -
              Math.max(t.recentLogs.length - index - 1, 0),
          ).toISOString(),
        })),
      );

      const seen = new Set(
        this.entries.map((e) => `${e.tunnelId}\n${e.at}\n${e.line}`),
      );
      for (const entry of historical) {
        const key = `${entry.tunnelId}\n${entry.at}\n${entry.line}`;
        if (!seen.has(key)) {
          this.entries.push(entry);
          seen.add(key);
        }
      }
      this.entries.sort(
        (a, b) => new Date(a.at).getTime() - new Date(b.at).getTime(),
      );
      this.trim();
    },

    push(entry: TunnelLogEvent): void {
      this.entries.push(entry);
      this.trim();
    },

    trim(): void {
      if (this.entries.length > MAX_ENTRIES) {
        this.entries.splice(0, this.entries.length - MAX_ENTRIES);
      }
    },
  },
});
