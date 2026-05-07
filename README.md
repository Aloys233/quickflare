# Quickflare

> A native-feeling Cloudflare Tunnel manager for Linux, macOS and Windows.

Quickflare turns any local port into a public HTTPS URL through Cloudflare
Tunnel — without ever touching the terminal. It scans your machine for
running services, identifies the framework (Vite, Next.js, Spring Boot,
Docker, …), and pins a `*.trycloudflare.com` address to whichever one you
pick. The whole thing lives in the system tray when you don't need it.

> Linux KDE Plasma + Wayland is the primary target — but it builds and
> ships clean on macOS and Windows with the same single binary.

---

## Features

- **Auto port scanner** — `netstat2` + `sysinfo` on every refresh,
  joined with a regex-based service classifier (Vite, Next.js, Nuxt,
  Spring Boot, Docker, Postgres, Redis, Mongo, Minecraft, …).
- **One-click tunneling** — pick a port, get a public URL. We supervise
  the `cloudflared` child, parse stdout for the URL, and surface it the
  moment it's available.
- **System tray** — Open Dashboard / Create / Stop / Copy URL /
  Recent Ports / Quit. Built on `core:tray` so the same code runs on
  Plasma, GNOME, macOS and Windows.
- **Crash recovery** — auto-restart with a windowed back-off, capped at
  5 attempts/minute.
- **Persisted state** — `tauri-plugin-store` keeps recent ports, recent
  URLs and the user's settings under the platform config dir.
- **Pluggable providers** — `TunnelProvider` trait. Cloudflared ships
  today; ngrok / Pinggy / Tailscale Funnel slot in without changes to
  the manager.

## Project layout

```
.
├── package.json            ← Vue 3 + Vite + TS + Tailwind v3
├── vite.config.ts          ← Tauri-aware Vite config
├── tailwind.config.js      ← "Atmospheric Operator" design tokens
├── index.html
├── src/                    ← Frontend
│   ├── main.ts
│   ├── App.vue
│   ├── style.css
│   ├── components/         ← StatusDot, TunnelCard, PortRow, …
│   ├── views/              ← Dashboard, Scanner, Create, Settings
│   ├── stores/             ← Pinia: tunnels, scanner, settings
│   ├── composables/        ← useTauriBridge.ts (typed invoke wrapper)
│   ├── router/
│   └── types/              ← TS mirror of Rust types
└── src-tauri/              ← Backend
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── capabilities/default.json
    ├── icons/              ← (drop your generated icon-set here)
    └── src/
        ├── main.rs         ← thin shim
        ├── lib.rs          ← composition root
        ├── error.rs        ← AppError + AppResult
        ├── types.rs        ← shared serde types
        ├── store.rs        ← typed wrapper over plugin-store
        ├── tray.rs         ← system tray menu + events
        ├── commands/       ← #[tauri::command] surface
        │   ├── scanner.rs
        │   ├── tunnel.rs
        │   └── system.rs
        ├── services/
        │   ├── port_scanner.rs
        │   ├── process_detector.rs
        │   └── tunnel_manager.rs   ← child-process supervisor
        └── providers/
            ├── traits.rs            ← TunnelProvider + ProviderRegistry
            └── cloudflared.rs       ← Cloudflare Quick Tunnel adapter
```

## Prerequisites

| Tool         | Version           | Notes                                        |
| ------------ | ----------------- | -------------------------------------------- |
| Rust         | 1.77+             | `rustup default stable`                      |
| Node.js      | 20+               | only used for Vite / build tooling           |
| `cloudflared`| current           | install once; Quickflare auto-detects PATH   |

### Linux (Debian/Ubuntu)

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl wget file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Linux (Arch / KDE Plasma)

```bash
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl wget file \
  openssl \
  libayatana-appindicator \
  librsvg
```

### macOS

```bash
xcode-select --install
brew install cloudflared
```

### Windows

Install [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/),
then `winget install Cloudflare.cloudflared`.

## Develop

```bash
npm install
npm run tauri:dev
```

The first build pulls Tauri 2 + plugins from crates.io — give it a few
minutes the first time. Subsequent rebuilds are incremental.

## Build a release binary

```bash
npm run tauri:build
```

Artefacts land under `src-tauri/target/release/bundle/`:

- Linux: `.deb`, `.rpm`, `.AppImage`
- macOS: `.dmg`, `.app`
- Windows: `.msi`, `.exe`

## Icon set

Drop your branded PNG / ICO / ICNS files into `src-tauri/icons/`. The
sizes referenced by `tauri.conf.json` are the canonical Tauri set —
generate them with the official tool:

```bash
npx @tauri-apps/cli icon path/to/source.png
```

The `tray.png` should be a **monochrome 22×22** PNG; we set
`iconAsTemplate: true` so macOS automatically inverts it for the menu
bar's appearance.

## Wayland / KDE Plasma notes

- Quickflare uses `tauri-plugin-os` so it can adapt the tray code path
  per-DE if needed in the future. Today's tray works on Plasma 5.27+
  out of the box thanks to the StatusNotifierItem support.
- We disable native window decorations (`decorations: false`) and ship
  our own minimal title bar — keeps the typography consistent with the
  rest of the UI under both `kwin_x11` and `kwin_wayland`.
- If you're on a tiling compositor (sway, hyprland) without a built-in
  SNI host, install one of: `waybar`, `swaync`, `i3status-rust`, or
  use KDE's `plasma-systemtray` from a panel.

## Roadmap (the architecture is already wired for these)

- [ ] Authenticated **named tunnels** & custom domains via Cloudflare API
- [ ] Cloudflare **Zero Trust / Access** policies on the tunnel
- [ ] **ngrok**, **Pinggy**, **Tailscale Funnel** providers
- [ ] **Docker auto-discovery** — listen on the docker socket
- [ ] In-app **traffic graph** (the manager already buffers per-tunnel
      log lines; it just needs an event meter)
- [ ] **Tunnel history** browser

## License

MIT.
