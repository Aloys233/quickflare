<script setup lang="ts">
/**
 * 应用根组件 — 左侧导航 + 主内容区。
 *
 * 标题栏交给系统原生 (`tauri.conf.json: decorations: true`)。
 * 主内容里的子 store 在 onMounted 异步初始化，每个 await 都包一层
 * try/catch — 确保即便某个 store 初始化失败，UI 仍然可见可操作。
 */
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
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
const showCloudflaredPrompt = ref(false);
const downloadingCloudflared = ref(false);
const cloudflaredDownload = ref<CloudflaredDownloadProgress | null>(null);
const cloudflaredDownloadError = ref<string | null>(null);
const selectedMirror = ref("https://hk.gh-proxy.org");

const mirrors = [
  "https://hk.gh-proxy.org",
  "https://gh-proxy.org",
  "https://cdn.gh-proxy.org",
  "https://edgeone.gh-proxy.org",
];

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
    await settings.downloadCloudflared(selectedMirror.value);
    showCloudflaredPrompt.value = false;
  } catch (e) {
    cloudflaredDownloadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    downloadingCloudflared.value = false;
  }
}

onMounted(async () => {
  // hydrate 失败不应阻塞 UI 渲染 —— 任意一步失败都打日志继续。
  try {
    await settings.hydrate();
    showCloudflaredPrompt.value = !settings.cloudflared?.installed;
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
          <h2 class="text-base font-semibold text-primary">下载 cloudflared</h2>
          <p class="mt-1 text-sm text-muted">
            Quickflare 需要 cloudflared 才能创建公网隧道。
          </p>
        </div>
        <button
          class="btn btn-ghost"
          type="button"
          :disabled="downloadingCloudflared"
          @click="showCloudflaredPrompt = false"
        >
          稍后
        </button>
      </header>

      <div class="mt-5 grid gap-3 md:grid-cols-[1fr_auto]">
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
</template>
