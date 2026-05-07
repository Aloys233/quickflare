<script setup lang="ts">
/**
 * 应用根组件 — 左侧导航 + 主内容区。
 *
 * 标题栏交给系统原生 (`tauri.conf.json: decorations: true`)。
 * 主内容里的子 store 在 onMounted 异步初始化，每个 await 都包一层
 * try/catch — 确保即便某个 store 初始化失败，UI 仍然可见可操作。
 */
import { onBeforeUnmount, onMounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRouter } from "vue-router";
import Sidebar from "@/components/Sidebar.vue";
import { useScannerStore } from "@/stores/scanner";
import { useSettingsStore } from "@/stores/settings";
import { useTunnelsStore } from "@/stores/tunnels";
import { useLogsStore } from "@/stores/logs";
import { Events } from "@/types";

const tunnels = useTunnelsStore();
const scanner = useScannerStore();
const settings = useSettingsStore();
const logs = useLogsStore();
const router = useRouter();

const offFns: UnlistenFn[] = [];

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
        <transition name="page" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>
  </div>
</template>
