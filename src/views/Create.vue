<script setup lang="ts">
/**
 * 创建隧道 — 端口选择 + 启动按钮。
 *
 * 关键改动：fire-and-forget。点击「打开隧道」后立刻路由到控制台 —
 * 不再 await Rust 端的返回值。即便 cloudflared 启动很慢、或者
 * Rust 端因为某种原因迟迟不回，用户也能立刻看到主界面，并通过
 * 「日志」页观察 cloudflared 的输出。
 *
 * 隧道的状态会通过 `tunnel://updated` 事件实时同步到 Pinia store，
 * 主界面会自动渲染出新的 Starting 卡片，并随后转为 Live。
 */
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useScannerStore } from "@/stores/scanner";
import { useTunnelsStore } from "@/stores/tunnels";
import { useSettingsStore } from "@/stores/settings";

const props = defineProps<{ prefilledPort?: number }>();
const router = useRouter();
const scanner = useScannerStore();
const tunnels = useTunnelsStore();
const settings = useSettingsStore();
const { ports } = storeToRefs(scanner);

const port = ref<number | null>(props.prefilledPort ?? null);
const label = ref("");

// The component is reused when only the query changes (tray quick-create
// while already on /create, or a second click from the scanner table), so
// the initial value alone is not enough.
watch(
  () => props.prefilledPort,
  (next) => {
    if (next) port.value = next;
  },
);

const quickPicks = computed(() =>
  ports.value
    .filter((p) => p.port >= 1024 && p.service !== "ssh")
    .slice(0, 8),
);

const cloudflaredReady = computed(
  () => settings.cloudflared?.installed === true,
);

function submit() {
  // Failures from the backend are surfaced as a toast by the tunnels
  // store, so the only thing worth blocking on here is a missing binary.
  if (!port.value || !cloudflaredReady.value) return;

  // 不 await：Rust 端会通过事件实时同步状态，主界面立刻可用。
  void tunnels
    .create(port.value, label.value || undefined)
    .catch(() => {
      // Already reported via `tunnels.lastError` — the toast renders it.
    });

  // 立即路由到控制台 —— 用户在那里观察新隧道的启动过程。
  router.push("/");
}

function cancel() {
  router.push("/");
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-8 py-7">
    <header>
      <p class="text-xs font-medium uppercase tracking-wider text-dim">
        新建隧道
      </p>
      <h1 class="mt-1.5 text-2xl font-semibold leading-tight text-primary">
        选择本地端口
      </h1>
      <p class="mt-1 text-sm text-muted">
        Quickflare 将运行
        <code class="mono text-primary"
          >cloudflared tunnel --url http://localhost:&lt;端口&gt;</code
        >
        ，公网地址出现后会自动出现在「控制台」与「日志」页。
      </p>
    </header>

    <form
      @submit.prevent="submit"
      class="surface flex flex-col gap-5 rounded-xl p-6"
    >
      <div class="grid grid-cols-[1fr_auto] gap-6">
        <label class="flex flex-col">
          <span class="text-xs font-medium uppercase tracking-wider text-dim">
            本地端口
          </span>
          <input
            v-model.number="port"
            type="number"
            inputmode="numeric"
            min="1"
            max="65535"
            placeholder="3000"
            class="mt-2 bg-transparent text-5xl font-semibold tracking-tight text-primary placeholder:text-dim focus:outline-none"
          />
        </label>

        <label class="flex flex-col">
          <span class="text-xs font-medium uppercase tracking-wider text-dim">
            备注（可选）
          </span>
          <input
            v-model="label"
            placeholder="my-app"
            class="input-text mt-2 w-72"
          />
        </label>
      </div>

      <div v-if="quickPicks.length" class="border-t hairline pt-4">
        <p class="text-xs font-medium uppercase tracking-wider text-dim">
          本机快捷选择
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            v-for="p in quickPicks"
            :key="`${p.address}:${p.port}`"
            type="button"
            class="pill transition hover:border-brand hover:text-brand"
            :class="port === p.port && 'border-brand text-brand'"
            @click="port = p.port"
          >
            <span class="mono">:{{ p.port }}</span>
            <span class="text-dim">· {{ p.process ?? "?" }}</span>
          </button>
        </div>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-2 border-t hairline pt-4"
      >
        <p class="text-xs text-muted">
          <template v-if="cloudflaredReady">
            cloudflared 位于
            <span class="mono text-primary">{{
              settings.cloudflared?.path
            }}</span>
          </template>
          <template v-else>
            <span class="text-red-600 dark:text-red-400">未检测到 cloudflared</span>
            — 请先安装，或在
            <button
              type="button"
              class="text-brand underline underline-offset-2"
              @click="router.push('/settings')"
            >
              设置
            </button>
            里指定路径
          </template>
        </p>
        <div class="flex items-center gap-2">
          <button type="button" class="btn" @click="cancel">取消</button>
          <button
            type="submit"
            class="btn btn-primary"
            :disabled="!port || !cloudflaredReady"
          >
            <svg
              viewBox="0 0 24 24"
              class="h-4 w-4"
              stroke="currentColor"
              fill="none"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M5 12h14" />
              <path d="m13 6 6 6-6 6" />
            </svg>
            打开隧道
          </button>
        </div>
      </div>
    </form>
  </section>
</template>
