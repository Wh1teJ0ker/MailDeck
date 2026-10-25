// 版本单一事实源:src-tauri/Cargo.toml 的 [package].version。
// 本脚本把 package.json 的 version 同步为该值;
// --check 模式只校验不一致即报错(pnpm build 前置钩子)。
import { readFileSync, writeFileSync } from "node:fs";

const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
if (!m) {
  console.error("✗ src-tauri/Cargo.toml 中找不到 [package].version");
  process.exit(1);
}
const cargoVersion = m[1];

const pkgPath = "package.json";
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));

if (process.argv.includes("--check")) {
  if (pkg.version !== cargoVersion) {
    console.error(
      `✗ 版本不一致:Cargo.toml=${cargoVersion} package.json=${pkg.version}\n` +
        `  运行 pnpm version:set 让 package.json 跟随 Cargo.toml(单一事实源)`
    );
    process.exit(1);
  }
  console.log(`✓ 版本一致:${cargoVersion}`);
} else {
  if (pkg.version !== cargoVersion) {
    pkg.version = cargoVersion;
    writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
    console.log(`→ package.json version 已同步为 ${cargoVersion}`);
  } else {
    console.log(`✓ 已是最新:${cargoVersion}`);
  }
}
