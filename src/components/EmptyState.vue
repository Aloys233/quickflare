<script setup lang="ts">
/**
 * 空状态 — 控制台 / 扫描页在没有内容时使用。
 */
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div
    class="surface flex flex-col items-center justify-center rounded-xl px-8 py-12 text-center"
  >
    <!-- 简洁图标 -->
    <div
      class="mb-5 grid h-14 w-14 place-items-center rounded-full bg-brand-soft text-brand dark:bg-brand-softDark"
    >
      <svg
        viewBox="0 0 24 24"
        class="h-6 w-6"
        stroke="currentColor"
        fill="none"
        stroke-width="1.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M3 12h4l3-9 4 18 3-9h4" />
      </svg>
    </div>

    <h3 class="text-base font-semibold text-primary">{{ title }}</h3>
    <p v-if="description" class="mt-1.5 max-w-sm text-sm text-muted">
      {{ description }}
    </p>
    <slot />
  </div>
</template>
