# Claude Usage Tray — website

The landing page for **Claude Usage Tray** — a single static page, no framework, no build step.
This source lives in a **private** repository (`Anty48/claude-usage-tray-web`); the app itself and
its installers stay in the public `Anty48/claude-usage-tray` repo.

```
├─ index.html        # markup + SEO/OG metadata
├─ styles.css        # warm cream/brown/orange theme, light + dark, responsive
├─ main.js           # OS detection, scroll reveal, count-ups, release check
├─ assets/
│  ├─ favicon.svg    # own gauge + terminal mark (no Anthropic assets)
│  ├─ og.svg         # 1200×630 social preview (editable source)
│  └─ og.jpg         # 1200×630 JPEG referenced by the meta tags (rendered from og.svg)
└─ README.md
```

## Downloads

Download buttons point at **stable** GitHub Release URLs, so they never need editing after a new release:

- `https://github.com/Anty48/claude-usage-tray/releases/latest/download/ClaudeUsageTray-Setup.exe`
- `https://github.com/Anty48/claude-usage-tray/releases/latest/download/ClaudeUsageTray-macOS-arm64.dmg`
- `https://github.com/Anty48/claude-usage-tray/releases/latest/download/ClaudeUsageTray-macOS-x64.dmg`

These names are produced verbatim by `.github/workflows/release.yml`. `main.js` makes one optional,
best-effort call to the GitHub API to show the current version and to disable a button if that asset
isn't published yet (e.g. the Intel `.dmg` while its runner is still building). If the call fails,
the stable links still work.

## Preview locally

Any static server works — no dependencies:

```bash
python -m http.server 8080      # then open http://localhost:8080
```

## Deploy (Vercel)

The site is deployed to Vercel (team `anty4`, project `claude-usage-tray-site`). Because the source
repo is private, the preview is served behind **Vercel Authentication** — visible to the account
owner, not the public. To take it fully public later, either turn off deployment protection for the
project in Vercel, or promote a production deployment.

For automatic redeploys on every push, connect this repo in the Vercel project's **Git** settings
(the Vercel GitHub app needs access to this private repo). Until then, redeploy manually.

The site is fully static, so it can also be hosted anywhere (Netlify, Cloudflare Pages, any bucket,
or GitHub Pages on a public repo) by serving these files as-is.

## Notes

- No tracking, no cookies, no analytics, no external fonts or scripts — everything is self-contained.
- Independent, unofficial project; not affiliated with Anthropic. See the disclaimer in the footer.
