/**
 * 主题切换：浅色 ↔ 深色。
 *
 * 默认浅色。用户切换后写入 localStorage，下次启动时由
 * index.html 顶部的内联脚本同步读取（避免首帧闪烁）。
 */
import { ref, watch } from "vue";

export type Theme = "light" | "dark";

const STORAGE_KEY = "quickflare:theme";

function detectInitial(): Theme {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {
    /* SSR / 隔离环境 */
  }
  return "light";
}

const theme = ref<Theme>(detectInitial());

function apply(next: Theme) {
  const cls = document.documentElement.classList;
  if (next === "dark") cls.add("dark");
  else cls.remove("dark");
}

apply(theme.value);

watch(theme, (next) => {
  apply(next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* 存储不可用时静默忽略 */
  }
});

export function useTheme() {
  return {
    theme,
    toggle() {
      theme.value = theme.value === "light" ? "dark" : "light";
    },
    set(next: Theme) {
      theme.value = next;
    },
  };
}
