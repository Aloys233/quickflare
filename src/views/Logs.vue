<script setup lang="ts">
/**
 * 日志查看 — 终端风的滚动列表。
 *
 *  - 默认显示全部隧道
 *  - 顶部下拉可单选某个隧道
 *  - 自动跟随到底部（新行到达时滚到底）
 *  - 一键清空 / 复制
 */
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useLogsStore } from "@/stores/logs";
import { useTunnelsStore } from "@/stores/tunnels";

const route = useRoute();
const router = useRouter();
const logs = useLogsStore();
const tunnels = useTunnelsStore();
const { entries } = storeToRefs(logs);

const filter = ref<string>(String(route.query.tunnel ?? "all"));
const autoScroll = ref(true);
const scrollEl = ref<HTMLDivElement | null>(null);

const snapshotEntries = computed(() =>
  tunnels.list.flatMap((t) =>
    t.recentLogs.map((line, index) => ({
      tunnelId: t.id,
      line,
      stream: "stdout" as const,
      at: new Date(
        new Date(t.updatedAt).getTime() -
          Math.max(t.recentLogs.length - index - 1, 0),
      ).toISOString(),
    })),
  ),
);

const mergedEntries = computed(() => {
  const seen = new Set<string>();
  return [...snapshotEntries.value, ...entries.value]
    .filter((e) => {
      const key = `${e.tunnelId}\n${e.line}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .sort((a, b) => new Date(a.at).getTime() - new Date(b.at).getTime());
});

const filtered = computed(() =>
  filter.value === "all"
    ? mergedEntries.value
    : mergedEntries.value.filter((e) => e.tunnelId === filter.value),
);

const tunnelOptions = computed(() => [
  { id: "all", label: "全部隧道" },
  ...tunnels.list.map((t) => ({
    id: t.id,
    label: `${t.label ?? "隧道"}:${t.localPort}`,
  })),
]);

watch(filter, (v) => {
  // 维持 URL 与下拉值同步，方便从其他页面带参数跳转。
  router.replace({
    path: "/logs",
    query: v === "all" ? {} : { tunnel: v },
  });
});

watch(
  () => filtered.value.length,
  async () => {
    if (!autoScroll.value) return;
    await nextTick();
    const el = scrollEl.value;
    if (el) el.scrollTop = el.scrollHeight;
  },
);

function tunnelLabel(id: string) {
  const t = tunnels.tunnels[id];
  return t ? `${t.label ?? "tunnel"}:${t.localPort}` : id.slice(0, 8);
}

function formatTime(iso: string) {
  try {
    return new Date(iso).toLocaleTimeString("zh-CN", { hour12: false });
  } catch {
    return iso;
  }
}

async function copyAll() {
  const text = filtered.value
    .map(
      (e) =>
        `[${formatTime(e.at)}] (${tunnelLabel(e.tunnelId)}) ${e.line}`,
    )
    .join("\n");
  await writeText(text);
}

onMounted(() => {
  void tunnels.refresh().catch((e) => {
    console.error("[logs] refresh failed:", e);
  });
});
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden px-8 py-7">
    <header class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <p class="text-xs font-medium uppercase tracking-wider text-dim">
          日志
        </p>
        <h1 class="mt-1.5 text-2xl font-semibold leading-tight text-primary">
          实时输出
        </h1>
        <p class="mt-1 text-sm text-muted">
          所有隧道 cloudflared 的 stdout / stderr 在此汇总，最多保留 1000 行。
        </p>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <select v-model="filter" class="input-text text-sm">
          <option v-for="opt in tunnelOptions" :key="opt.id" :value="opt.id">
            {{ opt.label }}
          </option>
        </select>

        <label
          class="surface-flat flex items-center gap-2 rounded-md px-3 py-2 text-xs"
        >
          <input v-model="autoScroll" type="checkbox" class="accent-brand" />
          跟随到底部
        </label>

        <button class="btn" @click="copyAll" :disabled="filtered.length === 0">
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4"
            stroke="currentColor"
            fill="none"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="9" y="9" width="11" height="11" rx="2" />
            <path d="M5 15V5a2 2 0 0 1 2-2h10" />
          </svg>
          复制
        </button>
        <button class="btn" @click="logs.clear" :disabled="entries.length === 0">
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4"
            stroke="currentColor"
            fill="none"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M3 6h18" />
            <path d="m19 6-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6M14 11v6" />
            <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
          </svg>
          清空
        </button>
      </div>
    </header>

    <!-- 终端 -->
    <div
      ref="scrollEl"
      class="surface-flat mono flex-1 overflow-y-auto rounded-lg p-4 text-xs leading-relaxed"
    >
      <p
        v-if="filtered.length === 0"
        class="flex h-full items-center justify-center text-dim"
      >
        暂无日志输出
      </p>
      <div
        v-for="(e, i) in filtered"
        :key="`${e.at}-${i}`"
        class="grid grid-cols-[auto_auto_1fr] gap-3 py-0.5"
      >
        <span class="text-dim">{{ formatTime(e.at) }}</span>
        <span
          class="font-medium"
          :class="
            e.stream === 'stderr'
              ? 'text-red-600 dark:text-red-400'
              : 'text-brand'
          "
        >
          {{ tunnelLabel(e.tunnelId) }}
        </span>
        <span class="whitespace-pre-wrap break-all text-muted">{{
          e.line
        }}</span>
      </div>
    </div>
  </section>
</template>
