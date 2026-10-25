// Release 说明生成:tag → docs/04-changelog/v<tag>.md
// 规则(见 docs/03-process/DEVELOPMENT.md 发布流程):
//   1. Release 页面说明必须使用对应版本更新日志的正文;
//   2. 日志文件缺失即失败——先定稿日志,再打 tag;
//   3. 末尾统一附加平台安装提示。
// 用法:node scripts/release-notes.mjs v0.1.1
import { readFileSync, existsSync } from "node:fs";

const tag = process.argv[2];
if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) {
  console.error("用法: node scripts/release-notes.mjs v<x.y.z>");
  process.exit(1);
}

const file = `docs/04-changelog/v${tag.slice(1)}.md`;
if (!existsSync(file)) {
  console.error(
    `✗ 缺少 ${file}:Release 说明必须来自对应版本的更新日志。\n` +
      `  请先定稿该文件(参照 04-changelog/README.md),再推送 ${tag}。`
  );
  process.exit(1);
}

const text = readFileSync(file, "utf8");
const lines = text.split("\n");

// 剥离文档管理头:首个标题行、空行、状态/发布日期行(Release 标题已含版本号)
let i = 0;
while (i < lines.length) {
  const l = lines[i].trim();
  if (l === "" || l.startsWith("# ") || l.startsWith("状态") || l.startsWith("发布日期")) {
    i += 1;
    continue;
  }
  break;
}
const body = lines.slice(i).join("\n").trim();

const PLATFORM_HINT =
  "---\n" +
  "**下载**:按平台选择安装包(macOS .dmg / Windows setup.exe 或 .msi / Linux .deb·AppImage·rpm)。" +
  "应用内「设置 → 检查更新」可在线升级。\n" +
  "- Windows 未做代码签名,首次运行 SmartScreen 提示时选「仍要运行」;\n" +
  "- macOS 未做公证,首次打开需右键 → 打开。";

console.log(`${body}\n\n${PLATFORM_HINT}`);
