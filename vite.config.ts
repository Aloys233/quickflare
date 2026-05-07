import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import path from "node:path";

// Tauri expects a fixed port and ignores the dev server URL during prod builds.
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  base: process.env.TAURI_ENV_PLATFORM ? "./" : "/",

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Vite options tailored for Tauri development
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tauri rebuilds on Rust changes, so we don't need Vite to also watch them.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Env variables prefixed with VITE_ are exposed to the client.
  envPrefix: ["VITE_", "TAURI_ENV_*"],

  build: {
    // Match the targets supported by Tauri 2's WebView pool.
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
