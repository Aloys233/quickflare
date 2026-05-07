<script setup lang="ts">
/**
 * 状态指示点 — 隧道与端口状态的统一视觉来源。
 */
import { computed } from "vue";
import type { TunnelStatus } from "@/types";

const props = defineProps<{
  status: TunnelStatus | "idle";
  label?: boolean;
}>();

const STATUS_LABELS: Record<TunnelStatus | "idle", string> = {
  starting: "启动中",
  live: "已连接",
  stopping: "停止中",
  stopped: "已停止",
  crashed: "已崩溃",
  idle: "空闲",
};

const tone = computed(() => {
  switch (props.status) {
    case "live":
      return { color: "#10B981", isLive: true, text: "text-live" };
    case "starting":
      return { color: "#F6821F", isLive: false, breathe: true, text: "text-brand" };
    case "stopping":
      return { color: "#F6821F", isLive: false, text: "text-brand" };
    case "crashed":
      return { color: "#EF4444", isLive: false, text: "text-red-600 dark:text-red-400" };
    case "stopped":
    case "idle":
    default:
      return { color: "#94A3B8", isLive: false, text: "text-dim" };
  }
});
</script>

<template>
  <span class="inline-flex items-center gap-2">
    <span
      class="status-dot"
      :class="[tone.isLive && 'status-dot--live', tone.breathe && 'animate-breathe']"
      :style="{ backgroundColor: tone.color }"
    />
    <span v-if="label" class="text-xs font-medium" :class="tone.text">
      {{ STATUS_LABELS[status] }}
    </span>
  </span>
</template>
