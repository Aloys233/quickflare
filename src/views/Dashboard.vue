<script setup lang="ts">
/**
 * 控制台 — 主屏。
 *
 *  - 顶部欢迎 + 新建按钮
 *  - 三个数据卡片
 *  - 主隧道详情卡（或空状态）
 *  - 其他会话网格
 */
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useTunnelsStore } from "@/stores/tunnels";
import { useSettingsStore } from "@/stores/settings";
import EmptyState from "@/components/EmptyState.vue";
import TunnelCard from "@/components/TunnelCard.vue";
import StatusDot from "@/components/StatusDot.vue";
import CopyButton from "@/components/CopyButton.vue";

const router = useRouter();
const store = useTunnelsStore();
const settings = useSettingsStore();
const { primary, list, live } = storeToRefs(store);

const others = computed(() =>
  primary.value ? list.value.filter((t) => t.id !== primary.value!.id) : [],
);

const stats = computed(() => [
  {
    label: "在线隧道",
    value: live.value.length,
    suffix: live.value.length === 1 ? "个会话" : `${live.value.length} 个会话`,
  },
  {
    label: "活跃端口",
    value: new Set(live.value.map((t) => t.localPort)).size,
    suffix: "使用中",
  },
  {
    label: "cloudflared",
    value: settings.cloudflared?.installed ? "已就绪" : "未安装",
    suffix:
      settings.cloudflared?.path?.split("/").slice(-1)[0] ?? "需要安装",
  },
]);

const greeting = ref(
  (() => {
    const h = new Date().getHours();
    if (h < 5) return "凌晨好";
    if (h < 12) return "早上好";
    if (h < 18) return "下午好";
    return "晚上好";
  })(),
);

function stopTunnel(id: string) {
  void store.stop(id).catch((e) => {
    console.error("[tunnel] stop failed:", e);
  });
}

function removeTunnel(id: string) {
  void store.remove(id).catch((e) => {
    console.error("[tunnel] remove failed:", e);
  });
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-8 py-7">
    <!-- 顶部 -->
    <header class="flex items-end justify-between">
      <div>
        <p class="text-xs font-medium uppercase tracking-wider text-dim">
          Quickflare
        </p>
        <h1 class="mt-1.5 text-3xl font-semibold leading-tight text-primary">
          {{ greeting }}
        </h1>
        <p class="mt-1.5 max-w-md text-sm text-muted">
          为本机的任意服务一键生成公网 HTTPS 地址，无需打开终端。
        </p>
      </div>

      <button class="btn btn-primary" @click="router.push('/create')">
        <svg
          viewBox="0 0 24 24"
          class="h-4 w-4"
          stroke="currentColor"
          fill="none"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 5v14M5 12h14" />
        </svg>
        新建隧道
      </button>
    </header>

    <!-- 统计 -->
    <div class="grid grid-cols-3 gap-3">
      <div
        v-for="stat in stats"
        :key="stat.label"
        class="surface flex items-center justify-between rounded-lg px-4 py-3"
      >
        <div>
          <p class="text-xs font-medium text-dim">{{ stat.label }}</p>
          <p class="mt-0.5 text-2xl font-semibold text-primary">
            {{ stat.value }}
          </p>
        </div>
        <p class="mono max-w-[110px] truncate text-right text-xs text-muted">
          {{ stat.suffix }}
        </p>
      </div>
    </div>

    <!-- 主隧道 / 空状态 -->
    <div v-if="primary">
      <TunnelCard :tunnel="primary" />
    </div>
    <EmptyState
      v-else
      title="还没有隧道"
      description="从端口扫描中选一个，或在「创建隧道」里输入端口号 — Quickflare 会自动启动 cloudflared 并返回公网地址。"
    >
      <div class="mt-5 flex gap-2">
        <button class="btn btn-primary" @click="router.push('/create')">
          创建第一个隧道
        </button>
        <button class="btn" @click="router.push('/scanner')">扫描端口</button>
      </div>
    </EmptyState>

    <!-- 其他会话 -->
    <section v-if="others.length > 0" class="space-y-3">
      <h2 class="text-base font-semibold text-primary">其他会话</h2>
      <div class="grid grid-cols-2 gap-3">
        <article
          v-for="t in others"
          :key="t.id"
          class="surface animate-fade-up flex flex-col gap-3 rounded-lg p-4"
        >
          <div class="flex items-center justify-between">
            <StatusDot :status="t.status" label />
            <span class="mono text-xs text-dim">:{{ t.localPort }}</span>
          </div>

          <p class="mono truncate text-sm text-primary">
            {{ t.publicUrl ?? "正在打开…" }}
          </p>

          <div class="flex items-center justify-between border-t hairline pt-3">
            <button
              class="btn btn-ghost"
              @click="stopTunnel(t.id)"
              v-if="t.status === 'live' || t.status === 'starting'"
            >
              停止
            </button>
            <button class="btn btn-ghost" @click="removeTunnel(t.id)" v-else>
              移除
            </button>
            <CopyButton v-if="t.publicUrl" :text="t.publicUrl" />
          </div>
        </article>
      </div>
    </section>
  </section>
</template>
