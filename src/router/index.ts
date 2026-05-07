import { createRouter, createWebHashHistory } from "vue-router";
import Dashboard from "@/views/Dashboard.vue";

/**
 * Hash 路由 — Tauri 单窗口下足够用，并且托盘菜单的深链能稳定工作。
 *
 * Dashboard 改为同步 import，避免首屏 lazy chunk 还在下载时显示空白。
 * 其他页面继续 lazy 以减小初始 JS 体积。
 */
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "dashboard",
      component: Dashboard,
    },
    {
      path: "/scanner",
      name: "scanner",
      component: () => import("@/views/Scanner.vue"),
    },
    {
      path: "/create",
      name: "create",
      component: () => import("@/views/Create.vue"),
      props: (route) => ({
        prefilledPort: route.query.port
          ? Number(route.query.port)
          : undefined,
      }),
    },
    {
      path: "/logs",
      name: "logs",
      component: () => import("@/views/Logs.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/Settings.vue"),
    },
    // 兜底：任何未匹配的路径都回到 dashboard
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});

export default router;
