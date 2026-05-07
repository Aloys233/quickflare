<script setup lang="ts">
/**
 * 主题切换按钮 — 太阳 / 月亮。
 */
import { useTheme } from "@/composables/useTheme";

const { theme, toggle } = useTheme();
</script>

<template>
  <button
    class="grid h-10 w-10 place-items-center rounded-lg text-muted transition hover:bg-black/5 hover:text-primary dark:hover:bg-white/5"
    :title="theme === 'light' ? '切换到深色模式' : '切换到浅色模式'"
    :aria-label="theme === 'light' ? '切换到深色模式' : '切换到浅色模式'"
    @click="toggle"
  >
    <svg
      v-if="theme === 'light'"
      viewBox="0 0 24 24"
      class="h-5 w-5"
      stroke="currentColor"
      fill="none"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="12" cy="12" r="4" />
      <path
        d="M12 2v2M12 20v2M2 12h2M20 12h2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
      />
    </svg>
    <svg
      v-else
      viewBox="0 0 24 24"
      class="h-5 w-5"
      stroke="currentColor"
      fill="none"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79Z" />
    </svg>
  </button>
</template>
