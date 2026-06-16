// Generate a simple, on-brand source icon (1024×1024 PNG) for the porter:
// a Verdure-green rounded square with the brand dot. No image libraries — emits
// a valid RGBA PNG with zlib. Feed the output to `tauri icon` to produce the
// platform icon set. Placeholder-grade but clean; replace with real art later.

import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const S = 1024;
const out = join(dirname(fileURLToPath(import.meta.url)), "..", "icon-source.png");

// Palette (matches app/src/styles.css).
const BG = [31, 122, 77]; // --green
const BG2 = [21, 92, 57]; // --green-deep (subtle vertical shade)
const DOT_LIGHT = [76, 194, 131];
const WHITE = [255, 255, 255];

const radius = 180; // rounded-corner radius
const cx = S / 2;
const cy = S / 2;
const dotR = 250; // brand dot radius

function inRoundedSquare(x, y) {
  const r = radius;
  if (x >= r && x <= S - r) return y >= 0 && y < S;
  if (y >= r && y <= S - r) return x >= 0 && x < S;
  // corners
  const cxs = x < r ? r : S - r;
  const cys = y < r ? r : S - r;
  return (x - cxs) ** 2 + (y - cys) ** 2 <= r * r;
}

// Build raw RGBA with a per-row filter byte (0 = none).
const raw = Buffer.alloc((S * 4 + 1) * S);
let p = 0;
for (let y = 0; y < S; y++) {
  raw[p++] = 0; // filter type
  for (let x = 0; x < S; x++) {
    let rgba;
    if (!inRoundedSquare(x, y)) {
      rgba = [0, 0, 0, 0]; // transparent outside the rounded square
    } else {
      const dist2 = (x - cx) ** 2 + (y - cy) ** 2;
      if (dist2 <= dotR * dotR) {
        // brand dot, with a soft top-left highlight like the CSS radial-gradient
        const hx = cx - dotR * 0.35;
        const hy = cy - dotR * 0.35;
        const hd = Math.sqrt((x - hx) ** 2 + (y - hy) ** 2) / dotR;
        const t = Math.max(0, Math.min(1, hd));
        rgba = [
          Math.round(DOT_LIGHT[0] * (1 - t) + WHITE[0] * 0.15 + 255 * 0 * t),
          Math.round(DOT_LIGHT[1] * (1 - t) + 122 * t),
          Math.round(DOT_LIGHT[2] * (1 - t) + 77 * t),
          255,
        ];
      } else {
        const v = y / S;
        rgba = [
          Math.round(BG[0] * (1 - v) + BG2[0] * v),
          Math.round(BG[1] * (1 - v) + BG2[1] * v),
          Math.round(BG[2] * (1 - v) + BG2[2] * v),
          255,
        ];
      }
    }
    raw[p++] = rgba[0];
    raw[p++] = rgba[1];
    raw[p++] = rgba[2];
    raw[p++] = rgba[3];
  }
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

// CRC32 (PNG spec).
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync(out, png);
console.log("Wrote", out, `(${png.length} bytes)`);
