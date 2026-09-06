#!/usr/bin/env node
/*
 * Icon generator for Claude Usage Tray — zero dependencies.
 *
 * Draws a warm, Anthropic-inspired "usage gauge + terminal prompt" mark and emits every PNG
 * size Tauri needs plus a multi-size .ico. Run: `node scripts/gen-icons.js`
 */
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const OUT = path.join(__dirname, "..", "src-tauri", "icons");
fs.mkdirSync(OUT, { recursive: true });

// ---- palette (warm / Anthropic-ish) ----
const CREAM = [243, 238, 228, 255];
const TRACK = [228, 219, 203, 255];
const CORAL = [217, 119, 87, 255];      // #D97757
const CORAL_HI = [230, 140, 108, 255];
const INK = [43, 42, 39, 255];          // warm near-black

function mix(a, b, t) {
  return [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
    a[3] + (b[3] - a[3]) * t,
  ];
}

// distance from point p to segment ab
function distSeg(px, py, ax, ay, bx, by) {
  const dx = bx - ax, dy = by - ay;
  const l2 = dx * dx + dy * dy;
  let t = l2 === 0 ? 0 : ((px - ax) * dx + (py - ay) * dy) / l2;
  t = Math.max(0, Math.min(1, t));
  const cx = ax + t * dx, cy = ay + t * dy;
  return Math.hypot(px - cx, py - cy);
}

// Sample the icon at normalized coords (0..1). Returns RGBA (0..255) premultiplied-free.
const PROGRESS = 0.68;           // gauge fill fraction
const START = (135 * Math.PI) / 180;
const SWEEP = (270 * Math.PI) / 180;

function sample(x, y) {
  // background: rounded square, transparent outside
  const r = 0.22; // corner radius
  const inRounded = roundedRectCoverage(x, y, 0.03, 0.03, 0.97, 0.97, r);
  let col = [0, 0, 0, 0];
  if (inRounded > 0) col = withAlpha(CREAM, inRounded);

  const cx = 0.5, cy = 0.5;
  const dx = x - cx, dy = y - cy;
  const dist = Math.hypot(dx, dy);
  const ringOuter = 0.34, ringThick = 0.085;
  const ringInner = ringOuter - ringThick;
  // angle measured clockwise from +x, but we want gauge from START sweeping clockwise
  let ang = Math.atan2(dy, dx);
  if (ang < 0) ang += Math.PI * 2;
  // fraction along the gauge arc
  let rel = ang - START;
  if (rel < 0) rel += Math.PI * 2;
  const onArc = rel <= SWEEP;
  const band = dist <= ringOuter && dist >= ringInner;
  if (band && onArc) {
    const frac = rel / SWEEP;
    const ringCol = frac <= PROGRESS ? mix(CORAL, CORAL_HI, frac) : TRACK;
    // soft edges on the band
    const edge = Math.min(dist - ringInner, ringOuter - dist);
    const cov = smooth(edge, 0.006);
    col = over(col, withAlpha(ringCol, cov));
    // rounded cap at progress boundary
  }

  // terminal prompt ">" chevron + cursor underscore, centered
  const gT = 0.028; // half thickness
  let g = 999;
  g = Math.min(g, distSeg(x, y, 0.435, 0.40, 0.525, 0.50));
  g = Math.min(g, distSeg(x, y, 0.525, 0.50, 0.435, 0.60));
  let u = distSeg(x, y, 0.55, 0.605, 0.62, 0.605);
  const glyphCov = Math.max(smooth(gT - g, 0.006), smooth(0.022 - u, 0.006));
  if (glyphCov > 0) col = over(col, withAlpha(INK, glyphCov));

  return col;
}

function smooth(d, w) {
  // 1 when d>=w, 0 when d<=-w, smooth between
  if (d >= w) return 1;
  if (d <= -w) return 0;
  const t = (d + w) / (2 * w);
  return t * t * (3 - 2 * t);
}

function withAlpha(c, a) {
  return [c[0], c[1], c[2], c[3] * a];
}
function over(bg, fg) {
  const fa = fg[3] / 255;
  const ba = bg[3] / 255;
  const oa = fa + ba * (1 - fa);
  if (oa === 0) return [0, 0, 0, 0];
  const r = (fg[0] * fa + bg[0] * ba * (1 - fa)) / oa;
  const g = (fg[1] * fa + bg[1] * ba * (1 - fa)) / oa;
  const b = (fg[2] * fa + bg[2] * ba * (1 - fa)) / oa;
  return [r, g, b, oa * 255];
}
function roundedRectCoverage(x, y, x0, y0, x1, y1, r) {
  // signed distance to rounded rect; return soft coverage
  const hw = (x1 - x0) / 2, hh = (y1 - y0) / 2;
  const ccx = (x0 + x1) / 2, ccy = (y0 + y1) / 2;
  const qx = Math.abs(x - ccx) - (hw - r);
  const qy = Math.abs(y - ccy) - (hh - r);
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) - r;
  const inside = Math.min(Math.max(qx, qy), 0);
  const sd = outside + inside;
  return smooth(-sd, 0.006);
}

function render(size) {
  const SS = 3; // supersample
  const N = size * SS;
  const buf = Buffer.alloc(size * size * 4);
  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let acc = [0, 0, 0, 0];
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const nx = (px * SS + sx + 0.5) / N;
          const ny = (py * SS + sy + 0.5) / N;
          const c = sample(nx, ny);
          const a = c[3] / 255;
          acc[0] += c[0] * a;
          acc[1] += c[1] * a;
          acc[2] += c[2] * a;
          acc[3] += a;
        }
      }
      const n = SS * SS;
      const alpha = acc[3] / n;
      const o = (py * size + px) * 4;
      if (alpha <= 0) {
        buf[o] = buf[o + 1] = buf[o + 2] = buf[o + 3] = 0;
      } else {
        buf[o] = Math.round(acc[0] / acc[3]);
        buf[o + 1] = Math.round(acc[1] / acc[3]);
        buf[o + 2] = Math.round(acc[2] / acc[3]);
        buf[o + 3] = Math.round(alpha * 255);
      }
    }
  }
  return buf; // RGBA
}

// ---- PNG encode ----
function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0);
  return Buffer.concat([len, t, data, crc]);
}
function encodePNG(size, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  // raw with filter byte 0 per row
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", idat),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---- ICO (PNG-embedded entries) ----
function encodeICO(entries) {
  // entries: [{size, png}]
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2); // type icon
  header.writeUInt16LE(entries.length, 4);
  const dir = Buffer.alloc(16 * entries.length);
  let offset = 6 + dir.length;
  const blobs = [];
  entries.forEach((e, i) => {
    const b = i * 16;
    dir[b] = e.size >= 256 ? 0 : e.size;
    dir[b + 1] = e.size >= 256 ? 0 : e.size;
    dir[b + 2] = 0;
    dir[b + 3] = 0;
    dir.writeUInt16LE(1, b + 4);
    dir.writeUInt16LE(32, b + 6);
    dir.writeUInt32LE(e.png.length, b + 8);
    dir.writeUInt32LE(offset, b + 12);
    offset += e.png.length;
    blobs.push(e.png);
  });
  return Buffer.concat([header, dir, ...blobs]);
}

// ---- emit ----
const sizes = [16, 32, 48, 64, 128, 256];
const pngs = {};
for (const s of sizes) pngs[s] = encodePNG(s, render(s));

// Tauri-standard filenames
fs.writeFileSync(path.join(OUT, "32x32.png"), pngs[32]);
fs.writeFileSync(path.join(OUT, "128x128.png"), pngs[128]);
fs.writeFileSync(path.join(OUT, "128x128@2x.png"), pngs[256]);
fs.writeFileSync(path.join(OUT, "icon.png"), pngs[256]);
// tray state variants: reuse base for normal; warn/crit tinted at runtime not needed here
fs.writeFileSync(
  path.join(OUT, "icon.ico"),
  encodeICO([16, 32, 48, 64, 128, 256].map((s) => ({ size: s, png: pngs[s] })))
);

console.log("Icons written to", OUT);
