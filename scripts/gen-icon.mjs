// 生成 1024x1024 应用图标:蓝色渐变圆角方 + 立体白信封(翻盖 + 投影 + 底部高光)。
// macOS 规范:圆角半径约为边长 22.5%,四角透明。
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const W = 1024;
const H = 1024;
const R = 230; // 圆角半径(约 22.5%)

// 信封几何(居中,略宽的黄金比例)
const EX0 = 232,
  EX1 = 792,
  EY0 = 350,
  EY1 = 674;
const ER = 44; // 信封圆角
const FLAP_H = 168; // 翻盖高度
const LINE_W = 16; // 翻盖线宽

function sdRoundRect(x, y, x0, y0, x1, y1, r) {
  const rx = Math.max(x0 + r - x, x - x1 + r, 0);
  const ry = Math.max(y0 + r - y, y - y1 + r, 0);
  const outside = Math.sqrt(rx * rx + ry * ry) - r;
  if (outside > 0) return outside;
  // 内部:有符号距离为负(到最近边的距离取负)
  return -Math.min(x - x0, x1 - x, y - y0, y1 - y);
}

// 平滑阶梯(1px 抗锯齿)
function cover(sd) {
  return Math.min(1, Math.max(0, 0.5 - sd));
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

const rows = [];
for (let y = 0; y < H; y++) {
  const row = Buffer.alloc(1 + W * 4);
  row[0] = 0;
  for (let x = 0; x < W; x++) {
    // ---- 背景:圆角方 + 对角渐变(#4aa3ff → #1554e0) ----
    const bgA = cover(sdRoundRect(x, y, 0, 0, W, H, R));
    const t = (x / W + y / H) / 2;
    let r = lerp(74, 21, t);
    let g = lerp(163, 84, t);
    let b = lerp(255, 224, t);

    // 背景顶部高光(柔和的白色渐隐,增加体积感)
    const hl = Math.max(0, 1 - y / 380) * 0.18;
    r = lerp(r, 255, hl);
    g = lerp(g, 255, hl);
    b = lerp(b, 255, hl);

    // ---- 信封投影(向下偏移的深蓝软阴影) ----
    const sh = cover(sdRoundRect(x, y - 18, EX0, EY0 + 14, EX1, EY1 + 14, ER));
    const shA = sh * 0.35 * bgA;
    r = lerp(r, 10, shA);
    g = lerp(g, 30, shA);
    b = lerp(b, 90, shA);

    // ---- 信封主体(白,圆角) ----
    const env = cover(sdRoundRect(x, y, EX0, EY0, EX1, EY1, ER)) * bgA;
    // 信封表面轻微上下渐变(白 → #eef3ff)
    const er = lerp(255, 238, (y - EY0) / (EY1 - EY0));
    const eg = lerp(255, 243, (y - EY0) / (EY1 - EY0));
    const eb = lerp(255, 255, (y - EY0) / (EY1 - EY0));
    r = lerp(r, er, env);
    g = lerp(g, eg, env);
    b = lerp(b, eb, env);

    // ---- 翻盖 V 线(品牌蓝,粗线圆头感用双线渐变近似) ----
    if (env > 0) {
      const fy = y - EY0;
      if (fy >= 0 && fy <= FLAP_H + LINE_W) {
        const k = (EX1 - EX0) / 2 / FLAP_H; // 斜率:顶角指向中央
        const midX = (EX0 + EX1) / 2;
        const cy = Math.min(fy, FLAP_H);
        const leftX = EX0 + cy * k;
        const rightX = EX1 - cy * k;
        const distV = Math.min(Math.abs(x - leftX), Math.abs(x - rightX));
        // 超过翻盖高度后线向中间收拢(实际 V 顶点在 FLAP_H 处重合)
        const inFlap = fy <= FLAP_H || Math.abs(x - midX) < 40;
        if (inFlap) {
          const line = cover(distV - LINE_W / 2) * env;
          // 线色跟随背景渐变端点
          const lr = lerp(74, 21, t);
          const lg = lerp(163, 84, t);
          const lb = lerp(255, 224, t);
          r = lerp(r, lr, line);
          g = lerp(g, lg, line);
          b = lerp(b, lb, line);
        }
      }
    }

    const o = 1 + x * 4;
    row[o] = Math.round(r);
    row[o + 1] = Math.round(g);
    row[o + 2] = Math.round(b);
    row[o + 3] = Math.round(bgA * 255);
  }
  rows.push(row);
}

const raw = Buffer.concat(rows);
const idat = deflateSync(raw, { level: 9 });

function crc32(buf) {
  let crc = 0xffffffff;
  for (let n = 0; n < buf.length; n++) {
    let c = (crc ^ buf[n]) & 0xff;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8;
ihdr[9] = 6;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync("app-icon.png", png);
console.log(`app-icon.png written (${png.length} bytes)`);
