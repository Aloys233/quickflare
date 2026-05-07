<script setup lang="ts">
/**
 * 控制台主卡片 — 公网地址、状态、操作按钮。
 *
 * 日志已经移到独立的 /logs 页面，这里只保留一个跳转入口。
 *
 * 取消逻辑：
 *   - Starting / Stopping → 立即调 `remove()`（停止 + 清除 entry，最快路径）
 *   - Live → 调 `stop()`（保留 entry，后续可重启）
 *   - Stopped / Crashed → 调 `remove()`（清理）
 */
import { computed } from "vue";
import { useRouter } from "vue-router";
// 注意:plugin-opener v2.5+ 把 `open` 重命名成了 `openUrl` / `openPath`。
// 用旧名字会在模块加载阶段抛 SyntaxError,导致整棵 Vue 树都无法挂载,
// 表现为「首次打开是空白」「控制台打不开」。
import { openUrl } from "@tauri-apps/plugin-opener";
import StatusDot from "@/components/StatusDot.vue";
import CopyButton from "@/components/CopyButton.vue";
import { useTunnelsStore } from "@/stores/tunnels";
import type { TunnelSnapshot } from "@/types";

const props = defineProps<{ tunnel: TunnelSnapshot }>();
const store = useTunnelsStore();
const router = useRouter();

async function visit(url: string) {
  await openUrl(url);
}

const stopLabel = computed(() => {
  switch (props.tunnel.status) {
    case "starting":
      return "取消";
    case "live":
      return "停止";
    case "stopping":
      return "取消";
    case "stopped":
    case "crashed":
    default:
      return "移除";
  }
});

function onStop() {
  if (props.tunnel.status === "live") {
    void store.stop(props.tunnel.id).catch((e) => {
      console.error("[tunnel] stop failed:", e);
    });
  } else {
    void store.remove(props.tunnel.id).catch((e) => {
      console.error("[tunnel] remove failed:", e);
    });
  }
}

function viewLogs() {
  router.push({ path: "/logs", query: { tunnel: props.tunnel.id } });
}
</script>

<template>
  <article class="surface animate-fade-up rounded-xl p-6">
    <!-- 顶部：状态 + 元信息 -->
    <div class="flex items-start justify-between gap-4">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
        <StatusDot :status="tunnel.status" label />
        <span class="text-xs text-muted">
          经由
          <span class="font-medium text-primary">Cloudflare Tunnel</span>
        </span>
        <span class="text-xs text-muted">
          本地
          <span class="mono ml-1 text-primary">{{ tunnel.localTarget }}</span>
        </span>
      </div>

      <div class="flex shrink-0 gap-1">
        <button
          v-if="tunnel.status === 'live' || tunnel.status === 'crashed'"
          class="btn btn-ghost"
          @click="store.restart(tunnel.id)"
        >
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4"
            stroke="currentColor"
            fill="none"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M3 12a9 9 0 1 0 3-6.7" />
            <path d="M3 4v5h5" />
          </svg>
          重启
        </button>
        <button class="btn btn-danger" @click="onStop">
          <svg viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
          {{ stopLabel }}
        </button>
      </div>
    </div>

    <!-- 公网地址 -->
    <div class="mt-6">
      <p class="text-xs font-medium uppercase tracking-wider text-dim">
        公网地址
      </p>
      <div class="mt-2 flex flex-wrap items-baseline gap-3">
        <span
          v-if="tunnel.publicUrl"
          class="mono break-all text-xl font-medium text-primary"
        >
          {{ tunnel.publicUrl }}
        </span>
        <span
          v-else-if="tunnel.status === 'starting'"
          class="text-xl text-muted"
        >
          正在打开隧道…
        </span>
        <span
          v-else-if="tunnel.status === 'crashed'"
          class="text-xl text-red-600 dark:text-red-400"
        >
          启动失败
        </span>
        <span v-else class="text-xl text-muted">未连接</span>
      </div>

      <div
        v-if="!tunnel.publicUrl && tunnel.status === 'starting'"
        class="progress-sweep mt-3 h-1 w-72"
      ></div>
    </div>

    <!-- 操作按钮 -->
    <div class="mt-5 flex flex-wrap items-center gap-2">
      <CopyButton
        v-if="tunnel.publicUrl"
        :text="tunnel.publicUrl"
        label="复制地址"
      />
      <button
        v-if="tunnel.publicUrl"
        class="btn"
        @click="visit(tunnel.publicUrl!)"
      >
        <svg
          viewBox="0 0 24 24"
          class="h-4 w-4"
          stroke="currentColor"
          fill="none"
          stroke-width="1.7"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M14 3h7v7" />
          <path d="M21 3l-9 9" />
          <path d="M21 14v7H3V3h7" />
        </svg>
        在浏览器中打开
      </button>
      <button class="btn btn-ghost ml-auto" @click="viewLogs">
        <svg
          viewBox="0 0 24 24"
          class="h-4 w-4"
          stroke="currentColor"
          fill="none"
          stroke-width="1.7"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M4 6h16M4 12h10M4 18h16" />
        </svg>
        查看日志
      </button>
    </div>
  </article>
</template>
