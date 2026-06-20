# OpenUsage for Windows

**See how much of your AI coding subscriptions you've used — at a glance, right from the Windows system tray. No digging through dashboards.**

[![Latest release](https://img.shields.io/github/v/release/mesomya/openusage-windows?label=Download&color=2ea44f)](https://github.com/mesomya/openusage-windows/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?logo=windows&logoColor=white)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-FFC131?logo=tauri&logoColor=white)](https://tauri.app)

> ### 🪟 A Windows port of [OpenUsage](https://github.com/robinebers/openusage) by [Robin Ebers](https://github.com/robinebers)
>
> [**OpenUsage**](https://github.com/robinebers/openusage) is a macOS menu-bar app that tracks your AI coding‑subscription usage. **This project is an unofficial, community‑built port of it to Windows** — it brings that same app to the Windows **system tray**, keeping the UI, UX, and every provider identical, and changing only what is fundamentally different between macOS and Windows.
>
> **All of the original design, the providers, the plugin system, and the idea are Robin Ebers' work** ([github.com/robinebers/openusage](https://github.com/robinebers/openusage)) — this repository only contains the changes needed to make it run on Windows. It is not affiliated with or endorsed by the original project.

![OpenUsage on Windows](screenshot.png)

## ⬇ Download & install

[**Download the latest release**](https://github.com/mesomya/openusage-windows/releases/latest) — Windows 10 / 11, 64‑bit.

1. Grab **`OpenUsage_x.y.z_x64-setup.exe`** from the [latest release](https://github.com/mesomya/openusage-windows/releases/latest) and run it.
2. Windows SmartScreen may warn that the publisher is unverified (the installer isn't code‑signed) — click **More info → Run anyway**.
3. The app lands in your **system tray** (bottom‑right, by the clock — click the `^` to find it the first time, then drag it onto the taskbar to pin it).
4. Press **Ctrl + Shift + U** anywhere, or click the tray icon, to open the panel.

The app **auto‑updates** itself, and can **start on login** (Settings → *Start on login*). Windows 11 already has the WebView2 runtime it needs; on Windows 10 the installer pulls it in if it's missing.

## What it does

OpenUsage lives in your system tray and shows how much of your AI coding subscriptions you've used — progress bars, badges, and clear labels. No mental math required.

- **One glance.** All your AI tools in one panel.
- **Always up‑to‑date.** Refreshes automatically on a schedule you pick, even while the panel is closed.
- **Global shortcut.** Toggle the panel from anywhere with a customizable keyboard shortcut.
- **Lightweight.** Opens instantly, stays out of your way, starts with Windows if you want.
- **Plugin‑based.** New providers can be added without rebuilding the whole app.
- **[Local HTTP API](docs/local-http-api.md).** Other apps can read your usage from `127.0.0.1:6736`.
- **[Proxy support](docs/proxy.md).** Route provider requests through a SOCKS5 or HTTP proxy.

## Supported providers

- [**Claude**](docs/providers/claude.md) — session, weekly, extra usage, local token usage (ccusage)
- [**Codex**](docs/providers/codex.md) — session, weekly, reviews, credits
- [**Cursor**](docs/providers/cursor.md) — credits, total / auto / API usage, on‑demand, CLI auth
- [**Copilot**](docs/providers/copilot.md) — premium, chat, completions
- [**Z.ai**](docs/providers/zai.md) — session, weekly, web searches
- [**Devin**](docs/providers/devin.md) — weekly quota, extra usage
- [**Factory / Droid**](docs/providers/factory.md) — standard, premium tokens
- [**Grok**](docs/providers/grok.md) — credits used, plan, pay‑as‑you‑go cap
- [**Amp**](docs/providers/amp.md) — free tier, bonus, credits
- [**Antigravity**](docs/providers/antigravity.md) — all models
- [**JetBrains AI Assistant**](docs/providers/jetbrains-ai-assistant.md) — quota, remaining
- [**Kiro**](docs/providers/kiro.md) — credits, bonus credits, overages
- [**Kimi Code**](docs/providers/kimi.md) — session, weekly
- [**MiniMax**](docs/providers/minimax.md) — coding‑plan session
- [**OpenCode Go**](docs/providers/opencode-go.md) — 5h, weekly, monthly spend limits

Each provider reads the login that its own app or CLI already saved on your machine — OpenUsage never asks you for a password. Providers are plugins; see the [plugin docs](docs/plugins/) for how they work.

## What's different on Windows

Same Tauri (Rust + web UI) codebase as upstream; only the genuinely OS‑specific pieces were changed:

- **The panel** — macOS uses a native `NSPanel` under the menu bar. Windows has no equivalent, so it's a borderless, transparent, always‑on‑top window anchored above the tray icon (and **draggable** — grab the bar at the top). Same look and behavior.
- **Credential paths** — providers that read a tool's local login (Cursor, Devin, Antigravity, Kiro, …) now look in the Windows locations (`%APPDATA%`/`%LOCALAPPDATA%`) instead of the macOS ones.
- **SQLite reads** — providers that read a SQLite DB used the macOS `sqlite3` CLI; Windows has none, so reads use an embedded SQLite engine (no external binary on any platform).
- **Process / port discovery** — language‑server discovery uses PowerShell + `netstat` instead of `ps`/`lsof`.
- **Background refresh** — a Windows WebView2 window suspends its scripts while hidden, so a small **Rust background loop** re‑probes your enabled providers on your interval even when the panel is closed, keeping the cache, the local HTTP API, and the tray tooltip up to date.
- **Tray icon** — a full‑color tray icon with a live tooltip (hover to glance), and a **single‑instance guard** so a second launch just focuses the running one.
- **TLS** — uses the native OS TLS stack (SChannel) instead of bundling a C crypto library.
- **No telemetry** — the upstream's anonymous analytics were removed.
- **Antigravity sign‑in** — upstream embeds Google's Antigravity OAuth client credentials in the plugin. Those belong to a third party, so this public repo does not republish that secret; Antigravity sign‑in needs those two values supplied in a local build. Every other provider is unaffected.

App Nap handling, the macOS activation policy, and other macOS‑only code are compiled only on macOS, so they don't exist in the Windows build.

## Credits & acknowledgements

This is an unofficial Windows port of **[OpenUsage](https://github.com/robinebers/openusage)** by **[Robin Ebers](https://github.com/robinebers)**. The design, the provider integrations, the plugin system, and the entire concept come from the original macOS project — full credit and thanks to Robin and the upstream contributors. This repository only holds the changes needed to run that work on Windows.

"OpenUsage" is the upstream project's name and brand, used here only to describe what this is. This is a personal/community build, shared as‑is — not officially affiliated with, endorsed by, or supported by the upstream project.

## License

[MIT](LICENSE). The original copyright notice is retained in [LICENSE](LICENSE) as the MIT license requires, alongside the copyright for the Windows‑port changes.

---

<details>
<summary><strong>🛠 Build from source (Windows)</strong></summary>

### Prerequisites

- **Rust** (stable, MSVC toolchain): install via [rustup](https://rustup.rs/), then `rustup default stable-x86_64-pc-windows-msvc`.
- **Visual Studio Build Tools** with the "Desktop development with C++" workload (the MSVC compiler/linker the Rust + SQLite/QuickJS C code need).
- **Node.js 20+** (ships `npm`).
- **WebView2 runtime** — preinstalled on Windows 11.

### Build & run

```powershell
npm install
npm run tauri dev          # run with hot-reloading
```

### Produce an installer

```powershell
npm run build:release      # NSIS installer under src-tauri/target/release/bundle/nsis/
```

### Tests

```powershell
npm test                                                   # frontend + plugin tests (vitest)
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1   # Rust tests
```

### Signed auto-update releases

Generate an updater key pair once and keep the private key secret:

```powershell
npm run tauri signer generate -- -w openusage.key
```

Put the public key in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) and add the private key as the `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret (plus `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Pushing a `vX.Y.Z` tag then builds, signs, and publishes a release via `.github/workflows/publish.yml`.

</details>
