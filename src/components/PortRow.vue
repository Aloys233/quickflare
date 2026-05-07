<script setup lang="ts">
/**
 * 端口扫描表格的一行。左边一道色条暗示服务种类，悬停时露出操作按钮。
 */
import { computed } from "vue";
import { useRouter } from "vue-router";
import StatusDot from "@/components/StatusDot.vue";
import type { ListeningPort, ServiceKind } from "@/types";

const props = defineProps<{ port: ListeningPort; isInUse: boolean }>();
const router = useRouter();

const SERVICE_LABEL: Record<ServiceKind, string> = {
  vite: "Vite",
  "next-js": "Next.js",
  nuxt: "Nuxt",
  "spring-boot": "Spring Boot",
  "node-js": "Node.js",
  python: "Python",
  docker: "Docker",
  ssh: "SSH",
  minecraft: "Minecraft",
  postgres: "Postgres",
  mysql: "MySQL",
  redis: "Redis",
  mongo: "MongoDB",
  elastic: "Elasticsearch",
  http: "HTTP",
  unknown: "未知",
};

const SERVICE_ACCENT: Record<ServiceKind, string> = {
  vite: "bg-violet-500",
  "next-js": "bg-slate-900 dark:bg-slate-100",
  nuxt: "bg-emerald-500",
  "spring-boot": "bg-lime-500",
  "node-js": "bg-emerald-400",
  python: "bg-blue-500",
  docker: "bg-cyan-500",
  ssh: "bg-rose-500",
  minecraft: "bg-green-600",
  postgres: "bg-blue-600",
  mysql: "bg-orange-500",
  redis: "bg-red-500",
  mongo: "bg-emerald-600",
  elastic: "bg-yellow-500",
  http: "bg-amber-500",
  unknown: "bg-slate-400",
};

const accent = computed(() => SERVICE_ACCENT[props.port.service]);
const serviceLabel = computed(() => SERVICE_LABEL[props.port.service]);

function tunnel() {
  router.push({ path: "/create", query: { port: props.port.port } });
}
</script>

<template>
  <div
    class="group surface-flat relative grid grid-cols-[80px_1fr_1fr_120px_auto] items-center gap-4 rounded-lg px-4 py-3 transition hover:bg-black/[0.02] dark:hover:bg-white/[0.02]"
  >
    <span
      class="absolute inset-y-2 left-0 w-[3px] rounded-r-full"
      :class="accent"
    />

    <!-- 端口号 -->
    <div class="mono text-lg font-semibold text-primary">
      {{ port.port }}
    </div>

    <!-- 服务 -->
    <div class="flex flex-col">
      <span class="text-sm font-medium text-primary">{{ serviceLabel }}</span>
      <span class="mono text-xs text-dim">{{ port.address }}</span>
    </div>

    <!-- 进程 -->
    <div class="flex flex-col min-w-0">
      <span class="mono truncate text-xs text-muted">
        {{ port.process ?? "—" }}
      </span>
      <span class="mono text-xs text-dim">
        PID {{ port.pid ?? "—" }}
      </span>
    </div>

    <!-- 状态 -->
    <div>
      <StatusDot :status="isInUse ? 'live' : 'idle'" label />
    </div>

    <!-- 打通按钮 -->
    <button
      class="btn opacity-0 transition group-hover:opacity-100"
      @click="tunnel"
    >
      <svg
        viewBox="0 0 24 24"
        class="h-4 w-4 text-brand"
        stroke="currentColor"
        fill="none"
        stroke-width="1.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M5 12h14" />
        <path d="m13 6 6 6-6 6" />
      </svg>
      打通
    </button>
  </div>
</template>
