<script setup lang="ts">
/**
 * 复制按钮 — 走 tauri-plugin-clipboard-manager 而不是浏览器 API，
 * 在 Wayland 上更稳。
 */
import { ref } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const props = defineProps<{
  text: string;
  label?: string;
}>();

const copied = ref(false);
async function copy() {
  await writeText(props.text);
  copied.value = true;
  window.setTimeout(() => (copied.value = false), 1400);
}
</script>

<template>
  <button class="btn" @click="copy" :title="`复制 ${text}`">
    <svg
      v-if="!copied"
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
    <svg
      v-else
      viewBox="0 0 24 24"
      class="h-4 w-4 text-live"
      stroke="currentColor"
      fill="none"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="m5 12 5 5L20 7" />
    </svg>
    <span>{{ copied ? "已复制" : (label ?? "复制") }}</span>
  </button>
</template>
