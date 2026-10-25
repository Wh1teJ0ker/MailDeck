// 应用图标源:SVG 精绘(石墨玻璃 M,透明底)。修改 d-fuse-graphite.svg 后运行本脚本。
import sharp from "sharp";

await sharp("scripts/icon-candidates/d-fuse-graphite.svg", { density: 96 })
  .resize(1024, 1024)
  .png()
  .toFile("app-icon.png");
console.log("app-icon.png rendered from SVG");
