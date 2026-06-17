# Track all your AI coding subscriptions in one place

See your usage at a glance from your system tray. No digging through dashboards.

> **This is the Windows port of [OpenUsage](https://github.com/robinebers/openusage)** by
> [Robin Ebers](https://github.com/robinebers). The upstream project is a macOS
> menu-bar app built with Tauri; this repo carries that same Tauri edition to
> Windows, keeping the UI, UX, providers, and behavior identical and changing
> only what is fundamentally different between the two operating systems. See
> [What's different on Windows](#whats-different-on-windows) for the specifics.

![OpenUsage Screenshot](screenshot.png)

## Download

[**Download the latest release**](https://github.com/mesomya/openusage-windows/releases/latest) (Windows 10/11, 64-bit)

The app auto-updates. Install once and you're set. (Windows 11 ships the WebView2
runtime the app needs; on Windows 10 the installer will pull it in if missing.)

## What It Does

OpenUsage lives in your system tray and shows you how much of your AI coding
subscriptions you've used. Progress bars, badges, and clear labels. No mental
math required.

- **One glance.** All your AI tools, one panel.
- **Always up-to-date.** Refreshes automatically on a schedule you pick.
- **Global shortcut.** Toggle the panel from anywhere with a customizable keyboard shortcut.
- **Lightweight.** Opens instantly, stays out of your way.
- **Plugin-based.** New providers get added without updating the whole app.
- **[Local HTTP API](docs/local-http-api.md).** Other apps can read your usage data from `127.0.0.1:6736`.
- **[Proxy support](docs/proxy.md).** Route provider HTTP requests through a SOCKS5 or HTTP proxy.

## Supported Providers

- [**Amp**](docs/providers/amp.md) / free tier, bonus, credits
- [**Antigravity**](docs/providers/antigravity.md) / all models
- [**Claude**](docs/providers/claude.md) / session, weekly, extra usage, local token usage (ccusage)
- [**Codex**](docs/providers/codex.md) / session, weekly, reviews, credits
- [**Copilot**](docs/providers/copilot.md) / premium, chat, completions
- [**Cursor**](docs/providers/cursor.md) / credits, total usage, auto usage, API usage, on-demand, CLI auth
- [**Factory / Droid**](docs/providers/factory.md) / standard, premium tokens
- [**Grok**](docs/providers/grok.md) / credits used, plan, pay-as-you-go cap
- [**JetBrains AI Assistant**](docs/providers/jetbrains-ai-assistant.md) / quota, remaining
- [**Kiro**](docs/providers/kiro.md) / credits, bonus credits, overages
- [**Kimi Code**](docs/providers/kimi.md) / session, weekly
- [**MiniMax**](docs/providers/minimax.md) / coding plan session
- [**OpenCode Go**](docs/providers/opencode-go.md) / 5h, weekly, monthly spend limits
- [**Devin**](docs/providers/devin.md) / weekly quota, extra usage
- [**Z.ai**](docs/providers/zai.md) / session, weekly, web searches

Want a provider that's not listed? [Open an issue.](https://github.com/mesomya/openusage-windows/issues/new)

## What's different on Windows

The app is the same Tauri (Rust + web UI) codebase as upstream. Only the pieces
that are genuinely OS-specific were changed:

- **The panel.** macOS uses a native `NSPanel` floating below the menu bar.
  Windows has no equivalent, so the panel is a borderless, transparent,
  always-on-top window anchored just above the system-tray icon that hides when
  it loses focus. Same look and behavior.
- **SQLite reads.** Providers that read a SQLite database used the macOS
  `sqlite3` command line. Windows doesn't ship one, so reads now use an embedded
  SQLite engine — no external binary on any platform.
- **Process / port discovery.** The language-server discovery used `ps`/`lsof`;
  on Windows it uses PowerShell process enumeration plus `netstat`.
- **`ccusage` (local token counts).** The `npx`/`bunx` runners are launched
  through `cmd /C` on Windows (with the console window suppressed), since `.cmd`
  shims can't be executed directly.
- **Refresh model.** macOS keeps the hidden webview's JS alive (via a WebKit
  scheduling tweak), so usage refreshes continuously in the background. Windows
  WebView2 suspends a fully hidden window's scripts, so the panel refreshes when
  you open it (and auto-refreshes while open); the tray reflects the last check.
  WebView2 background-throttling flags are set so refresh stays responsive while
  the panel is open but unfocused.
- **TLS.** Uses the native OS TLS stack (SChannel on Windows) instead of
  bundling a C crypto library.
- **No telemetry.** The upstream's anonymous analytics were removed from this
  fork.
- **Antigravity sign-in.** Upstream embeds Google's Antigravity OAuth client
  credentials in the plugin; this public fork does not republish that
  third-party secret, so Antigravity sign-in needs those two values supplied in
  a local build (copy them from upstream). Every other provider is unaffected.

App Nap handling, the macOS activation policy, and other macOS-only code are
compiled only on macOS, so they simply don't exist in the Windows build.

## Credits

This is an unofficial Windows port of **[OpenUsage](https://github.com/robinebers/openusage)**
by **[Robin Ebers](https://github.com/robinebers)** — all the original design,
providers, and ideas are his. Upstream is itself inspired by
[CodexBar](https://github.com/steipete/CodexBar) by
[@steipete](https://github.com/steipete).

"OpenUsage" may be a trademark of the upstream author; this fork uses the name
only to describe what it is and is not an official build.

## License

[MIT](LICENSE) — same as upstream.

---

<details>
<summary><strong>Build from source (Windows)</strong></summary>

### Prerequisites

- **Rust** (stable, MSVC toolchain): install via [rustup](https://rustup.rs/),
  then `rustup default stable-x86_64-pc-windows-msvc`.
- **Visual Studio Build Tools** with the "Desktop development with C++" workload
  (provides the MSVC compiler/linker the Rust and SQLite/QuickJS C code need).
- **Node.js 20+** (ships `npm`).
- **WebView2 runtime** — preinstalled on Windows 11.

### Build & run

```powershell
npm install
npm run tauri dev      # run the app with hot-reloading
```

### Produce an installer

```powershell
npm run tauri build --bundles nsis   # outputs an NSIS installer under src-tauri/target/release/bundle/
```

### Signed auto-update releases (optional)

Generate an updater key pair once and keep the private key secret:

```powershell
npm run tauri signer generate -- -w openusage.key
```

Put the public key in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) and
add the private key as the `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret
(plus `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Pushing a `vX.Y.Z` tag then builds,
signs, and publishes a release via `.github/workflows/publish.yml`.

</details>
