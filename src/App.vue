<script setup lang="ts">
/**
 * 应用根组件 — 左侧导航 + 主内容区。
 *
 * 标题栏交给系统原生 (`tauri.conf.json: decorations: true`)。
 * 主内容里的子 store 在 onMounted 异步初始化，每个 await 都包一层
 * try/catch — 确保即便某个 store 初始化失败，UI 仍然可见可操作。
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRouter } from "vue-router";
import Sidebar from "@/components/Sidebar.vue";
import { useScannerStore } from "@/stores/scanner";
import { useSettingsStore } from "@/stores/settings";
import { useTunnelsStore } from "@/stores/tunnels";
import { useLogsStore } from "@/stores/logs";
import { Events, type CloudflaredDownloadProgress } from "@/types";

const tunnels = useTunnelsStore();
const scanner = useScannerStore();
const settings = useSettingsStore();
const logs = useLogsStore();
const router = useRouter();

const offFns: UnlistenFn[] = [];
const cloudflaredPromptDismissed = ref(false);
const downloadingCloudflared = ref(false);
const cloudflaredDownload = ref<CloudflaredDownloadProgress | null>(null);
const cloudflaredDownloadError = ref<string | null>(null);
const selectedMirror = ref("https://hk.gh-proxy.org");
let errorTimer = 0;

const mirrors = [
  "https://hk.gh-proxy.org",
  "https://gh-proxy.org",
  "https://cdn.gh-proxy.org",
  "https://edgeone.gh-proxy.org",
];

/** Only meaningful once `settings.hydrate()` has resolved. */
const cloudflaredMissing = computed(
  () => settings.cloudflared !== null && !settings.cloudflared.installed,
);

/** Auto-download only exists on Windows; elsewhere we point at the docs. */
const canDownloadCloudflared = computed(
  () => settings.cloudflared?.canDownload === true,
);

const showCloudflaredPrompt = computed(
  () => cloudflaredMissing.value && !cloudflaredPromptDismissed.value,
);

const cloudflaredDownloadPercent = computed(() => {
  const progress = cloudflaredDownload.value;
  const total = progress?.total;
  if (!total) return 0;
  return Math.min(100, Math.round((progress.downloaded / total) * 100));
});

const cloudflaredDownloadSize = computed(() => {
  const progress = cloudflaredDownload.value;
  if (!progress) return "";
  const downloaded = formatBytes(progress.downloaded);
  if (!progress.total) return downloaded;
  return `${downloaded} / ${formatBytes(progress.total)}`;
});

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) return `${Math.max(0, bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

async function downloadCloudflared() {
  downloadingCloudflared.value = true;
  cloudflaredDownload.value = null;
  cloudflaredDownloadError.value = null;
  try {
    // On success `settings.cloudflared.installed` flips to true and the
    // prompt closes on its own — no need to poke the flag directly.
    await settings.downloadCloudflared(selectedMirror.value);
  } catch (e) {
    cloudflaredDownloadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    downloadingCloudflared.value = false;
  }
}

function dismissCloudflaredPrompt() {
  cloudflaredPromptDismissed.value = true;
}

function openSettings() {
  cloudflaredPromptDismissed.value = true;
  void router.push("/settings");
}

// Surface action failures. The create view navigates away the moment the
// user submits, so a rejected `create_tunnel` has nowhere else to land.
watch(
  () => tunnels.lastError,
  (message) => {
    window.clearTimeout(errorTimer);
    if (!message) return;
    errorTimer = window.setTimeout(() => tunnels.clearError(), 8000);
  },
);

// Changing the scan interval in Settings has to take effect immediately —
// the poll was previously only started once, at mount.
watch(
  () => settings.settings.scanIntervalSeconds,
  (next) => {
    if (next !== scanner.intervalSeconds) scanner.startPolling(next);
  },
);

onMounted(async () => {
  // hydrate 失败不应阻塞 UI 渲染 —— 任意一步失败都打日志继续。
  try {
    await settings.hydrate();
  } catch (e) {
    console.error("[hydrate] settings:", e);
  }
  try {
    await tunnels.bind();
  } catch (e) {
    console.error("[hydrate] tunnels:", e);
  }
  try {
    await logs.bind();
  } catch (e) {
    console.error("[hydrate] logs:", e);
  }
  try {
    scanner.startPolling(settings.settings.scanIntervalSeconds);
  } catch (e) {
    console.error("[hydrate] scanner:", e);
  }

  try {
    offFns.push(
      await listen<CloudflaredDownloadProgress>(
        Events.CloudflaredDownloadProgress,
        (e) => {
          cloudflaredDownload.value = e.payload;
          if (e.payload.phase === "failed") {
            cloudflaredDownloadError.value = e.payload.message ?? "下载失败";
          }
        },
      ),
    );
    offFns.push(
      await listen<string>(Events.TrayNavigate, (e) => {
        router.push(e.payload);
      }),
    );
    offFns.push(
      await listen<{ port: number }>(Events.TrayQuickCreate, (e) => {
        router.push({ path: "/create", query: { port: e.payload.port } });
      }),
    );
  } catch (e) {
    console.error("[hydrate] tray events:", e);
  }
});

onBeforeUnmount(() => {
  for (const off of offFns) off();
  window.clearTimeout(errorTimer);
  scanner.stopPolling();
  void tunnels.dispose();
  void logs.dispose();
});
</script>

<template>
  <div class="flex h-full min-h-0">
    <Sidebar />

    <main class="flex min-w-0 flex-1 flex-col overflow-hidden">
      <router-view v-slot="{ Component }">
        <transition name="page" mode="out-in" appear>
          <component :is="Component" />
        </transition>
      </router-view>
    </main>
  </div>

  <div
    v-if="showCloudflaredPrompt"
    class="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/45 px-4"
  >
    <section class="surface w-full max-w-lg rounded-xl p-6 shadow-2xl">
      <header class="flex items-start justify-between gap-4">
        <div>
          <h2 class="text-base font-semibold text-primary">
            {{ canDownloadCloudflared ? "下载 cloudflared" : "需要 cloudflared" }}
          </h2>
          <p class="mt-1 text-sm text-muted">
            Quickflare 依赖官方的 cloudflared CLI 才能创建公网隧道。
          </p>
        </div>
        <button
          class="btn btn-ghost"
          type="button"
          :disabled="downloadingCloudflared"
          @click="dismissCloudflaredPrompt"
        >
          稍后
        </button>
      </header>

      <div
        v-if="canDownloadCloudflared"
        class="mt-5 grid gap-3 md:grid-cols-[1fr_auto]"
      >
        <select
          class="input-text"
          v-model="selectedMirror"
          :disabled="downloadingCloudflared"
        >
          <option v-for="mirror in mirrors" :key="mirror" :value="mirror">
            {{ mirror }}
          </option>
        </select>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="downloadingCloudflared"
          @click="downloadCloudflared"
        >
          {{ downloadingCloudflared ? "下载中" : "下载" }}
        </button>
      </div>

      <div v-else class="mt-5 rounded-lg border hairline bg-[var(--bg)] p-4">
        <p class="text-sm text-muted">
          请先用系统包管理器安装 cloudflared，例如
          <code class="mono text-primary">sudo apt install cloudflared</code>
          、
          <code class="mono text-primary">brew install cloudflared</code>
          ，或前往
          <code class="mono text-primary">developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/</code>
          。
        </p>
        <p class="mt-2 text-xs text-dim">
          已经装好了？在「设置」里指定二进制路径即可。
        </p>
        <button class="btn mt-3" type="button" @click="openSettings">
          打开设置
        </button>
      </div>

      <div v-if="cloudflaredDownload" class="mt-4">
        <div class="h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800">
          <div
            class="h-full rounded-full bg-brand transition-all"
            :style="{ width: `${cloudflaredDownloadPercent}%` }"
          />
        </div>
        <div class="mt-2 flex items-center justify-between gap-3 text-xs text-muted">
          <span class="truncate">
            {{
              cloudflaredDownload.phase === "finished"
                ? "下载完成"
                : cloudflaredDownload.url
            }}
          </span>
          <span class="shrink-0 mono">{{ cloudflaredDownloadSize }}</span>
        </div>
      </div>

      <p
        v-if="cloudflaredDownloadError"
        class="mt-3 text-xs text-red-600 dark:text-red-400"
      >
        {{ cloudflaredDownloadError }}
      </p>
    </section>
  </div>

  <!-- 操作失败提示 -->
  <Transition name="page">
    <div
      v-if="tunnels.lastError"
      class="fixed bottom-5 right-5 z-50 flex w-full max-w-sm items-start gap-3 rounded-lg border border-red-500/30 bg-[var(--bg-elev)] px-4 py-3 shadow-lg"
    >
      <div class="min-w-0 flex-1">
        <p class="text-sm font-medium text-primary">操作失败</p>
        <p class="mt-0.5 break-words text-xs text-muted">
          {{ tunnels.lastError }}
        </p>
      </div>
      <button class="btn btn-ghost shrink-0" @click="tunnels.clearError()">
        关闭
      </button>
    </div>
  </Transition>
</template>
