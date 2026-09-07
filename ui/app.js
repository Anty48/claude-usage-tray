"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const $ = (id) => document.getElementById(id);
const state = {
  settings: null,
  usage: null, // last UsageResponse
};

// ---------- theme ----------
function applyTheme(theme) {
  const root = document.documentElement;
  let effective = theme;
  if (theme === "system") {
    effective = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  root.setAttribute("data-theme", effective);
}

// ---------- navigation ----------
function switchView(view) {
  const map = { "usage-refresh": "usage" };
  const target = map[view] || view;
  document.querySelectorAll(".view").forEach((v) => v.classList.toggle("is-active", v.dataset.view === target));
  document.querySelectorAll("#segmented .seg-btn").forEach((b) => b.classList.toggle("is-active", b.dataset.view === target));

  if (target === "usage") fetchUsage(view === "usage-refresh");
  if (target === "settings") loadHistory();
  if (target === "calculator") syncCalcRemaining();
}

// ---------- usage ----------
function sevClass(sev) {
  return sev || "normal";
}

function formatReset(iso) {
  if (!iso) return "—";
  const then = new Date(iso).getTime();
  const now = Date.now();
  let diff = Math.round((then - now) / 60000); // minutes
  if (diff <= 0) return "now";
  const h = Math.floor(diff / 60);
  const m = diff % 60;
  if (h === 0) return `in ${m}m`;
  return `in ${h}h ${m}m`;
}

function timeAgo(ms) {
  const s = Math.round((Date.now() - ms) / 1000);
  if (s < 8) return "just now";
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  return `${m}m ago`;
}

async function fetchUsage(force) {
  const updated = $("updated");
  const refreshOnClick = state.settings ? state.settings.refresh_on_click : true;
  const doFetch = force || refreshOnClick || !state.usage;

  if (doFetch) updated.innerHTML = `<span class="spin"></span>Updating…`;

  let resp;
  try {
    resp = await invoke("get_usage", { force: !!force });
  } catch (e) {
    renderError({ error_kind: "unavailable", error: String(e) });
    return;
  }
  state.usage = resp;
  renderUsage(resp);
}

function renderError(resp) {
  const box = $("usageError");
  const body = $("usageBody");
  const TS = "https://github.com/Anty48/claude-usage-tray/blob/main/docs/TROUBLESHOOTING.md";
  const help = {
    not_found: { icon: "🔍", title: "Claude Code not found", msg: "No Claude Code credentials on this device. Install Claude Code and sign in — it's required.", url: TS + "#claude-code-not-found" },
    not_authenticated: { icon: "🔒", title: "Claude Code is not authenticated", msg: "Open Claude Code and sign in to your account, then try again.", claude: true },
    token_expired: { icon: "⏰", title: "Session expired", msg: "Your Claude Code session token expired. Open Claude Code to refresh it.", claude: true },
    rate_limited: { icon: "⏳", title: "Try again shortly", msg: "The usage endpoint is rate limited. Showing your last known values.", url: TS + "#rate-limited" },
    unauthorized: { icon: "🔒", title: "Token rejected", msg: "The stored token was rejected. Re-open Claude Code to refresh your session.", claude: true },
    network: { icon: "📶", title: "Network unavailable", msg: "Couldn't reach the usage endpoint. Check your connection and retry.", url: TS + "#network" },
    unavailable: { icon: "•••", title: "Usage information unavailable", msg: "We couldn't read your usage right now.", url: TS + "#unavailable" },
  };
  const h = help[resp.error_kind] || help.unavailable;
  box.hidden = false;
  box.innerHTML = `
    <div class="state-icon">${h.icon}</div>
    <div class="state-title">${h.title}</div>
    <div class="state-msg">${h.msg}</div>
    <div class="btn-row">
      <button class="btn" id="stateFix">How to fix</button>
      <button class="btn btn-ghost" id="stateRetry">Retry</button>
    </div>`;
  // Hide the numeric body only when we have no cached data at all.
  body.style.opacity = resp.snapshot ? "1" : "0.35";
  $("stateRetry").onclick = () => fetchUsage(true);
  $("stateFix").onclick = () => {
    if (h.claude) invoke("open_claude_code");
    else if (h.url) invoke("open_url", { url: h.url });
    else fetchUsage(true);
  };
}

function renderUsage(resp) {
  if (resp.error) {
    renderError(resp);
  } else {
    $("usageError").hidden = true;
    $("usageBody").style.opacity = "1";
  }
  const snap = resp.snapshot;
  if (!snap) return;

  const s = snap.session;
  if (s) {
    const rem = Math.round(s.percent_remaining);
    $("sessionRemaining").textContent = rem;
    $("sessionRemaining").className = "big " + sevClass(s.severity);
    const meter = $("sessionMeter");
    meter.style.width = Math.min(100, s.percent_remaining) + "%";
    meter.className = "meter-fill " + sevClass(s.severity);
    $("sessionReset").textContent = formatReset(s.resets_at);
    const dot = $("statusDot");
    dot.className = "dot " + sevClass(s.severity);
  }

  const w = snap.weekly;
  const weeklyRow = $("weeklyRow");
  if (w) {
    weeklyRow.hidden = false;
    $("weeklyVal").textContent = Math.round(w.percent_remaining) + "% left";
    const wm = $("weeklyMeter");
    wm.style.width = Math.min(100, w.percent_remaining) + "%";
    wm.className = "meter-fill " + sevClass(w.severity);
    $("weeklyReset").textContent = formatReset(w.resets_at);
  } else {
    weeklyRow.hidden = true;
  }

  // extra per-model / credit rows
  const ex = $("extraRows");
  ex.innerHTML = "";
  (snap.extras || []).forEach((x) => {
    const div = document.createElement("div");
    div.className = "row";
    div.innerHTML = `
      <div class="row-head"><span class="row-label">${x.label}</span><span class="row-val">${Math.round(x.percent_remaining)}% left</span></div>
      <div class="meter meter-sm"><div class="meter-fill ${sevClass(x.severity)}" style="width:${Math.min(100, x.percent_remaining)}%"></div></div>`;
    ex.appendChild(div);
  });
  if (snap.extra_usage && snap.extra_usage.enabled) {
    const eu = snap.extra_usage;
    const div = document.createElement("div");
    div.className = "row";
    const cur = eu.currency || "";
    div.innerHTML = `<div class="row-head"><span class="row-label">Extra usage credits</span><span class="row-val">${Math.round(eu.spend_percent || 0)}% used</span></div>
      <p class="submeta small">${(eu.used_credits ?? 0)} / ${(eu.monthly_limit ?? 0)} ${cur}</p>`;
    ex.appendChild(div);
  }

  $("updated").textContent = "Last updated: " + timeAgo(snap.fetched_at_ms) + (resp.from_cache && !resp.error ? " (cached)" : "");
  syncCalcRemaining();

  // reflect on tray tooltip + severity-colored icon
  const showPct = state.settings ? state.settings.show_percentage_in_tray : true;
  const rem = s ? s.percent_remaining : null;
  const sev = s ? s.severity : null;
  invoke("update_tray", { remaining: rem, severity: sev, show_percent: showPct });
}

// ---------- account ----------
async function loadAccount() {
  try {
    const a = await invoke("get_account");
    const chip = $("planChip");
    if (a.subscription_type) {
      chip.textContent = a.subscription_type;
      chip.hidden = false;
    }
  } catch (_) {}
}

// ---------- calculator ----------
let complexityChoice = "auto";

function syncCalcRemaining() {
  const s = state.usage && state.usage.snapshot && state.usage.snapshot.session;
  const rem = s ? Math.round(s.percent_remaining) : 100;
  $("calcRemaining").textContent = rem + "%";
}

async function runEstimate() {
  const task = $("taskInput").value.trim();
  const s = state.usage && state.usage.snapshot && state.usage.snapshot.session;
  const remaining = s ? s.percent_remaining : 100;
  const req = {
    task,
    complexity: complexityChoice === "auto" ? null : complexityChoice,
    model: $("modelSelect").value,
    sensitivity: state.settings ? state.settings.sensitivity : "balanced",
    remaining_percent: remaining,
    use_history: state.settings ? state.settings.use_history !== false : true,
  };
  let est;
  try {
    est = await invoke("estimate", { req });
  } catch (e) {
    return;
  }
  renderEstimate(est);
}

function renderEstimate(e) {
  const box = $("estimateResult");
  box.hidden = false;
  const verdictMap = {
    likely_to_finish: { t: "Likely to finish", c: "normal" },
    borderline: { t: "Borderline", c: "warnsoft" },
    risk_of_running_out: { t: "Risk of running out", c: "critical" },
  };
  const v = verdictMap[e.verdict] || verdictMap.borderline;
  const prob = Math.round(e.finish_probability);
  const marginTxt = (e.margin >= 0 ? "+" : "") + e.margin.toFixed(0) + "%";
  const factors = (e.factors || []).map((f) => `<li>${f}</li>`).join("");
  box.innerHTML = `
    <div class="result-verdict"><span class="dot ${v.c}"></span>${v.t}</div>
    <div class="prob-meter"><div class="prob-fill" style="width:${prob}%;background:var(--${v.c})"></div></div>
    <p class="submeta small">${prob}% chance of finishing with your current budget</p>
    <div class="result-grid">
      <div><div class="k">Estimated usage</div><div class="v">${e.usage_min}–${e.usage_max}%</div></div>
      <div><div class="k">Remaining</div><div class="v">${Math.round(e.remaining_percent)}%</div></div>
      <div><div class="k">Estimated margin</div><div class="v">${marginTxt}</div></div>
      <div><div class="k">Confidence</div><div class="v"><span class="badge ${e.confidence}">${e.confidence}</span></div></div>
    </div>
    <ul class="factors">${factors}</ul>
    <p class="disclaimer">Estimates are rough ranges, not predictions. Real Claude Code usage depends on context size, files read, tools, iterations and retries — none knowable in advance.</p>`;
}

// ---------- settings ----------
async function loadSettings() {
  state.settings = await invoke("get_settings");
  const s = state.settings;
  if (s.use_history === undefined) s.use_history = true;
  applyTheme(s.theme);
  $("setTrayPct").checked = s.show_percentage_in_tray;
  $("setRefreshClick").checked = s.refresh_on_click;
  $("setTheme").value = s.theme;
  $("setModel").value = s.model;
  $("setSensitivity").value = s.sensitivity;
  $("setUseHistory").checked = s.use_history !== false;
  $("modelSelect").value = s.model;
  try {
    $("setAutostart").checked = await invoke("get_autostart");
  } catch (_) {}
}

async function saveSettings() {
  const s = state.settings;
  s.show_percentage_in_tray = $("setTrayPct").checked;
  s.refresh_on_click = $("setRefreshClick").checked;
  s.theme = $("setTheme").value;
  s.model = $("setModel").value;
  s.sensitivity = $("setSensitivity").value;
  s.use_history = $("setUseHistory").checked;
  applyTheme(s.theme);
  $("modelSelect").value = s.model;
  // strip our UI-only field before persisting core Settings
  const payload = {
    start_with_windows: $("setAutostart").checked,
    show_percentage_in_tray: s.show_percentage_in_tray,
    refresh_on_click: s.refresh_on_click,
    theme: s.theme,
    model: s.model,
    sensitivity: s.sensitivity,
    use_history: s.use_history,
  };
  try {
    await invoke("save_settings", { settings: payload });
  } catch (_) {}
}

async function loadHistory() {
  try {
    const h = await invoke("get_history");
    const sum = $("historySummary");
    if (h.cleared) {
      sum.textContent = "History cleared for this session.";
    } else if (h.sessions === 0) {
      sum.textContent = "No transcript history found yet.";
    } else {
      const k = (n) => (n >= 1000 ? (n / 1000).toFixed(0) + "k" : String(n));
      sum.textContent = `${h.sessions} sessions · ~${h.avg_messages} replies/session · ~${k(h.avg_tokens)} tokens/session` + (h.has_signal ? " · calibrating estimates" : "");
    }
  } catch (_) {}
}

// ---------- wiring ----------
function wire() {
  // segmented main nav
  document.querySelectorAll("#segmented .seg-btn").forEach((b) => {
    b.onclick = () => switchView(b.dataset.view);
  });
  // complexity segmented
  document.querySelectorAll("#complexitySeg .seg-btn").forEach((b) => {
    b.onclick = () => {
      complexityChoice = b.dataset.c;
      document.querySelectorAll("#complexitySeg .seg-btn").forEach((x) => x.classList.toggle("is-active", x === b));
    };
  });

  $("btnClose").onclick = () => getCurrentWindow().hide();
  $("navSettings").onclick = () => switchView("settings");
  $("backFromSettings").onclick = () => switchView("usage");
  $("backFromAbout").onclick = () => switchView("usage");
  $("btnRefresh").onclick = () => fetchUsage(true);
  $("btnGoCalc").onclick = () => switchView("calculator");
  $("btnEstimate").onclick = runEstimate;
  $("btnDocs").onclick = () => invoke("open_url", { url: "https://docs.claude.com/en/docs/claude-code/overview" });
  $("btnClearHistory").onclick = async () => { await invoke("clear_history"); loadHistory(); };

  // settings change listeners
  ["setTrayPct", "setRefreshClick", "setTheme", "setModel", "setSensitivity", "setUseHistory"].forEach((id) => {
    $(id).addEventListener("change", saveSettings);
  });
  $("setAutostart").addEventListener("change", async (e) => {
    try {
      await invoke("set_autostart", { enabled: e.target.checked });
      saveSettings();
    } catch (_) {
      e.target.checked = !e.target.checked;
    }
  });

  // Esc closes
  window.addEventListener("keydown", (e) => { if (e.key === "Escape") getCurrentWindow().hide(); });

  // react to system theme changes when in "system" mode
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (state.settings && state.settings.theme === "system") applyTheme("system");
  });
}

// ---------- init ----------
async function init() {
  wire();
  await loadSettings();
  loadAccount();
  await fetchUsage(false); // warm cache so first open is instant
  listen("navigate", (e) => switchView(e.payload));
}

document.addEventListener("DOMContentLoaded", init);
