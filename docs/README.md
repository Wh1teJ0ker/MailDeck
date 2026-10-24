# MailDeck 文档体系

更新日期：2026-10-24

## 目录结构

```
docs/
├── README.md                # 文档导航与维护约定（本文件）
├── 01-product/              # 产品与规划
│   ├── PLAN.md              # 产品与技术总体规划
│   └── ROADMAP.md           # 版本规划与里程碑
├── 02-architecture/         # 技术与架构
│   └── TECH_STACK.md        # 技术选型与决策记录
├── 03-process/              # 流程与规范
│   └── DEVELOPMENT.md       # 开发流程
├── 04-changelog/            # 更新日志（一个版本一个文件）
│   ├── README.md            # 版本目录索引
│   ├── v0.1.0.md            # v0.1.0（已发布）
│   └── v0.1.1.md            # v0.1.1 变更记录（开发中）
└── 05-design/               # 设计与技术文档
    └── code-audit-2026-10-25.md  # 代码审计与架构优化报告
```

## 文档职责

| 文档 | 定位 | 维护时机 |
| --- | --- | --- |
| [01-product/PLAN.md](01-product/PLAN.md) | 产品定位、需求背景、页面结构、同步与存储总体设计 | 需求或总体方案变化时 |
| [01-product/ROADMAP.md](01-product/ROADMAP.md) | 各版本范围、验收门槛与里程碑状态 | 版本范围调整、里程碑推进时 |
| [02-architecture/TECH_STACK.md](02-architecture/TECH_STACK.md) | 已确认技术栈、依赖选型、验证点与决策记录 | 引入或替换依赖、技术决策变化时 |
| [03-process/DEVELOPMENT.md](03-process/DEVELOPMENT.md) | 分支、提交、测试、发布、文档同步与构建产物清理规则 | 流程规则调整时 |
| [04-changelog/README.md](04-changelog/README.md) | 版本目录索引：每个版本的状态、日期与变更摘要 | 新版本文件建立、版本发布时 |
| [04-changelog/v0.1.0.md](04-changelog/v0.1.0.md) 等版本文件 | 单个版本的详细变更记录（新增 / 变更 / 修复） | 每次合入变更时 |

## 扩展约定

- `05-design/`：设计文档（同步机制、数据库结构、适配层接口、代码审计报告等），一事一文，文件名用小写中划线（如 `sync-design.md`、`code-audit-2026-10-25.md`）。
- `06-verification/`：验证记录（服务商实测、兼容性回放），按里程碑命名（如 `m1-qq-imap.md`）。
- 目录编号只增不改；新目录建立时在本文件登记。

## 维护约定

- 技术栈与版本范围以 TECH_STACK.md、ROADMAP.md 为准；PLAN.md 中与此冲突的旧建议作废。
- 每次合入变更前，按 DEVELOPMENT.md 第 5 节的对照表完成文档同步，文档不同步不合入。
- 技术决策记录（ADR）统一登记在 TECH_STACK.md 第 7 节，条目增多后再考虑拆分为独立目录。
- 更新日志按版本归档：条目写入 `04-changelog/` 当前开发版本的文件，发布时定稿并在版本目录登记，规则见 [04-changelog/README.md](04-changelog/README.md)。

## 当前状态

- 当前版本：v0.1.0，开发中；MVP 代码已落地（Rust + Tauri 2 后端 + React 三栏界面），下一步为 M1 真实账号验证。
- v0.1.0 范围：QQ 邮箱（授权码）多账号的接收、阅读与基础管理闭环；Outlook 等服务商延后（见 TECH_STACK.md 决策记录）。
- 运行开发版：`pnpm tauri dev`；测试：`cargo test`（src-tauri）与 `pnpm test`；真实账号冒烟：`cargo test --test provider_qq -- --ignored`（需设置 MAILDECK_QQ_EMAIL / MAILDECK_QQ_AUTH_CODE）。
