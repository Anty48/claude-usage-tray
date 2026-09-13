/* Claude Usage Tray — landing page behaviour.
   No tracking, no cookies. One optional, best-effort GitHub API read to show the
   version and avoid offering downloads for assets that aren't published yet.
   Everything degrades gracefully: with JS off, the stable /releases/latest/download
   links still work. */

(function () {
  "use strict";

  var REPO = "Anty48/claude-usage-tray";
  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ---------- Nav shadow on scroll ---------- */
  var nav = document.getElementById("nav");
  function onScroll() { nav.classList.toggle("scrolled", window.scrollY > 8); }
  onScroll();
  window.addEventListener("scroll", onScroll, { passive: true });

  /* ---------- Scroll reveal ---------- */
  var reveal = [].slice.call(document.querySelectorAll(".reveal, .hero-visual"));
  if (reduceMotion || !("IntersectionObserver" in window)) {
    reveal.forEach(function (el) { el.classList.add("is-visible"); });
  } else {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) { e.target.classList.add("is-visible"); io.unobserve(e.target); }
      });
    }, { threshold: 0.16, rootMargin: "0px 0px -8% 0px" });
    reveal.forEach(function (el) { io.observe(el); });
  }

  /* ---------- Count-up numbers ---------- */
  function countUp(el) {
    var target = parseInt(el.getAttribute("data-count"), 10) || 0;
    if (reduceMotion) { el.textContent = String(target); return; }
    var start = null, dur = 1100;
    function tick(ts) {
      if (start === null) start = ts;
      var p = Math.min((ts - start) / dur, 1);
      var eased = 1 - Math.pow(1 - p, 3);
      el.textContent = String(Math.round(target * eased));
      if (p < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }
  var counters = [].slice.call(document.querySelectorAll("[data-count]"));
  if (!("IntersectionObserver" in window)) {
    counters.forEach(countUp);
  } else {
    var co = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) { countUp(e.target); co.unobserve(e.target); }
      });
    }, { threshold: 0.6 });
    counters.forEach(function (el) { co.observe(el); });
  }

  /* ---------- OS / architecture detection ---------- */
  function detectOS() {
    var ua = (navigator.userAgent || "").toLowerCase();
    var plat = (navigator.platform || "").toLowerCase();
    if (ua.indexOf("windows") > -1 || plat.indexOf("win") > -1) return "win";
    // iPadOS reports as Mac; treat any Apple desktop UA as mac for download purposes.
    if (ua.indexOf("mac") > -1 || plat.indexOf("mac") > -1) return "mac";
    return null;
  }

  // Best-effort Apple Silicon vs Intel. Browsers don't expose this reliably, so we
  // only upgrade the hint when we're fairly confident; otherwise show both equally.
  function detectMacArch() {
    try {
      var canvas = document.createElement("canvas");
      var gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
      if (gl) {
        var dbg = gl.getExtension("WEBGL_debug_renderer_info");
        if (dbg) {
          var r = (gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) || "").toString().toLowerCase();
          if (r.indexOf("apple") > -1 && r.indexOf("intel") === -1) return "arm";
          if (r.indexOf("intel") > -1 || r.indexOf("amd") > -1 || r.indexOf("radeon") > -1) return "x64";
        }
      }
    } catch (e) { /* ignore */ }
    return null;
  }

  var os = detectOS();
  var osHint = document.getElementById("osHint");

  function recommendCard(platform, label) {
    var card = document.querySelector('.dl-card[data-platform="' + platform + '"]');
    if (!card) return;
    card.classList.add("is-recommended");
    var badge = card.querySelector(".dl-badge");
    if (badge) { badge.hidden = false; if (label) badge.textContent = label; }
  }

  if (os === "win") {
    recommendCard("win");
    if (osHint) { osHint.textContent = "Recommended for your device: Windows"; osHint.hidden = false; }
  } else if (os === "mac") {
    recommendCard("mac");
    var arch = detectMacArch();
    var arm = document.querySelector('[data-dl="mac-arm"]');
    var x64 = document.querySelector('[data-dl="mac-x64"]');
    if (arch === "arm" && arm) {
      arm.classList.add("is-recommended");
      if (osHint) { osHint.textContent = "Recommended for your device: macOS (Apple Silicon)"; osHint.hidden = false; }
    } else if (arch === "x64" && x64) {
      x64.classList.add("is-recommended");
      if (osHint) { osHint.textContent = "Recommended for your device: macOS (Intel)"; osHint.hidden = false; }
    } else if (osHint) {
      osHint.textContent = "Recommended for your device: macOS — pick Apple Silicon or Intel below";
      osHint.hidden = false;
    }
  }

  /* ---------- Optional: version + asset availability (progressive enhancement) ---------- */
  // If this fetch fails (offline, rate-limited, blocked) nothing breaks — the stable
  // download links remain intact.
  if ("fetch" in window) {
    fetch("https://api.github.com/repos/" + REPO + "/releases/latest", {
      headers: { Accept: "application/vnd.github+json" }
    })
      .then(function (r) { return r.ok ? r.json() : Promise.reject(r.status); })
      .then(function (rel) {
        var tag = rel.tag_name || "";
        if (tag) {
          var inline = document.getElementById("versionInline");
          if (inline) inline.textContent = tag;
          var latestTag = document.getElementById("latestTag");
          if (latestTag) latestTag.textContent = tag;
          document.title = document.title; // no-op, keep title stable
        }

        // Map published asset names so we can flag anything not yet built.
        var names = {};
        (rel.assets || []).forEach(function (a) { names[a.name] = true; });

        var map = [
          { key: "ClaudeUsageTray-Setup.exe", sels: ['[data-dl="win"]'] },
          { key: "ClaudeUsageTray-macOS-arm64.dmg", sels: ['[data-dl="mac"]', '[data-dl="mac-arm"]'] },
          { key: "ClaudeUsageTray-macOS-x64.dmg", sels: ['[data-dl="mac-x64"]'] }
        ];
        map.forEach(function (m) {
          if (names[m.key]) return; // asset exists — leave the link alone
          m.sels.forEach(function (sel) {
            [].slice.call(document.querySelectorAll(sel)).forEach(function (el) {
              el.setAttribute("aria-disabled", "true");
              el.setAttribute("title", "This build isn't published yet — check GitHub Releases.");
              el.setAttribute("href", "https://github.com/" + REPO + "/releases");
            });
          });
        });
      })
      .catch(function () { /* stable links already work; ignore */ });
  }

  /* ---------- Year (none needed) & smooth anchor focus ---------- */
  // Native smooth scrolling handles anchors; no extra JS required.
})();
