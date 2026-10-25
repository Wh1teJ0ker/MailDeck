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

// 安装指南:版本号取自 tag,文件名与产物一致;禁止使用 emoji(项目规范)
const ver = tag.slice(1);
const B = "`";
const PLATFORM_HINT = [
  "---",
  "",
  "## 下载与安装",
  "",
  "| 平台 | 安装包 | 适用 |",
  "| --- | --- | --- |",
  `| macOS · Apple Silicon | ${B}MailDeck_${ver}_aarch64.dmg${B} | M 系列 Mac |`,
  `| macOS · Intel | ${B}MailDeck_${ver}_x64.dmg${B} | Intel Mac |`,
  `| Windows · x64 | ${B}MailDeck_${ver}_x64-setup.exe${B}(推荐)或 ${B}MailDeck_${ver}_x64_en-US.msi${B} | Windows 10/11 |`,
  `| Linux · x64 | ${B}MailDeck_${ver}_amd64.deb${B} / ${B}MailDeck_${ver}_amd64.AppImage${B} / ${B}rpm${B} | Debian 系 / 通用 / RHEL 系 |`,
  "",
  "**已安装旧版本?** 无需重新下载——打开应用,「设置 → 检查更新」即可在线升级到本版本。",
  "",
  "**首次运行提示**(当前版本未购买代码签名证书,系统提示属正常现象,仅首次出现):",
  "",
  "- **macOS**:双击 dmg 安装后,首次打开请**右键点击应用 → 打开 → 再点「打开」**;若仍被拦,到「系统设置 → 隐私与安全性」底部点「仍要打开」。",
  "- **Windows**:SmartScreen 弹窗时点**「更多信息」→「仍要运行」**;" + B + ".msi" + B + " 与 " + B + ".exe" + B + " 二选一即可。",
  "- **Linux**:AppImage 需先赋予执行权限(" + B + "chmod +x MailDeck_*.AppImage" + B + ")再运行;deb/rpm 用对应包管理器安装。",
  "",
  "**首次使用**:添加邮箱前,请先在邮箱网页版开启 IMAP 服务并生成授权码——QQ 邮箱在「设置 → 账户」,163 邮箱在「设置 → POP3/SMTP/IMAP」;应用内填入的是**授权码**(不是邮箱密码)。数据(账号配置、邮件缓存)全部保存在本机,数据库整库加密。",
].join("\n");

console.log(`${body}\n\n${PLATFORM_HINT}`);
