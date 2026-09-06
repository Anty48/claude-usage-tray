<h1 align="center">Claude Usage Tray</h1>

<p align="center">
  <strong>Monitor your Claude Code usage directly from your Windows system tray or macOS menu bar.</strong>
</p>

<p align="center">
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS-2b2a27">
  <img alt="Built with" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%20v2-d97757">
  <a href="https://github.com/Anty48/claude-usage-tray/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Anty48/claude-usage-tray/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/Anty48/claude-usage-tray/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Anty48/claude-usage-tray?display_name=tag&color=6f9b6b"></a>
  <img alt="License" src="https://img.shields.io/badge/license-MIT-informational">
</p>

<p align="center">
  A tiny, on-demand utility that reads your local Claude&nbsp;Code session and shows how much of your
  <strong>5-hour</strong> and <strong>weekly</strong> usage is left — plus a
  <em>“Can my prompt finish?”</em> estimator. No polling, no stored secrets, prompts never leave your machine.
</p>

---

## ⬇️ Download

<table>
  <thead>
    <tr><th>Platform</th><th>Installer</th><th>Notes</th></tr>
  </thead>
  <tbody>
    <tr>
      <td>🪟 <strong>Windows</strong> 10 / 11</td>
      <td><a href="https://github.com/Anty48/claude-usage-tray/releases/latest"><code>ClaudeUsageTray-Setup.exe</code></a></td>
      <td>Per-user install, no admin needed</td>
    </tr>
    <tr>
      <td>🍎 <strong>macOS</strong> (Apple Silicon)</td>
      <td><a href="https://github.com/Anty48/claude-usage-tray/releases/latest"><code>ClaudeUsageTray-macOS-arm64.dmg</code></a></td>
      <td>M1 / M2 / M3 / M4</td>
    </tr>
    <tr>
      <td>🍎 <strong>macOS</strong> (Intel)</td>
      <td><a href="https://github.com/Anty48/claude-usage-tray/releases/latest"><code>ClaudeUsageTray-macOS-x64.dmg</code></a></td>
      <td>Intel Macs</td>
    </tr>
  </tbody>
</table>

```
┌──────────────────────────────────────────────┐
│            Download Claude Usage Tray          │
│                                                │
│     🪟  Windows            🍎  macOS           │
│     [ Setup.exe ]      [ arm64 / x64 .dmg ]    │
│                                                │
│      → github.com/Anty48/claude-usage-tray/releases/latest
└──────────────────────────────────────────────┘
```

> **Which one am I?** On **Windows** grab the `.exe`. On **macOS**, Apple-silicon Macs (2020+) use
> **arm64**; older Intel Macs use **x64**. If unsure on macOS, run `uname -m` — `arm64` → Apple
> Silicon, `x86_64` → Intel.

All installers are published as **GitHub Release assets** (never committed to the repo). Requires
[Claude Code](https://docs.claude.com/en/docs/claude-code/overview) installed and signed in on the
same device — this is a companion utility, see [How it works](#how-it-gets-the-data).

---

## ✨ Features

- **Live usage at a glance** — current **5-hour session** and **weekly** utilization, how much is
  left, when each window resets, and per-model breakdowns.
- **Traffic-light tray/menu-bar icon** — the gauge greens → ambers → **reddens** as your budget is
  spent, so you can read your status without even opening the popup.
- **On-demand only** — nothing runs in the background. It fetches when you click, then goes idle.
- **“Can my prompt finish?” calculator** — honest **ranges + confidence**, never a fake exact number.
- **Learns from your real history (locally)** — optional calibration from your own transcripts.
- **Native on both platforms** — Windows **system tray**, macOS **menu-bar** utility (no Dock icon).
- **Premium, minimal UI** — warm, Anthropic-inspired design with **System / Light / Dark** themes
  that follow the OS.

---

## 🖥️ Windows & 🍎 macOS

Same experience, native conventions on each OS:

| | Windows | macOS |
| --- | --- | --- |
| Lives in | Notification area (system tray) | Menu bar (no Dock icon) |
| Open popup | Left-click the icon | Left-click the icon |
| Menu | Right-click | Right-click |
| Credentials source | `%USERPROFILE%\.claude\.credentials.json` | login **Keychain** (`Claude Code-credentials`), file fallback |
| Installer | NSIS `.exe` | `.dmg` (arm64 + x64) |
| Theme | Follows Windows light/dark | Follows macOS light/dark |

The popup shows the same content everywhere: current session, % remaining, time to reset, weekly
usage, model, last-updated, **Refresh**, and the **Prompt Calculator**.

---

## 🧮 Prompt Calculator

Describe a task and get an **honest estimate** of whether it can finish before your session limit —
as a range, with a confidence level. It never claims exact numbers.

```
Likely to finish            ████████████████░░░░  78%

Estimated usage:  25–40%      Remaining:  67%
Estimated margin: ~27%        Confidence: Medium
```

Complexity is auto-detected from your description (keywords, scope, length) or set manually
(**S / M / L / XL**). Estimates are scaled by the selected **model** (Opus/Sonnet/Haiku burn very
differently) and, optionally, calibrated by **your own local history**. The UI always shows the
factors that drove the estimate and a clear disclaimer.

> Why ranges? Real Claude Code consumption depends on message count, context size, files read, tools
> used, iterations, retries and response length — none knowable in advance. Selling a single exact
> number would be dishonest.

---

## 🔐 Privacy

- **No secrets stored.** The app reads Claude Code's existing token **in place** and uses it only for
  the single official usage request. It never copies, logs, persists, or writes it.
- **Native secure storage.** On macOS the token lives in the **Keychain**; on Windows in the file
  Claude Code manages. The app only reads; it never creates its own credential store.
- **Prompts stay local.** The calculator runs entirely in local Rust.
- **Settings** live in your OS config dir and contain **only UI preferences** — no credentials.
- No cookies, no traffic interception, no scraping.

Full details for every state: **[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)**.

---

## <a id="how-it-gets-the-data"></a>How it gets the data

Claude Code's own `/usage` command is powered by an authenticated endpoint. This app calls **the same
endpoint, with your own local token**, exactly as Claude Code does:

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <local Claude Code OAuth token>
anthropic-beta: oauth-2025-04-20
User-Agent: claude-code/<version>
```

The token is read (read-only) from your platform's native store (Keychain on macOS, the
`.credentials.json` file on Windows/Linux). The response is normalized into session / weekly /
per-model windows. History for the calculator is parsed locally from
`~/.claude/projects/**/*.jsonl`.

**Why on-demand and not polling?** The endpoint is aggressively rate-limited (HTTP 429). On-demand
fetching is what the design calls for *and* the correct approach. The app enforces a 60-second
minimum between live fetches and keeps the last snapshot on screen if it hits a limit.

---

## 📦 Installation

### Windows
1. Download `ClaudeUsageTray-Setup.exe` from the [latest release](https://github.com/Anty48/claude-usage-tray/releases/latest).
2. Run it (per-user, no admin). SmartScreen may warn for unsigned builds → **More info → Run anyway**.
3. The icon appears in the notification area. Left-click to open.

### macOS
1. Download the `.dmg` matching your chip (arm64 / x64).
2. Open it and drag **Claude Usage Tray** to Applications.
3. First launch: right-click the app → **Open** (unsigned builds are Gatekeeper-blocked on
   double-click). Or: `xattr -dr com.apple.quarantine "/Applications/Claude Usage Tray.app"`.
4. The icon appears in the menu bar (there is no Dock icon).

Both need [Claude Code](https://docs.claude.com/en/docs/claude-code/overview) installed and signed in.

---

## 🛠️ Development

Prerequisites: **Rust** (stable), **Node** (for the prebuilt Tauri CLI and icon script), and the
platform WebView (WebView2 is preinstalled on Win 11; WKWebView ships with macOS).

```bash
# install the prebuilt Tauri CLI (fast, no compile)
npm i -g @tauri-apps/cli@^2

# run in development
tauri dev

# fast, platform-independent tests (no network, no Claude required)
cargo test -p claude-usage-core

# lint & format (what CI enforces)
cargo fmt -p claude-usage-core -- --check
cargo clippy -p claude-usage-core --all-targets -- -D warnings

# regenerate icons (brand + severity tray variants)
node scripts/gen-icons.js
```

Build installers locally:

```bash
# Windows
tauri build --bundles nsis
#   → target/release/bundle/nsis/Claude Usage Tray_<ver>_x64-setup.exe

# macOS (on a Mac)
rustup target add aarch64-apple-darwin
tauri build --target aarch64-apple-darwin --bundles dmg
#   → target/aarch64-apple-darwin/release/bundle/dmg/Claude Usage Tray_<ver>_aarch64.dmg
```

---

## 🧱 Architecture

A Cargo workspace with **shared business logic** and a thin, per-platform shell. Only OS integration
is platform-specific; everything else is common and unit-tested.

```
claude-usage-tray/
├─ core/                       # claude-usage-core — shared, GUI-free, fully unit-tested
│  └─ src/
│     ├─ credentials.rs        # ClaudeProvider: read token (macOS Keychain / Win-Linux file)
│     ├─ client.rs             # on-demand usage fetch + 60s rate guard (ureq)
│     ├─ usage.rs              # endpoint → normalized UsageSnapshot + severity
│     ├─ models.rs             # model catalog + relative burn weighting
│     ├─ estimator.rs          # heuristic "can my prompt finish?" (deterministic)
│     ├─ history.rs            # parse local transcripts → real session stats
│     └─ settings.rs           # persisted preferences (no secrets)
├─ src-tauri/                  # claude-usage-tray — the tray / menu-bar app
│  ├─ src/
│  │  ├─ main.rs               # tray + menu; macOS Accessory policy (no Dock)
│  │  └─ commands.rs           # #[tauri::command] bridge; severity-colored tray icon
│  ├─ icons/                   # brand icon + tray-normal/warnsoft/warn/critical variants
│  ├─ capabilities/            # Tauri v2 permissions
│  └─ tauri.conf.json          # window + NSIS (Windows) + DMG (macOS) config
├─ ui/                         # static frontend (no bundler): index.html, styles.css, app.js
├─ docs/TROUBLESHOOTING.md
├─ scripts/gen-icons.js
└─ .github/workflows/          # ci.yml (lint/test/build) + release.yml (installers)
```

**Platform-specific logic is intentionally tiny** and isolated with `#[cfg(target_os = ...)]`:
reading the macOS Keychain (`credentials.rs`) and the macOS Dock/activation policy (`main.rs`).
Everything else — usage parsing, estimation, history, settings, UI — is 100% shared.

Low-footprint choices: **Tauri v2** (native WebView, not Electron), **no background work / timers /
open sockets**, **`ureq`** (blocking, rustls) for the single HTTP call, and a size-optimized release
profile (`opt-level="z"`, LTO, `panic="abort"`, stripped). Idle main process ≈ **29 MB**.

---

## 🚀 CI/CD & Releases

- **`ci.yml`** runs on every push/PR: `rustfmt` + `clippy -D warnings` + unit tests on the shared
  core, and a full compile on **Windows** and **macOS**.
- **`release.yml`** runs on a version tag and builds every installer, publishing them to a GitHub
  Release:

  ```bash
  git tag v1.0.0
  git push --tags
  ```

  Produces `ClaudeUsageTray-Setup.exe`, `ClaudeUsageTray-macOS-arm64.dmg`, and
  `ClaudeUsageTray-macOS-x64.dmg` automatically.

### Code signing & notarization

The builds work **unsigned** (users get the standard SmartScreen/Gatekeeper prompt described in
[Installation](#-installation)). For a smooth public distribution you'll want to sign; the release
workflow already reads these **optional** secrets — add them under *Settings → Secrets → Actions* and
signing turns on automatically, no workflow edits needed:

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | base64 of your *Developer ID Application* `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` / `APPLE_PASSWORD` | Apple ID + app-specific password (notarization) |
| `APPLE_TEAM_ID` | your Apple Developer Team ID |
| `TAURI_SIGNING_PRIVATE_KEY` / `..._PASSWORD` | Windows Authenticode (optional) |

You need a paid **Apple Developer** account for macOS notarization and a code-signing certificate for
Windows. No certificates are bundled or faked here — until you add them, distribution is unsigned.

---

## 🩺 Troubleshooting

Every error the app can show — “Claude Code not found”, “not authenticated”, “session expired”, rate
limits, etc. — is documented with the exact cause and fix in
**[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)**. The in-app **How to fix** button links
straight there.

---

## License

MIT — see [LICENSE](LICENSE). Independent, unofficial project; **not affiliated with Anthropic**.
“Claude” and “Claude Code” are trademarks of Anthropic.
