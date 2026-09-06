# Troubleshooting

Claude Usage Tray never invents data. When it can't read your usage it tells you exactly what's
wrong and how to fix it. This page explains every state you might see, on both Windows and macOS.

> **How does it get my usage?** It reuses the session **you already have** in Claude Code. It reads
> your local Claude Code OAuth token (read-only) and calls the same official endpoint that powers
> Claude Code's `/usage` command. It never asks for a password, never stores or copies the token,
> and never writes to your credentials. See [Privacy](../README.md#privacy).

---

## <a id="claude-code-not-found"></a>🔍 "Claude Code not found"

**Meaning:** the app couldn't find any Claude Code credentials on this device.

Where it looks:

| Platform | Location |
| --- | --- |
| **Windows / Linux** | the file `%USERPROFILE%\.claude\.credentials.json` (or `$CLAUDE_CONFIG_DIR\.credentials.json`) |
| **macOS** | the login **Keychain** item `Claude Code-credentials` (service name), falling back to `~/.claude/.credentials.json` |

**If that file / Keychain item does not exist, it means Claude Code is not installed or has never
been signed in on this machine.** Claude Usage Tray depends on Claude Code being present — it is a
companion utility, not a replacement.

**Fix:**

1. Install Claude Code — <https://docs.claude.com/en/docs/claude-code/overview>
2. Sign in once by running `claude` in a terminal and completing the login.
3. Back in Claude Usage Tray, click **Refresh**.

> The credential file/Keychain item is **per-device**. Installing only Claude Usage Tray on a new
> computer that has never run Claude Code will always show this until you install and sign in to
> Claude Code there.

---

## <a id="not-authenticated"></a>🔒 "Claude Code is not authenticated"

**Meaning:** the credentials store exists, but there is no usable OAuth access token in it (for
example Claude Code was installed but you never completed sign-in, or you signed out).

**Fix:** open Claude Code (`claude`) and sign in, then click **Refresh**.

---

## <a id="session-expired"></a>⏰ "Session expired"

**Meaning:** the stored access token is past its expiry time.

Claude Usage Tray **deliberately does not refresh the token itself** — it never writes to your
credentials, so it can't disturb Claude Code's own session. Claude Code refreshes the token
automatically during normal use.

**Fix:** open Claude Code (which refreshes the token), then click **Refresh**.

---

## <a id="rate-limited"></a>⏳ "Try again shortly"

**Meaning:** the usage endpoint returned HTTP 429 (rate limited).

This endpoint is intentionally aggressive about rate limiting. Claude Usage Tray is **on-demand and
never polls**, and it enforces a 60-second minimum between live fetches, but if you refresh many
times in quick succession you can still hit it. Your **last known values stay on screen**.

**Fix:** wait a minute and click **Refresh** again.

---

## <a id="token-rejected"></a>🔒 "Token rejected"

**Meaning:** the endpoint returned 401/403 — the token was read but not accepted.

**Fix:** re-open Claude Code to refresh your session, then **Refresh**. If it persists, sign out and
back in to Claude Code.

---

## <a id="network"></a>📶 "Network unavailable"

**Meaning:** the app couldn't reach `api.anthropic.com` (offline, DNS, proxy, firewall).

**Fix:** check your connection and click **Retry**. Corporate proxies/VPNs that intercept TLS can
also cause this.

---

## <a id="unavailable"></a>••• "Usage information unavailable"

**Meaning:** an unexpected error (e.g. the response couldn't be parsed).

**Fix:** click **Retry**. If it keeps happening, please [open an issue](https://github.com/Anty48/claude-usage-tray/issues)
and include your OS and Claude Code version (`claude --version`).

---

## macOS-specific notes

- **Keychain prompt:** the very first time the app reads your token, macOS may ask to allow
  `Claude Usage Tray` (or `security`) to access the `Claude Code-credentials` item. Choose
  **Always Allow** to avoid repeated prompts.
- **Locked keychain over SSH/headless:** if the login keychain is locked (common in SSH sessions)
  reading it returns `errSecInteractionNotAllowed`; the app then falls back to
  `~/.claude/.credentials.json` if present.
- **No Dock icon:** the app runs as a menu-bar utility (`Accessory` activation policy) — look for
  its icon in the **menu bar**, not the Dock.
- **Gatekeeper:** if you downloaded an **unsigned** build, macOS may say the app "can't be opened".
  Right-click the app → **Open**, or run `xattr -dr com.apple.quarantine "/Applications/Claude Usage Tray.app"`.
  See [signing & notarization](../README.md#code-signing--notarization).

## Windows-specific notes

- **SmartScreen:** an unsigned installer may show "Windows protected your PC". Click
  **More info → Run anyway**.
- **Tray icon hidden:** Windows often hides new tray icons in the overflow (`^`) area. Drag the
  Claude Usage Tray icon onto the taskbar to keep it visible.

---

Still stuck? Logs and details welcome at
<https://github.com/Anty48/claude-usage-tray/issues>.
