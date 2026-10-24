# MailDeck 技术选型

确认日期：2026-10-24  
状态：已确认（第 6 节验证点条目以 M1 实测结果为准）  
决策范围：v0.1.0 起的全部开发工作

## 1. 总体结论

### 1.1 后端与技术底座

| 层级 | 选择 |
| --- | --- |
| 桌面框架 | Rust + Tauri v2（系统 WebView） |
| 异步运行时 | tokio |
| IMAP 客户端 | async-imap + tokio-rustls |
| 认证 | QQ 授权码（IMAP 登录） |
| 邮件解析 | mail-parser |
| HTML 安全清理 | ammonia |
| 本地存储 | SQLite（rusqlite，启用 bundled） |
| 检索 | SQLite FTS5 + trigram tokenizer（M1 实测确认，失败退回 LIKE） |
| 凭证保护 | keyring（macOS 走 Keychain） |
| 序列化 | serde / serde_json |
| 错误处理 | thiserror（库层）+ anyhow（应用层） |
| 日志 | tracing + tracing-subscriber（文件输出按天滚动，脱敏规则见 PLAN.md 第 8 节） |

### 1.2 前端

| 层级 | 选择 |
| --- | --- |
| 框架与构建 | React + TypeScript（Vite） |
| 样式 | Ant Design 主题定制（Design Token + 少量全局 CSS） |
| 组件 | Ant Design 5 |
| 图标 | @ant-design/icons |
| 服务端状态 | TanStack Query（封装 Tauri invoke 调用与缓存失效） |
| 本地 UI 状态 | Zustand |
| 长列表虚拟化 | TanStack Virtual |
| 后端类型绑定 | tauri-specta（由 Rust command 定义生成 TypeScript 类型；MVP 阶段以手写 `types.ts` 过渡，待引入） |
| 前端测试 | Vitest + Testing Library |
| 路由 | 首版不引入，视图间用状态切换；出现多窗口需求时再评估 React Router |

### 1.3 两条横切约定

- HTML 邮件正文渲染：经 ammonia 清理后的 HTML 以沙箱 iframe 注入方式展示，不进入 React 组件树；禁用脚本、表单与主动导航（对应 PLAN.md 第 5、8 节隔离要求）。
- 时间处理：不引入日期库；Rust 侧以 Unix 时间戳存储，前端用原生 Intl API 格式化显示；邮件头部时间解析由 mail-parser 承担。

## 2. 框架对比记录

结论：**Rust + Tauri**。对比于 2026-10-24 完成，候选为 Electron、Go + Wails、Rust + Tauri。

| 候选 | 结论 | 关键依据 |
| --- | --- | --- |
| Rust + Tauri | 采用 | 邮件解析生态质量与活跃度（mail-parser）；长期后台运行与不可信内容解析的内存安全；插件生态覆盖通知、托盘、自动更新；包体积与资源占用最小 |
| Go + Wails | 落选，保留为备选 | goroutine 管理多连接体验好、编译快，但 go-imap / go-message 维护放缓，边界场景需自行补齐；Wails 桌面能力插件生态较薄 |
| Electron | 落选 | 包体积与资源占用偏高；引入 mail-parser 后，Node 邮件库的优势不再关键 |

取舍说明：

- MailDeck 的核心负载是多账号 IMAP 长连接、复杂 MIME 与中文编码解析、长期后台运行，这三点 Rust 生态的保障最好。
- Rust 的开发效率劣势（编译速度、异步生命周期门槛）通过收敛 v0.1.0 范围来控制。
- 降级路径：若 M1 验证中 async-imap 无法满足 QQ 的实际行为，改用同步 `imap` crate 包一层 tokio 阻塞任务，框架层决策不变。

## 3. 服务商接入方案(v0.1.1:QQ 邮箱 + 163 邮箱)

| 项目 | QQ 邮箱 | 163 邮箱 |
| --- | --- | --- |
| 服务器 | `imap.qq.com:993` | `imap.163.com:993` |
| 协议 | IMAP over TLS | IMAP over TLS |
| 认证 | 授权码(网页版「设置 → 账户」生成，流程见 PLAN.md 4.1) | 客户端授权密码(网页版「设置 → POP3/SMTP/IMAP」开启服务并生成) |
| 登录后 ID 上报 | 不需要 | **必须**:登录后先发 IMAP `ID` 命令标识客户端，否则后续命令被拒(Unsafe Login);经 async-imap 原生 `session.id()` 实现 |
| 文件夹名 | 英文/中文混杂 | modified UTF-7 编码(如 `&XfJT0ZAB-` = 已发送)，须经解码再分类 |
| 账户类型 | QQ 邮箱地址 | 163/126 等网易邮箱地址 |
| 凭证保存 | 授权码 → keyring(`qq:<邮箱>`) | 授权码 → keyring(`163:<邮箱>`) |

服务商参数(端点、是否要求 ID)统一收敛在 provider 注册表(`provider/mod.rs` 的 REGISTRY),连接经 `provider::connect_for(provider_id, ...)` 分发;同步调度与 UI 不感知服务商差异。前端添加弹窗由注册表驱动，163 已可选。

后续服务商预告(不纳入当前版本):Outlook、Gmail 等需 OAuth2(XOAUTH2、浏览器授权、令牌刷新)，接入前须在本节登记方案并补验证点，OAuth2 相关依赖(oauth2、open 等)届时再引入。

163 专项验证点(M1.1,真实账号待实测，冒烟用例 `tests/provider_163.rs --ignored`):ID 上报后 SELECT/LIST/FETCH 全链路、UTF-7 文件夹解码与分类落地、授权码失效路径。

## 4. 依赖明细与理由

### 4.1 Rust 侧

| 用途 | 选择 | 理由与备注 |
| --- | --- | --- |
| IMAP 协议 | async-imap（启用 `runtime-tokio` feature，必须显式选择一个 runtime feature） | tokio 生态，支持自定义 authenticate 机制，IDLE 支持良好 |
| 异步工具 | futures | 消费 async-imap 的 UID FETCH / UID STORE 响应流 |
| TLS | tokio-rustls + rustls-native-certs | 证书源用系统证书库，避免内置根证书与实际环境差异；与 QQ 的握手兼容性是验证点 |
| 邮件解析 | mail-parser 0.9 系列 | Stalwart 出品，复杂 MIME、多编码（含中文）、内嵌结构覆盖最好，维护活跃；1.0 需 rustc ≥ 1.98，当前工具链 1.97.1，升级后可切回 1.x |
| 内嵌图片编码 | base64 | cid 内嵌图转 data URI 嵌入正文 |
| HTML 清理 | ammonia | 白名单式清理，剥除脚本/iframe/事件属性；保留邮件 CSS（<style> 与 style 属性，外联由 iframe 沙箱 + CSP 阻断）；规则在 M3 结合真实邮件验证 |
| 数据库 | rusqlite（bundled）+ rusqlite_migration | 单文件嵌入，避免运行时 SQLite 依赖；迁移脚本纳入版本控制 |
| 全文检索 | SQLite FTS5 + trigram | trigram 支持中文子串匹配；效果在 M1 用真实中文主题数据确认（见第 6 节） |
| 凭证 | keyring | macOS Keychain 的封装（首发仅 macOS；Windows / Linux 后端后续平台再启用） |
| Tauri 插件 | updater（已在 v0.1 设置中心引入，更新源待发布前配置）；notification、tray（v0.2 预留） | 对应 ROADMAP 各版本能力 |

### 4.2 前端侧

| 用途 | 选择 | 理由与备注 |
| --- | --- | --- |
| 样式 | Ant Design 主题定制 | 通过 Design Token 统一改色板、圆角、间距；不引入 Tailwind，少量全局 CSS 补齐 WebView 基础样式 |
| 组件 | Ant Design 5 | 组件全（Layout、List、Table、Badge、Modal、Menu 等），信息密集界面开箱即用，中文文档一流；三栏布局用 Layout + Sider 直接搭建 |
| 图标 | @ant-design/icons | 与组件库同源，风格统一 |
| 服务端状态 | TanStack Query | 账号、邮件列表、同步状态天然是「后端数据 + 缓存失效」模型，正好封装 Tauri invoke；重试、失效、加载态开箱即用 |
| 本地 UI 状态 | Zustand | 筛选器、当前选中邮件等轻量状态，避免 Context 样板代码 |
| 虚拟列表 | TanStack Virtual | 邮件列表千级条目流畅滚动；与 React 生态集成最好 |
| 类型绑定 | tauri-specta | Rust command 定义生成 TS 类型，前后端接口单一来源，不手写两份 |
| 前端测试 | Vitest + Testing Library | 与 Vite 同生态，启动快 |
| i18n | 手写模块（src/i18n，词典 + useT + {n} 插值） | 双语规模小,不值得引入 react-i18next;词典键位对齐有测试约束 |
| 设置持久化 | zustand persist(localStorage) | 语言与主题色本地记忆 |

### 4.3 依赖引入规则

新增任何依赖须先在本文档登记用途与理由；同类用途只保留一个库。

## 5. 平台与分发

| 项目 | 决定 |
| --- | --- |
| 首发操作系统 | macOS,优先 Apple Silicon(平台级 QA 基准) |
| 后续平台 | Windows 与 Linux 的**构建与发布**已随 CD 矩阵启用(NSIS/MSI、deb/AppImage/rpm);平台级运行 QA 随 M1 后排期,问题以 v0.1.x 修复 |
| 凭证后端 | keyring 已启用三平台后端(Keychain / Windows 凭据管理器 / libsecret) |
| 打包 | Tauri bundler,targets=all(macOS dmg、Windows NSIS+MSI、Linux deb/AppImage/rpm) |
| 签名与公证 | 更新包已用 minisign 私钥签名(仓库 Secrets);Apple Developer ID 签名与公证、Windows 代码签名仍待 M4 决定,未签名安装包首次打开需用户确认 |
| 自动更新 | 已上线:updater 插件 + GitHub Releases latest.json,覆盖三平台六种目标;v0.1.1 起实测客户端在线升级链路 |

## 6. 验证点（M1 实测通过后才视为确认）

| 验证点 | 风险 | 通过标准 |
| --- | --- | --- |
| rustls 与 QQ 的 TLS 握手 | 证书链或协议版本不兼容 | 可稳定建立 IMAPS 连接 |
| QQ IDLE 稳定性与频率限制 | 长连接被断开或触发风控 | 连续 24 小时 IDLE，重连退避有效 |
| mail-parser 中文与异常编码覆盖 | 乱码或解析崩溃 | 真实中文邮件样例回放通过；异常邮件被隔离，不阻塞账号 |
| FTS5 trigram 中文检索 | trigram 对中文无效或性能不可接受 | 中文主题、发件人可检索；若失败退回 LIKE 并在本节登记结论 |

## 7. 决策记录

| 日期 | 决策 | 背景 |
| --- | --- | --- |
| 2026-10-24 | 桌面框架采用 Rust + Tauri v2，放弃 Electron 与 Go + Wails | 三候选对比，见第 2 节 |
| 2026-10-24 | v0.1.0 同时适配 QQ 邮箱与 Outlook | 已由同日“范围回调为仅 QQ”决策取代 |
| 2026-10-24 | v0.1.0 范围回调为仅 QQ 邮箱，Outlook 转入后续服务商候选 | 聚焦首版闭环，避免首版同时引入 OAuth2 复杂度 |
| 2026-10-24 | 凭证只经 keyring 存取，不落普通文件 | PLAN.md 第 8 节安全要求 |
| 2026-10-24 | IMAP 客户端首选 async-imap，同步 imap crate 为降级路径 | IDLE 与 QQ 实际行为需实测，见第 6 节 |
| 2026-10-24 | 前端配套定型：Tailwind CSS + shadcn/ui、TanStack Query / TanStack Virtual、Zustand、tauri-specta、Vitest；首版不引入路由库 | 样式与组件部分已由同日“改用 Ant Design”决策取代，其余仍有效 |
| 2026-10-24 | 检索定案 FTS5 + trigram tokenizer，M1 实测确认，LIKE 为退路 | 中文子串检索需求 |
| 2026-10-24 | 首发平台确认 macOS（Apple Silicon 优先），Windows / Linux 按需后置 | 开发环境即 macOS；分发策略见第 5 节 |
| 2026-10-24 | 时间处理不引入日期库：Unix 时间戳存储 + 前端 Intl 显示 | 降低依赖面；邮件头部时间解析由 mail-parser 承担 |
| 2026-10-24 | MVP 实现简化：IMAP 每操作短连接、无自动轮询/IDLE（手动同步）、检索用 LIKE、附件仅展示元数据、tauri-specta 延后（手写 types.ts） | 优先跑通 v0.1.0 闭环；补齐计划随 M1 验证结论排定（ROADMAP 第 6 节） |
| 2026-10-24 | mail-parser 固定 0.9 系列；async-imap 启用 runtime-tokio feature | 1.0 的 MSRV 需 rustc 1.98（当前 1.97.1）；async-imap 无默认 runtime 时无法编译 |
| 2026-10-24 | Tauri 命令体统一经 spawn_blocking + block_on 执行（LocalBoxFuture 装箱） | async-imap 响应流非 Send，不能直接进入 Tauri 异步命令的 Send 约束 |
| 2026-10-24 | 添加账号改为按服务商类型选择：后端维护 provider 注册表，UI 按注册表渲染、未接入类型置灰 | 用户新需求；新服务商接入收敛为注册表登记 |
| 2026-10-24 | 在线更新提前至 v0.1.0 设置中心（tauri-plugin-updater）；更新源暂为占位地址，签名密钥已生成并入库配置 | 用户新需求；发布前须配置真实更新源 |
| 2026-10-24 | 前端视觉采用品牌渐变蓝（135° #3d8bfd→#1554e0，主色 #1677ff）卡片式设计语言 | 用户指定；初版色偏紫已按反馈改为真蓝色调，常量收敛在 src/theme.ts |
| 2026-10-24 | v0.1.0 提前引入多文件夹支持（发现/分类/按邮箱同步与未读统计），v0.3 规划相应收窄 | 用户要求；IMAP UTF-7 解码为手写实现（约 60 行，含单测），不引入额外依赖 |
| 2026-10-24 | 邮件 CSS 保留策略：<style> 块与 style 属性经 ammonia 白名单保留，脚本与外联由 iframe 沙箱 + CSP 双重阻断 | 邮件 HTML 无 CSS 渲染基本不可读；远程资源仍默认不加载 |
| 2026-10-24 | i18n 手写实现（zh-CN/en-US 双词典），主题色五预设经 CSS 变量切换；主题渐变/主色/光晕全部变量化 | 用户要求设置含语言与色彩；无新增运行时依赖 |
| 2026-10-24 | 代码托管 GitHub（公开仓库），CI/CD 用 GitHub Actions：发布用 tauri-action 构建 macOS 双架构并产出 Release 与更新源；日常测试按反馈移至本地 `pnpm verify` | 用户要求；更新源由此落定为 GitHub Releases latest.json |
| 2026-10-25 | 发布矩阵扩展至 Windows / Linux（keyring 三平台后端、targets=all、四 runner 矩阵）；createUpdaterArtifacts 开启，latest.json 与 .sig 产物上线 | 用户要求补全多平台与 CD;首个 tag v0.1.0 已实测全绿 |
| 2026-10-25 | v0.1.1 新增 163 邮箱适配（授权码 + 登录后 IMAP ID 上报 + UTF-7 文件夹），registry 增加 endpoint 字段，连接收敛为 connect_for 分发 | 用户要求;真实账号冒烟用例就绪待实测 |
| 2026-10-24 | 样式与组件改用 Ant Design 5（含 @ant-design/icons），弃用 Tailwind CSS + shadcn/ui | 用户偏好成品组件库；信息密集界面开箱即用，中文文档好 |

决策记录只追加不删除；推翻旧决策时在对应行“决策”列标注“已由某日期决策取代”。
