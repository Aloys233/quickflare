/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{vue,ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        // 普通的现代无衬线 + 系统中文字体回退。
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "PingFang SC",
          "Microsoft YaHei",
          "Noto Sans CJK SC",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
        mono: [
          "JetBrains Mono",
          "SF Mono",
          "Menlo",
          "Consolas",
          "Courier New",
          "monospace",
        ],
      },
      colors: {
        // 品牌主色 — Cloudflare 橙。
        brand: {
          DEFAULT: "#F6821F",
          hover: "#E07414",
          soft: "#FFF4EB",
          softDark: "#3A1E08",
        },
        live: {
          DEFAULT: "#10B981",
          dark: "#059669",
          soft: "#ECFDF5",
        },
      },
      keyframes: {
        breathe: {
          "0%, 100%": { opacity: "0.5", transform: "scale(1)" },
          "50%": { opacity: "1", transform: "scale(1.15)" },
        },
        sweep: {
          "0%": { transform: "translateX(-110%)" },
          "100%": { transform: "translateX(110%)" },
        },
        "fade-up": {
          "0%": { opacity: "0", transform: "translateY(4px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
      animation: {
        breathe: "breathe 2.4s ease-in-out infinite",
        sweep: "sweep 1.8s cubic-bezier(0.4,0,0.2,1) infinite",
        "fade-up": "fade-up 0.3s cubic-bezier(0.2,0.65,0.3,1) both",
      },
    },
  },
  plugins: [],
};
