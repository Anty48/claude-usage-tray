# Claude Usage Tray

A tiny, fast, **Windows system-tray** utility that shows how much of your **Claude Code** usage
you have left — on demand, from a single click. Built in **Rust + Tauri v2**, it sits quietly in
the notification area, does **no background polling**, stores **no secrets**, and never sends your
prompts anywhere.

<p align="center"><em>Click the tray icon → see your remaining 5-hour session &amp; weekly usage → optionally estimate whether a task can finish before you run out.</em></p>

---

## What it does

- **Live usage at a glance.** Reads your current **5-hour session** and **weekly** utilization
  and shows how much is left, when each window resets, and a traffic-light status.
- **On-demand only.** Nothing runs in the background. It fetches fresh data when you click the
  icon (or press *Refresh*), then goes idle again.
- **"Can my prompt finish?" calculator.** Estimate — as an honest **range with a confidence
  level**, never a fake exact number — whether a task is likely to complete before your session
  limit is reached, based on task complexity, the model, and your remaining budget.
- **Learns from your real history (locally).** Optionally calibrates estimates using statistics
  parsed from your own local Claude Code transcripts. Read-only; nothing leaves your machine.
- **Premium, minimal UI.** A small, warm, Anthropic-inspired popup with light/dark themes.

---

## How it gets the data

This was researched before writing any code (see the endpoint details below).

### Usage numbers — the official on-demand endpoint

Claude Code's own `/usage` command is powered by an authenticated endpoint:

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <your local Claude Code OAuth token>
anthropic-beta: oauth-2025-04-20
User-Agent: claude-code/<version>
```

The app calls **the same endpoint, with your own token**, exactly as Claude Code does. The token
is read (read-only) from the credentials Claude Code already stores locally on this machine:

```
%USERPROFILE%\.claude\.credentials.json   →  claudeAiOauth.accessToken
```

(The directory can be overridden with the `CLAUDE_CONFIG_DIR` environment variable, matching
Claude Code.) The response is normalized into a small model:

| Field                 | Meaning                                              |
| --------------------- | ---------------------------------------------------- |
| `five_hour`           | Current 5-hour **session** window (`utilization` %)  |
| `seven_day`           | **Weekly** usage across all models                   |
| `seven_day_opus` / `seven_day_sonnet` | Per-model weekly windows (when present) |
| `resets_at`           | When each window resets (ISO 8601)                   |
| `extra_usage` / `spend` | Paid overflow credits, if enabled                  |

> `utilization` is the fraction **used** (0–100); the app shows **remaining = 100 − used**.

**Why on-demand and not polling?** This endpoint is aggressively rate-limited and returns HTTP
429 if hit too often. On-demand fetching is therefore both what the spec asked for **and** the
technically correct approach. The app also enforces a **60-second minimum interval** between live
fetches (serving the last snapshot from an in-memory cache in between) and backs off gracefully on
429, keeping the last known values on screen.

### History for the calculator — local transcripts

Claude Code stores each conversation as JSON Lines under `~/.claude/projects/<slug>/<id>.jsonl`.
Assistant messages carry a `usage` object (`input_tokens`, `output_tokens`, `cache_*`) and a
`model`. The app reads these **locally and read-only** to compute real per-session statistics
(sessions, replies/session, tokens/session, per-model split) shown in Settings and used to lightly
calibrate estimates.

---

## Honest limitations

This app deliberately **does not fake precision or data**:

- **The calculator is an estimate, not a prediction.** Real Claude Code consumption depends on
  message count, context size, files read, tools used, iterations, retries, response length and
  the model — none knowable in advance. Output is always a **range + confidence (capped at
  Medium)** with the factors that drove it.
- **No token → rate-limit-% mapping.** Anthropic does not expose one, so history is used only for
  relative sizing, never presented as an exact percentage.
- **The endpoint can rate-limit you.** If it returns 429, the app shows your last known values and
  a gentle "try again shortly" message.
- **Token refresh is delegated to Claude Code.** The app never writes to your credentials. If the
  stored token has expired, it asks you to open Claude Code (which refreshes it) rather than
  refreshing it itself — this avoids interfering with Claude Code's own session.
- **Windows-first.** The credential path and installer target Windows. macOS stores the token in
  the Keychain instead of a file and is out of scope for now.
- **Tray "percentage" is a tooltip.** Windows tray icons can't render a text badge, so the
  remaining % is surfaced on hover.

---

## Privacy

- **No secrets are stored by this app.** It reads Claude Code's existing token in-place and uses
  it only for the single official usage request. It never copies, logs, or persists the token.
- **Your prompts never leave your machine.** The calculator runs entirely locally in Rust.
- **Settings** are stored at `%APPDATA%\ClaudeUsageTray\settings.json` and contain **only UI
  preferences** — no credentials.
- **"Clear local history"** only forgets the app's *derived* stats for the current run. It never
  deletes Claude Code's transcripts — those belong to Claude Code.
- No browser cookies are read, no traffic is intercepted, no web scraping is performed.

---

## Architecture

A Cargo workspace with a clean split between **testable core logic** (no GUI) and the **thin Tauri
shell**:

```
running-out-of-claude/
├─ core/                       # claude-usage-core — pure logic, no Tauri (unit-tested)
│  └─ src/
│     ├─ credentials.rs        # locate + read local Claude Code OAuth token (read-only)
│     ├─ client.rs             # on-demand HTTP + rate guard (ureq)
│     ├─ usage.rs              # endpoint types → normalized UsageSnapshot + severity
│     ├─ models.rs             # model catalog + relative burn weighting
│     ├─ estimator.rs          # heuristic "can my prompt finish?" (deterministic)
│     ├─ history.rs            # parse local transcripts → real session stats
│     └─ settings.rs           # persisted preferences (no secrets)
├─ src-tauri/                  # claude-usage-tray — the tray app
│  ├─ src/
│  │  ├─ main.rs               # tray icon, context menu, popup positioning, autostart
│  │  └─ commands.rs           # #[tauri::command] bridge to core
│  ├─ icons/                   # generated app + tray icons
│  ├─ capabilities/            # Tauri v2 permissions
│  └─ tauri.conf.json          # window + NSIS installer config
├─ ui/                         # static frontend (no bundler): index.html, styles.css, app.js
└─ scripts/gen-icons.js        # zero-dependency icon generator
```

Design choices for **low footprint**:

- **Tauri v2**, not Electron: the popup uses the Windows-native **WebView2** (already on
  Windows 11) instead of bundling Chromium.
- **No background work, no open sockets, no timers.** The app is event-driven: it does nothing
  until you click.
- **`ureq`** (blocking, `rustls`) for the single HTTP call — no async runtime, no OpenSSL.
- Release profile optimizes for size (`opt-level = "z"`, LTO, `panic = "abort"`, stripped).

---

## Building from source

### Prerequisites

- **Rust** (stable) — <https://rustup.rs>
- **Node.js** — only used once to regenerate icons (optional; icons are committed)
- **WebView2 Runtime** — preinstalled on Windows 11; on older Windows install the Evergreen
  runtime from Microsoft.
- The Tauri CLI:

  ```powershell
  cargo install tauri-cli --version "^2"
  ```

### Run in development

```powershell
cargo tauri dev
```

### Run the unit tests (no network, no Claude required)

```powershell
cargo test -p claude-usage-core
```

### Build a Windows installer

```powershell
cargo tauri build
```

The NSIS installer is produced under:

```
src-tauri\target\release\bundle\nsis\Claude Usage Tray_0.1.0_x64-setup.exe
```

(Rename to `ClaudeUsageTray-Setup.exe` if you like.)

### Regenerate icons (optional)

```powershell
node scripts/gen-icons.js
```

---

## Installing

1. Run the generated **`...-setup.exe`**. It installs per-user (no admin needed), adds a Start-menu
   entry and an uninstaller.
2. Launch **Claude Usage Tray**. Its icon appears in the notification area.
3. **Left-click** the icon to open the popup; **right-click** for the menu
   (Refresh · Usage details · Prompt calculator · Settings · Open Claude Code · About · Quit).
4. Optionally enable **Start with Windows** in Settings.

To uninstall: *Settings → Apps → Claude Usage Tray → Uninstall*.

---

## Usage

- **Left-click tray icon** → popup opens and refreshes usage (respecting the 60s guard).
- **Refresh** → force a live fetch.
- **Calculator** → describe a task, pick complexity (or leave on *Auto*) and model, then
  *Estimate*. You get a verdict, a probability, an estimated usage range, your margin, a confidence
  badge, and the factors behind it.
- **Click away / Esc / ✕** → popup hides and the app goes idle again.

---

## Troubleshooting

| You see… | What it means | Fix |
| --- | --- | --- |
| **Claude Code not found** | No `~/.claude/.credentials.json` | Install Claude Code and sign in. |
| **Claude Code is not authenticated** | Credentials file has no usable token | Open Claude Code and sign in. |
| **Session expired** | Stored token is past its expiry | Open Claude Code (it refreshes the token), then Refresh. |
| **Try again shortly** | Endpoint returned 429 (rate limited) | Wait a bit; the last known values stay on screen. |
| **Network unavailable** | Couldn't reach `api.anthropic.com` | Check your connection and Retry. |
| **Usage information unavailable** | Unexpected/parse error | Retry; check logs if it persists. |

Every error card includes a **How to fix** button that opens the right action (Claude Code or the
docs). Friendly messages are shown to you; technical detail is preserved for logs.

---

## Acknowledgements & references

- Anthropic Claude Code and its `/usage` command / `/api/oauth/usage` endpoint.
- Community reports documenting the endpoint's shape, required headers and rate-limit behavior:
  - <https://github.com/anthropics/claude-code/issues/31637>
  - <https://github.com/anthropics/claude-code/issues/31021>
  - <https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor/issues/202>

## License

MIT — see [LICENSE](LICENSE). Independent, unofficial project; not affiliated with Anthropic.
