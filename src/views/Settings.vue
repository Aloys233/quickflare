<script setup lang="ts">
/**
 * 设置 — 偏好项 + cloudflared 路径。
 */
import { ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "@/stores/settings";

const store = useSettingsStore();
const tokenInput = ref("");

const local = ref({ ...store.settings });
watch(
  () => store.settings,
  (v) => (local.value = { ...v }),
);

async function pickBinary() {
  const file = await openDialog({
    multiple: false,
    title: "选择 cloudflared 二进制",
  });
  if (typeof file === "string") {
    await store.update({ cloudflaredPath: file });
  }
}

async function clearBinary() {
  await store.update({ cloudflaredPath: null });
}

function commit<K extends keyof typeof local.value>(
  key: K,
  val: (typeof local.value)[K],
) {
  store.update({ [key]: val } as Partial<typeof store.settings>);
}

function commitText<K extends keyof typeof local.value>(
  key: K,
  val: string,
) {
  const normalized = val.trim() || null;
  store.update({ [key]: normalized } as Partial<typeof store.settings>);
}

async function saveToken() {
  if (!tokenInput.value.trim()) return;
  await store.saveTunnelToken(tokenInput.value);
  tokenInput.value = "";
}

async function clearToken() {
  await store.clearTunnelToken();
  tokenInput.value = "";
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-8 py-7">
    <header>
      <p class="text-xs font-medium uppercase tracking-wider text-dim">
        偏好
      </p>
      <h1 class="mt-1.5 text-2xl font-semibold leading-tight text-primary">
        设置
      </h1>
    </header>

    <!-- cloudflared 路径 -->
    <section class="surface rounded-xl p-6">
      <header class="flex items-center justify-between">
        <div>
          <h2 class="text-base font-semibold text-primary">cloudflared</h2>
          <p class="mt-1 max-w-md text-sm text-muted">
            Quickflare 使用官方的 cloudflared CLI。在此覆盖路径可跳过 PATH
            查找，直接使用打包的二进制。
          </p>
        </div>
        <span
          v-if="store.cloudflared?.installed"
          class="pill text-live"
          style="border-color: rgba(16, 185, 129, 0.4)"
        >
          已就绪
        </span>
        <span
          v-else
          class="pill text-red-600 dark:text-red-400"
          style="border-color: rgba(239, 68, 68, 0.4)"
        >
          未安装
        </span>
      </header>
      <div
        class="mono mt-4 break-all rounded-md border hairline bg-[var(--bg)] px-3 py-2.5 text-sm"
      >
        {{ store.cloudflared?.path ?? "PATH 中未找到" }}
      </div>
      <div class="mt-3 flex gap-2">
        <button class="btn" @click="pickBinary">选择二进制…</button>
        <button
          v-if="store.settings.cloudflaredPath"
          class="btn btn-ghost"
          @click="clearBinary"
        >
          清除覆盖
        </button>
      </div>
    </section>

    <!-- Cloudflare Tunnel token -->
    <section class="surface rounded-xl p-6">
      <header>
        <h2 class="text-base font-semibold text-primary">自定义域名</h2>
        <p class="mt-1 max-w-2xl text-sm text-muted">
          填入 Cloudflare Tunnel token 后，Quickflare 会用 token 运行正式隧道；域名需要已在 Cloudflare 中路由到该 tunnel。
        </p>
      </header>

      <div class="mt-4 grid gap-4 md:grid-cols-2">
        <label class="flex flex-col gap-2">
          <span class="text-xs font-medium uppercase tracking-wider text-dim">
            Tunnel token
          </span>
          <input
            class="input-text"
            type="password"
            autocomplete="off"
            spellcheck="false"
            v-model="tokenInput"
            placeholder="eyJhIjoi..."
          />
          <div class="flex items-center gap-2">
            <button class="btn" type="button" :disabled="!tokenInput.trim()" @click="saveToken">
              保存到密钥环
            </button>
            <button
              v-if="store.tunnelToken.saved"
              class="btn btn-ghost"
              type="button"
              @click="clearToken"
            >
              清除 token
            </button>
            <span class="text-xs text-muted">
              {{ store.tunnelToken.saved ? "已保存" : "未保存" }}
            </span>
          </div>
        </label>

        <label class="flex flex-col gap-2">
          <span class="text-xs font-medium uppercase tracking-wider text-dim">
            公网域名
          </span>
          <input
            class="input-text"
            type="text"
            autocomplete="off"
            spellcheck="false"
            :value="local.customHostname ?? ''"
            placeholder="app.example.com"
            @change="
              commitText(
                'customHostname',
                ($event.target as HTMLInputElement).value,
              )
            "
          />
        </label>
      </div>

      <p class="mt-3 text-xs text-muted">
        token 存在系统密钥环中，不写入 quickflare.json；未保存 token 时继续使用随机 trycloudflare.com。
      </p>
    </section>

    <!-- 行为 -->
    <section class="surface rounded-xl p-6">
      <h2 class="text-base font-semibold text-primary">行为</h2>
      <ul class="mt-3 divide-y divide-[var(--border)]">
        <li class="flex items-center justify-between py-3">
          <div class="pr-4">
            <p class="text-sm font-medium text-primary">自动重启崩溃的隧道</p>
            <p class="mt-0.5 text-xs text-muted">
              一分钟内最多重启 5 次，超过后将上报错误。
            </p>
          </div>
          <input
            type="checkbox"
            class="accent-brand"
            :checked="local.autoRestart"
            @change="
              commit('autoRestart', ($event.target as HTMLInputElement).checked)
            "
          />
        </li>
        <li class="flex items-center justify-between py-3">
          <div class="pr-4">
            <p class="text-sm font-medium text-primary">关闭时隐藏到托盘</p>
            <p class="mt-0.5 text-xs text-muted">
              关闭窗口后隧道继续运行；右键托盘图标可重新打开主界面。
            </p>
          </div>
          <input
            type="checkbox"
            class="accent-brand"
            :checked="local.closeToTray"
            @change="
              commit('closeToTray', ($event.target as HTMLInputElement).checked)
            "
          />
        </li>
        <li class="flex items-center justify-between py-3">
          <div class="pr-4">
            <p class="text-sm font-medium text-primary">开机启动</p>
            <p class="mt-0.5 text-xs text-muted">
              调用系统的开机启动机制 — Linux 上是 XDG，macOS 上是登录项，Windows
              上是 Run 注册项。
            </p>
          </div>
          <input
            type="checkbox"
            class="accent-brand"
            :checked="local.launchAtLogin"
            @change="
              commit(
                'launchAtLogin',
                ($event.target as HTMLInputElement).checked,
              )
            "
          />
        </li>
        <li class="flex items-center justify-between py-3">
          <div class="pr-4">
            <p class="text-sm font-medium text-primary">扫描间隔</p>
            <p class="mt-0.5 text-xs text-muted">
              多久重新探测一次监听端口。值越小越灵敏，CPU 占用也越高。
            </p>
          </div>
          <select
            class="input-text"
            :value="local.scanIntervalSeconds"
            @change="
              commit(
                'scanIntervalSeconds',
                Number(($event.target as HTMLSelectElement).value),
              )
            "
          >
            <option :value="3">3 秒</option>
            <option :value="5">5 秒</option>
            <option :value="10">10 秒</option>
            <option :value="30">30 秒</option>
          </select>
        </li>
      </ul>
    </section>

    <footer class="text-center text-xs text-dim">
      <p class="text-sm font-medium text-primary">Quickflare</p>
      <p class="mt-1">v0.1.0 · 为 Linux 优先而生 · Wayland 友好</p>
    </footer>
  </section>
</template>
