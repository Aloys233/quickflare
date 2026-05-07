<script setup lang="ts">
/**
 * 端口扫描 — 列出本机所有监听的 TCP 端口。
 */
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useScannerStore } from "@/stores/scanner";
import { useTunnelsStore } from "@/stores/tunnels";
import PortRow from "@/components/PortRow.vue";
import EmptyState from "@/components/EmptyState.vue";

const scanner = useScannerStore();
const tunnels = useTunnelsStore();
const { ports, loading, lastScannedAt } = storeToRefs(scanner);

const search = ref("");
const showSystem = ref(false);

const inUsePorts = computed(
  () => new Set(tunnels.list.map((t) => t.localPort)),
);

const filtered = computed(() => {
  let rows = ports.value;
  if (!showSystem.value) {
    rows = rows.filter((p) => p.port >= 1024 && p.service !== "ssh");
  }
  if (search.value.trim()) {
    const q = search.value.toLowerCase();
    rows = rows.filter(
      (p) =>
        String(p.port).includes(q) ||
        p.service.toLowerCase().includes(q) ||
        (p.process ?? "").toLowerCase().includes(q) ||
        (p.command ?? "").toLowerCase().includes(q),
    );
  }
  return rows;
});

function timeAgo(iso: string | null): string {
  if (!iso) return "从未";
  const d = (Date.now() - new Date(iso).getTime()) / 1000;
  if (d < 5) return "刚刚";
  if (d < 60) return `${Math.floor(d)} 秒前`;
  return `${Math.floor(d / 60)} 分钟前`;
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-5 overflow-hidden px-8 py-7">
    <!-- 顶部 -->
    <header class="flex items-end justify-between gap-4">
      <div>
        <p class="text-xs font-medium uppercase tracking-wider text-dim">
          扫描
        </p>
        <h1 class="mt-1.5 text-2xl font-semibold leading-tight text-primary">
          监听端口
        </h1>
        <p class="mt-1 text-sm text-muted">
          上次扫描 <span class="mono">{{ timeAgo(lastScannedAt) }}</span>
          · 每 {{ scanner.intervalSeconds }} 秒自动刷新
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <label
          class="surface-flat flex items-center gap-2 rounded-md px-3 py-2 text-xs"
        >
          <input v-model="showSystem" type="checkbox" class="accent-brand" />
          显示系统端口
        </label>
        <div
          class="surface-flat flex items-center gap-2 rounded-md px-3 py-2 text-sm"
        >
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4 text-dim"
            stroke="currentColor"
            fill="none"
            stroke-width="1.7"
          >
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-4-4" />
          </svg>
          <input
            v-model="search"
            placeholder="端口、服务、进程…"
            class="w-52 bg-transparent placeholder:text-dim focus:outline-none"
          />
        </div>
        <button class="btn" @click="scanner.scan()" :disabled="loading">
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4"
            :class="loading && 'animate-spin'"
            stroke="currentColor"
            fill="none"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M3 12a9 9 0 1 0 3-6.7" />
            <path d="M3 4v5h5" />
          </svg>
          刷新
        </button>
      </div>
    </header>

    <!-- 表头 -->
    <div
      class="grid grid-cols-[80px_1fr_1fr_120px_auto] gap-4 px-4 text-xs font-medium uppercase tracking-wider text-dim"
    >
      <span>端口</span>
      <span>服务</span>
      <span>进程</span>
      <span>状态</span>
      <span class="opacity-0">操作</span>
    </div>

    <!-- 列表 -->
    <div class="flex-1 space-y-1.5 overflow-y-auto pr-1">
      <PortRow
        v-for="row in filtered"
        :key="`${row.address}:${row.port}`"
        :port="row"
        :is-in-use="inUsePorts.has(row.port)"
      />
      <EmptyState
        v-if="!loading && filtered.length === 0"
        title="当前没有监听"
        description="启动一个开发服务器后再回来。Quickflare 会持续监听。"
      />
    </div>
  </section>
</template>
