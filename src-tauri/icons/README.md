# icons/

Drop the canonical Tauri icon set here. Generate it from a single
high-res source PNG with:

    npx @tauri-apps/cli icon path/to/your/source.png

The required filenames match `tauri.conf.json -> bundle.icon`:

- `32x32.png`
- `128x128.png`
- `128x128@2x.png`
- `icon.icns`            ← macOS
- `icon.ico`             ← Windows
- `tray.png`             ← monochrome 22×22 for the system tray

The repo ships a quick SVG mark at `public/quickflare.svg` that you can
use as a starting point until you have a finished design.
