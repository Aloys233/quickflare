<script setup lang="ts">
/**
 * 左侧导航 — 64px 宽，图标优先，活跃项有左侧高亮条。
 * 底部固定一个主题切换按钮。
 */
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useTunnelsStore } from "@/stores/tunnels";
import ThemeToggle from "@/components/ThemeToggle.vue";

const route = useRoute();
const router = useRouter();
const tunnels = useTunnelsStore();

const items = [
  {
    name: "dashboard",
    label: "控制台",
    path: "/",
    icon: "M3 12 12 4l9 8M5 10v10h14V10",
  },
  {
    name: "scanner",
    label: "端口扫描",
    path: "/scanner",
    icon: "M11 4a7 7 0 1 0 4.95 11.95l4.55 4.55-1.41 1.41-4.55-4.55A7 7 0 0 0 11 4Zm0 2a5 5 0 1 1 0 10 5 5 0 0 1 0-10Z",
  },
  {
    name: "create",
    label: "创建隧道",
    path: "/create",
    icon: "M12 5v14M5 12h14",
  },
  {
    name: "logs",
    label: "日志",
    path: "/logs",
    icon: "M4 6h16M4 12h10M4 18h16",
  },
  {
    name: "settings",
    label: "设置",
    path: "/settings",
    icon: "M19.4 13.5a7.5 7.5 0 0 0 0-3l2.1-1.6-2-3.4-2.5.9a7.6 7.6 0 0 0-2.6-1.5L14 2h-4l-.4 2.9c-.9.4-1.8.9-2.6 1.5l-2.5-.9-2 3.4 2.1 1.6a7.5 7.5 0 0 0 0 3L2.5 15.1l2 3.4 2.5-.9c.8.6 1.7 1.1 2.6 1.5L10 22h4l.4-2.9c.9-.4 1.8-.9 2.6-1.5l2.5.9 2-3.4-2.1-1.6ZM12 15a3 3 0 1 1 0-6 3 3 0 0 1 0 6Z",
  },
];

const liveCount = computed(() => tunnels.live.length);
</script>

<template>
  <nav
    class="flex w-[64px] flex-col items-center gap-1 border-r border-[var(--border)] bg-[var(--bg-elev)] py-3"
  >
    <!-- 品牌图标 -->
    <div class="mb-2 grid h-9 w-9 place-items-center rounded-lg bg-brand">
      <span class="text-base font-semibold text-white">Q</span>
    </div>

    <button
      v-for="item in items"
      :key="item.name"
      @click="router.push(item.path)"
      :title="item.label"
      :aria-label="item.label"
      class="relative grid h-10 w-10 place-items-center rounded-lg text-muted transition hover:text-primary"
      :class="[
        route.name === item.name
          ? 'bg-brand-soft text-brand dark:bg-brand-softDark dark:text-brand'
          : 'hover:bg-black/5 dark:hover:bg-white/5',
      ]"
    >
      <span
        v-if="route.name === item.name"
        class="absolute -left-2 top-1/2 h-5 w-[2.5px] -translate-y-1/2 rounded-r-full bg-brand"
      />
      <svg
        viewBox="0 0 24 24"
        class="h-5 w-5"
        stroke="currentColor"
        fill="none"
        stroke-width="1.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path :d="item.icon" />
      </svg>

      <!-- 控制台徽标：有活跃隧道时显示 -->
      <span
        v-if="item.name === 'dashboard' && liveCount > 0"
        class="absolute right-1 top-1 h-1.5 w-1.5 animate-breathe rounded-full bg-live"
      />
    </button>

    <div class="flex-1" />

    <!-- 底部：主题切换 -->
    <ThemeToggle />
  </nav>
</template>
