# MailDeck 开发流程

更新日期：2026-10-24

## 1. 总体流程

需求或想法 → 记入 ROADMAP（范围确认）→ 跨模块或高风险改动先写简要方案 → feature 分支实现 → 自验（测试 + 验收点）→ 文档同步 → 合入 main → 达到版本门槛时发布。

个人项目，流程以轻量可执行为原则，规则重点只有两条：**范围先文档后代码**、**变更必有记录**。

## 2. 分支与提交

| 场景 | 分支 | 示例 |
| --- | --- | --- |
| 功能 | `feature/<模块>-<内容>` | `feature/163-imap-id` |
| 修复 | `fix/<内容>` | `fix/uidvalidity-reset` |
| 文档 | `docs/<内容>` | `docs/tech-stack` |
| 重构 / 测试 / 杂务 | `refactor/` `test/` `chore/` | `refactor/sync-state-machine` |

- main 始终保持可构建、可运行。
- **推送远程的硬性前置条件:本地完整测试已完成且 `pnpm verify` 全绿。任何代码或文档变更,未通过本地全量校验前一律不得 `git push`**(文档-only 变更同样执行,校验成本约 1 分钟)。
- 提交信息使用 Conventional Commits：`feat` / `fix` / `docs` / `refactor` / `test` / `chore`，格式 `类型(范围): 描述`，例如 `feat(sync): UIDVALIDITY 变化时重建索引`。

## 3. 测试策略(本地执行)

**硬性规则：先写完全部测试文件，再写实现代码；测试与源码完全分离，源码文件内不写内嵌测试（Rust 不用 `#[cfg(test)]`，前端不用同文件 `*.test.tsx`）。**

**测试与构建检查全部在本地完成,远程不跑 CI**(GitHub 只保留 tag → Release 构建推送)。**推送门槛:任何变更 push 前,必须已完成完整本地测试且 `pnpm verify` 全绿——远程没有第二道防线,本地校验是唯一闸门,这条纪律不能破。** 合入前运行一键校验:

```bash
pnpm verify   # = 前端 Vitest + 前端构建(tsc+vite) + Rust cargo test
```

测试文件固定放在独立目录,与源码目录分开:

| 端 | 测试目录 | 说明 |
| --- | --- | --- |
| Rust | `src-tauri/tests/` | 集成测试，只经 `maildeck_lib` 公开 API 访问被测模块 |
| 前端 | `tests/`（仓库根目录） | Vitest 测试文件，`tests/**/*.test.{ts,tsx}` |

| 层级 | 覆盖范围 | 要求 |
| --- | --- | --- |
| 单元测试 | 邮件解析、HTML 清理、同步状态机、已读操作合并、数据库迁移 | 随实现一起提交 |
| 样例邮件回放 | 脱敏或构造的 .eml 样例，覆盖中文、异常编码、内嵌图片、大正文 | 解析相关改动必须跑 |
| 真实账号冒烟 | M1 起维护 QQ ×2 测试账号，验证连接、IDLE、已读同步 | 里程碑与每次发布前执行；以 `#[ignore]` 用例形式放在 `src-tauri/tests/`，显式指定才运行 |
| 手动验收 | 按 ROADMAP 当前版本的验收门槛逐项核对并记录 | 每版本发布前 |

## 4. 发布流程

1. ROADMAP 当前版本验收门槛全部通过。
2. 更新日志定稿：`04-changelog/v<版本号>.md` 补发布日期；随后新建下一开发版本的日志文件，并在 `04-changelog/README.md` 版本目录登记。
3. 版本号同步三处并保持一致：`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json`。
4. **本地一键校验 `pnpm verify` 全绿**（前端测试 + 前端构建 + Rust 测试）。
5. 提交后打 tag 并推送：`git tag v<版本号> && git push origin v<版本号>`。
6. GitHub Actions「Release」流水线自动构建 macOS（Apple Silicon / Intel）dmg，创建 GitHub Release 并附安装包与 `latest.json` 在线更新源（签名私钥存于仓库 Secrets `TAURI_SIGNING_PRIVATE_KEY`）。
7. Release 页核对产物；应用内「设置 → 检查更新」验证在线更新链路。

## 5. 文档同步规则（合入前完成）

| 变更类型 | 更新文档 | 时机 |
| --- | --- | --- |
| 用户可感知的功能或行为变更 | `04-changelog/` 当前版本文件 | 合入前 |
| 版本范围调整、里程碑状态变化 | ROADMAP.md | 范围决策时 |
| 引入或替换依赖、技术决策 | TECH_STACK.md（含第 7 节决策记录） | 决策时 |
| 流程规则调整 | DEVELOPMENT.md | 决策时 |
| 需求背景与总体设计变化 | PLAN.md | 随变更 |

## 6. 安全红线（全程有效）

- 授权码、refresh token、access token 不写入日志、调试输出或普通配置文件。
- 日志不记录邮件正文；错误信息脱敏后再输出或上报。
- 邮件 HTML 渲染前必须经 ammonia 清理；远程图片默认不加载。
- 凭证仅经 keyring 存取，任何代码路径不得绕过。

## 7. 工程结构约定（M2 首次搭建时执行）

```
MailDeck/
├── docs/                  # 本文档体系
├── src/                   # React + TypeScript 前端
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/      # Tauri IPC 命令，保持薄层
│   │   ├── provider/      # 服务商适配：mod.rs / qq.rs / outlook.rs
│   │   ├── sync/          # 同步调度、IDLE、重连与退避
│   │   ├── store/         # SQLite 访问与迁移
│   │   ├── credentials/   # keyring 封装
│   │   └── mail/          # 解析与 HTML 清理
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## 8. 开发环境

- Rust：rustup 管理 stable 工具链，另装 tauri-cli 与 cargo-sweep（`cargo install cargo-sweep`，用于第 9 节的产物清理）。
- 前端：Node.js + pnpm。
- 代码规范工具：Rust 用 rustfmt + clippy（随工具链自带）；前端用 ESLint + Prettier。
- 平台系统依赖按 Tauri 官方 prerequisites 文档安装。

## 9. 构建产物清理（Rust + Tauri）

Rust 与 Tauri 的构建中间产物集中在 `src-tauri/target/`（增量编译缓存、依赖编译产物、历史构建输出），长期不清理可达数十 GB。原则：**及时清、只清过多的部分、不堆积**。

| 时机 | 操作 | 说明 |
| --- | --- | --- |
| 日常开发收尾 / 长编译会话结束 | 在 `src-tauri/` 下执行 `cargo sweep --time 7` | 清除连续 7 天未使用的产物，保留增量编译基础 |
| 每次版本发布后 | 先把 `target/release/bundle/` 下的安装包转移归档，再 `cargo clean` | 全量清理；下次构建顺带验证干净构建可用 |
| 阈值触发 | `du -sh src-tauri/target` 超过 10 GB 时立即清理 | 无论处于哪个阶段 |
| 打包机 / CI（如有） | 不保留 `target/`，每次从干净状态构建 | 避免产物污染发布结果 |

注意：不要在每次 `cargo build` / `tauri dev` 之后直接 `cargo clean`——增量编译缓存是快速迭代的基础，清掉它会让每次构建都退化为全量重编；日常用 `cargo sweep` 清掉「过多的部分」即可，`cargo clean` 留给发布后的全量清理。
