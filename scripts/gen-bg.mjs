// 生成两套全屏氛围画背景(参考 Sui-IB/InternalBeyond 的雨玻璃语言):
//   rain — 深蓝雨夜(默认)  mist — 浅蓝晨雾(亮色)
// 元素:渐变天光、雨痕、水珠(带高光)、雾团、蝶影、暗角。确定性伪随机,可重复生成。
import { writeFileSync, mkdirSync } from "node:fs";
import { createHash } from "node:crypto";

const W = 1600, H = 1000;
function rng(seed) {
  let h = parseInt(createHash("sha256").update(seed).digest("hex").slice(0, 12), 16);
  return () => { h = (h * 1664525 + 1013904223) >>> 0; return h / 4294967296; };
}

function butterfly(cx, cy, s, rot, op, fill) {
  return `<g transform="translate(${cx} ${cy}) rotate(${rot}) scale(${s})" opacity="${op}">
    <ellipse cx="-16" cy="0" rx="17" ry="26" fill="${fill}" transform="rotate(-24)"/>
    <ellipse cx="16" cy="0" rx="17" ry="26" fill="${fill}" transform="rotate(24)"/>
    <ellipse cx="-10" cy="14" rx="9" ry="13" fill="${fill}" transform="rotate(-10)"/>
    <ellipse cx="10" cy="14" rx="9" ry="13" fill="${fill}" transform="rotate(10)"/>
    <rect x="-2" y="-16" width="4" height="34" rx="2" fill="${fill}"/>
  </g>`;
}

function scene(theme) {
  const dark = theme === "rain";
  const rand = rng(theme);
  // 天光底:深蓝雨夜 / 浅蓝晨雾
  const base = dark
    ? `<linearGradient id="sky" x1="0" y1="0" x2="0.25" y2="1">
        <stop offset="0" stop-color="#0b1c38"/><stop offset="0.45" stop-color="#24466f"/>
        <stop offset="0.75" stop-color="#5b83b8"/><stop offset="1" stop-color="#8fb0d8"/>
      </linearGradient>
      <radialGradient id="glow" cx="0.5" cy="0.12" r="0.9">
        <stop offset="0" stop-color="#cfe2ff" stop-opacity="0.75"/>
        <stop offset="0.4" stop-color="#7ea6dd" stop-opacity="0.28"/>
        <stop offset="1" stop-color="#7ea6dd" stop-opacity="0"/>
      </radialGradient>`
    : `<linearGradient id="sky" x1="0" y1="0" x2="0.25" y2="1">
        <stop offset="0" stop-color="#dfe9f6"/><stop offset="0.5" stop-color="#b9cfe8"/>
        <stop offset="0.8" stop-color="#9db9d9"/><stop offset="1" stop-color="#c4d6ea"/>
      </linearGradient>
      <radialGradient id="glow" cx="0.5" cy="0.1" r="0.95">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.9"/>
        <stop offset="0.45" stop-color="#e6f0ff" stop-opacity="0.4"/>
        <stop offset="1" stop-color="#e6f0ff" stop-opacity="0"/>
      </radialGradient>`;
  const streakFill = dark ? "rgba(205,224,255,VAR)" : "rgba(255,255,255,VAR)";
  const butterflyFill = dark ? "#16304f" : "#5e79a0";

  let els = "";
  // 大雾团(柔)
  for (let i = 0; i < 7; i++) {
    const x = 100 + rand() * (W - 200), y = 80 + rand() * (H - 160);
    const rx = 180 + rand() * 260, ry = 90 + rand() * 150;
    const op = dark ? 0.05 + rand() * 0.08 : 0.16 + rand() * 0.14;
    els += `<ellipse cx="${x.toFixed(0)}" cy="${y.toFixed(0)}" rx="${rx.toFixed(0)}" ry="${ry.toFixed(0)}" fill="${dark ? "#a9c6ec" : "#ffffff"}" opacity="${op.toFixed(2)}" filter="url(#soft)"/>`;
  }
  // 斜雨痕(细长线,微倾斜)
  for (let i = 0; i < 46; i++) {
    const x = rand() * W, y = rand() * H, len = 60 + rand() * 190;
    const op = (dark ? 0.05 : 0.25) + rand() * (dark ? 0.2 : 0.3);
    const wd = (0.8 + rand() * 1.6).toFixed(1);
    els += `<line x1="${x.toFixed(0)}" y1="${y.toFixed(0)}" x2="${(x + len * 0.12).toFixed(0)}" y2="${(y + len).toFixed(0)}" stroke="${streakFill.replace("VAR", op.toFixed(2))}" stroke-width="${wd}" stroke-linecap="round"/>`;
  }
  // 水珠(圆 + 内高光)
  for (let i = 0; i < 60; i++) {
    const x = 30 + rand() * (W - 60), y = 30 + rand() * (H - 60);
    const r = 2 + rand() * 9;
    const op = dark ? 0.10 + rand() * 0.22 : 0.30 + rand() * 0.30;
    els += `<g opacity="${op.toFixed(2)}">
      <circle cx="${x.toFixed(0)}" cy="${y.toFixed(0)}" r="${r.toFixed(1)}" fill="${dark ? "#9dc0ea" : "#ffffff"}"/>
      <circle cx="${(x - r * 0.3).toFixed(1)}" cy="${(y - r * 0.35).toFixed(1)}" r="${(r * 0.32).toFixed(1)}" fill="#ffffff" opacity="${dark ? 0.8 : 0.95}"/>
    </g>`;
  }
  // 蝶影(4-5 只,低透明度)
  const spots = [[390, 210, 1.5, -18], [720, 340, 2.0, 14], [1080, 240, 1.7, -8], [560, 620, 2.6, 20], [1220, 560, 1.4, -26]];
  for (const [cx, cy, s, rot] of spots) {
    els += butterfly(cx, cy, s * (dark ? 1 : 1.1), rot, dark ? 0.34 : 0.20, butterflyFill);
  }
  // 顶部斜光带
  els += `<path d="M ${W * 0.30} 0 L ${W * 0.52} 0 L ${W * 0.30} ${H} L ${W * 0.12} ${H} Z" fill="${dark ? "rgba(210,228,255,0.10)" : "rgba(255,255,255,0.4)"}" filter="url(#soft)"/>`;
  els += `<path d="M ${W * 0.62} 0 L ${W * 0.70} 0 L ${W * 0.50} ${H} L ${W * 0.36} ${H} Z" fill="${dark ? "rgba(210,228,255,0.07)" : "rgba(255,255,255,0.3)"}" filter="url(#soft)"/>`;

  return `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
  <defs>${base}
    <filter id="soft" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="26"/></filter>
    <radialGradient id="vig" cx="0.5" cy="0.45" r="0.85">
      <stop offset="0.55" stop-color="#000000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="${dark ? 0.5 : 0.16}"/>
    </radialGradient>
  </defs>
  <rect width="${W}" height="${H}" fill="url(#sky)"/>
  <rect width="${W}" height="${H}" fill="url(#glow)"/>
  ${els}
  <rect width="${W}" height="${H}" fill="url(#vig)"/>
</svg>`;
}

mkdirSync("src/assets", { recursive: true });
writeFileSync("src/assets/bg-rain.svg", scene("rain"));
writeFileSync("src/assets/bg-mist.svg", scene("mist"));
console.log("bg-rain.svg & bg-mist.svg generated");
