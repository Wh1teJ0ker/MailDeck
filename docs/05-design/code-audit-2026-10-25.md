# MailDeck 全量代码审计报告

审计日期：2026-10-25
审计范围：当前工作树中的 Rust/Tauri 后端、React 前端、SQLite/凭证处理、测试、构建配置、发布工作流及产品规划文档。
审计方式：逐模块静态检查、跨层调用链核对、现有自动化验证与风险分级。
本轮限制：未连接真实邮箱，未对原生 Tauri 窗口做交互验收；涉及 QQ/Gmail 服务端行为的结论仍需真实账号验证。

## 执行摘要

当前工作树存在多个直接影响邮件接收主流程、邮件完整性、已读状态和凭证可用性的缺陷。最优先问题是邮件列表查询被永久禁用；即使修复该问题，历史邮件分页、同步上限游标、已有邮件状态回收仍会导致数据不完整或过期。pnpm verify 通过并不能覆盖这些问题：现有测试主要验证纯函数、SQLite 查询和 API 参数映射，没有 React 主流程集成测试，真实邮箱测试也被忽略。

严重级别：P0 为首版核心流程不可用；P1 为数据完整性、账号可用性或核心状态正确性问题；P2 为条件性可靠性、安全边界、性能或较低影响功能问题。

| 编号 | 级别 | 发现 | 判断 |
| --- | --- | --- | --- |
| F-01 | P0 | 邮件列表查询初始即禁用，且无可达路径启用 | 静态确认 |
| F-02 | P1 | 增量邮件超过单次上限时，未同步 UID 会被游标永久跳过 | 静态确认 |
| F-03 | P1 | “加载更早邮件”只增加本地 LIMIT，无法从服务器补取历史 | 静态确认 |
| F-04 | P1 | 增量同步不刷新已存在邮件的状态，也不清理已移出文件夹的记录 | 静态确认 |
| F-05 | P1 | Gmail 正文读取把 refresh token 当作 access token 使用 | 静态确认 |
| F-06 | P1 | IMAP 已读写响应错误被吞掉，本地可能显示已成功而服务器未变更 | 静态确认 |
| F-07 | P1 | IMAP 请求无显式超时，多账号同步串行执行 | 静态确认 |
| F-08 | P1 | 凭证文件读取错误被当作空账号表，后续保存可能覆盖原凭证 | 静态确认 |
| F-09 | P1 | 凭证迁移有失败时仍切换全局模式，失败账号在当前模式下不可读 | 静态确认 |
| F-10 | P1 | 明文数据库升级在两次 rename 之间中断时没有自动恢复路径 | 静态确认 |
| F-11 | P2 | 打开邮件会下载完整原始邮件及所有附件，且没有总大小上限 | 静态确认 |
| F-12 | P2 | Gmail client ID 前后端设置 DTO 不一致，设置页不能回显已保存值 | 静态确认 |
| F-13 | P2 | 移除账号时忽略凭证删除失败，可能遗留授权凭证 | 静态确认 |
| F-14 | P2 | 自定义背景把整个字节数组展开为函数参数，大图会触发参数数量限制 | 静态确认 |
| F-15 | P2 | 凭证文件模式每次读写重复执行昂贵 PBKDF2，测试耗时异常 | 静态确认 |
| F-16 | P2 | 凭证文件十六进制字段长度为奇数时会 panic | 静态确认 |
| F-17 | P2 | SQLCipher 迁移通过拼接 SQL 路径，路径含单引号时失败 | 静态确认 |
| F-18 | P2 | 未读统计只基于本地已同步邮件，界面未表达其覆盖范围 | 静态确认 |
| F-19 | P2 | 数据库加密密钥来自可查询的机器标识，不是独立秘密 | 安全边界风险 |

## 缺陷详情

### F-01：邮件列表查询永久禁用

src/components/MessageList.tsx 中 limit 初值为 null，React Query 的 enabled 条件却要求 limit !== null。唯一的 setLimit 调用位于“加载更早邮件”按钮，而按钮又要求已有消息数达到上限；被禁用的查询返回空列表，因此按钮也不会出现。应用启动后无法进入邮件列表查询，显示空状态，即使本地数据库已有邮件也一样。

证据：src/components/MessageList.tsx:23,26-30,215-223。

### F-02：增量同步上限会永久跳过 UID

pick_uid_window 在新 UID 超过上限时从升序列表头部删掉旧 UID，只保留最新一段；同步随后把游标推进到该窗口中的最大 UID。比如游标为 100、新 UID 为 101–500、上限为 200，首次只保存 301–500 并将游标设为 500，之后的搜索从 501 开始，101–300 永远不会再同步。

现有测试 incremental_backlog_is_capped_to_latest 明确把“只取最新 200 封”作为预期，因此测试通过掩盖了数据遗漏。

证据：src-tauri/src/sync/mod.rs:29-38,233-238,249-281；src-tauri/tests/sync.rs:37-45。

### F-03：历史邮件加载只是扩大本地查询上限

前端“加载更早邮件”只增加 limit，请求始终使用 offset: 0；后端只查询本地 SQLite，并把 limit 限制在 500。首次同步只保留最新的配置数量，并将游标推进到当前最大 UID。现有设计没有按需从服务器拉取更旧 UID 的路径，因此同步范围外的历史邮件无法通过按钮加载。

证据：src/components/MessageList.tsx:26-29,215-223；src-tauri/src/commands/mod.rs:226-245；src-tauri/src/sync/mod.rs:233-281。

### F-04：已有邮件状态和离开文件夹的邮件不会收敛

当本地已有游标时，同步仅搜索 UID last+1:*，仅对返回的新 UID 执行 FETCH。它不会重新获取游标内邮件的 FLAGS，也不会对比邮箱当前 UID 集合与本地记录。因此其他客户端标记已读/未读后，本地状态会持续过期；邮件移出或删除后，原本地记录仍留在该文件夹。

这与 ROADMAP 中“其他客户端移走邮件后，本地收件箱经同步正确反映变化”的验收项不一致。

证据：src-tauri/src/sync/mod.rs:20-26,233-255；src-tauri/src/store.rs:538-565。

### F-05：Gmail 正文读取使用了错误令牌

open_message 从凭证库读取的是 Gmail refresh token，却调用 provider::connect_for。Gmail 分支把传入值直接交给 XOAUTH2，不会刷新 access token。同步及其他账号命令走 connect_account，会读取 refresh token 并刷新 access token；正文命令没有走这条路径。Gmail 邮件正文未缓存时，打开邮件会以 refresh token 尝试登录并失败。

证据：src-tauri/src/commands/mod.rs:267-273；src-tauri/src/provider/mod.rs:71-89,97-116；src-tauri/src/provider/gmail.rs:204-213。

### F-06：已读写命令可能报告假成功

async-imap 的 uid_store 返回 Stream<Item = Result<Fetch>>。当前 set_seen_flag 只判断流是否结束，不检查每个 item 的 Err；服务器拒绝或响应解析错误可能被忽略。set_seen 随后仍更新本地状态，打开邮件也可能把本地标为已读。批量已读又对每个 UID chunk 使用 if let Ok(()) 忽略失败，最终只返回数量，前端无法区分部分失败。

证据：src-tauri/src/provider/qq.rs:123-135；src-tauri/src/commands/mod.rs:319-329,658-680；async-imap 的 uid_store 返回类型见依赖 async-imap-0.10.4/src/client.rs:808-829。

### F-07：无网络超时且账号串行同步

IMAP TCP 连接及 LIST/SELECT/SEARCH/FETCH/STORE 操作没有显式超时。sync_all 按账号顺序逐个 await；如果某个服务器连接建立或响应长期挂起，后续账号在本次同步中都不会开始，刷新按钮会一直等待。数据库锁已移除，因此不会锁死所有 IPC，但“单账号故障不拖累其他账号”的目标仍不成立。

证据：src-tauri/src/provider/qq.rs:43-60；src-tauri/src/sync/mod.rs:147-152,182-200；src-tauri/src/commands/mod.rs:210-220。

### F-08：凭证读取失败后可能覆盖原文件

read_file_map 把所有错误（文件损坏、解密失败、口令状态异常、权限错误等）统一 unwrap_or_default 成空映射。文件模式新增授权码时，save_auth_code 会基于该空映射写回整个凭证文件；因此已存在但损坏/无法解密的文件可能被新账号写入覆盖，丢失其中所有旧凭证。文件写入也直接覆盖目标文件，不是原子替换，写入中断可能留下不完整文件。

证据：src-tauri/src/credentials.rs:23-25,95-113,126-135。

### F-09：部分凭证迁移失败仍切换全局存储模式

迁移逐账号搬运凭证，失败账号留在旧存储；循环结束后不论 failed 是否为空，都会把 credential_store 保存为新模式。之后 load_auth_code 只按新的全局模式读取，失败账号凭证虽然仍在旧存储，却无法正常使用。界面只提示失败数量；且再次选择当前新模式会跳过迁移分支，不能直接重试。

证据：src-tauri/src/commands/mod.rs:537-591；前端只展示失败数量见 src/components/SettingsPage.tsx:70-84。

### F-10：旧数据库迁移不是可恢复的原子切换

明文数据库升级先把原文件重命名为 .plain-bak，再把加密临时文件重命名为正式路径，最后删除备份。若进程在前两个 rename 之间退出，正式路径不存在；下次启动没有检查 .plain-bak 的恢复逻辑，Connection::open 会创建新库并初始化空 schema。若在删除备份前退出，则明文备份会留在磁盘。

证据：src-tauri/src/store.rs:164-167,255-263。

### F-11：读取邮件会拉取整个原始 RFC822 内容

正文使用 BODY.PEEK[] 获取整封邮件后交给解析器；这会把普通附件内容一起下载并读入内存。实现没有邮件总字节上限，内嵌图片的 4MB 限制只限制展示转换，不限制完整邮件响应本身。打开含大附件的邮件会造成高网络流量和内存占用，与“正文加载不自动下载普通附件”的规划不一致。

证据：src-tauri/src/provider/qq.rs:100-118；src-tauri/src/commands/mod.rs:271-277；src-tauri/src/mail/mod.rs:8-10,61-96。

### F-12：Gmail client ID 设置字段不会正确回显

TypeScript 的 AppSettingsDto 声明了 gmail_client_id，但 Rust 的同名 DTO 与 get_app_settings 返回值都没有此字段。设置页却从 getAppSettings() 读取它并传给 setGmailId，所以每次打开设置都无法回显已保存值，运行时会把状态设成 undefined。用户仍可重新输入并保存，但不能查看当前保存内容。

证据：src/types.ts:20-27；src-tauri/src/commands/mod.rs:345-352,392-403；src/components/SettingsPage.tsx:52-60,411-425。

### F-13：移除账号不保证删除凭证

账号数据库记录先被删除，之后凭证删除结果被 let _ 丢弃。文件模式在应用重启后默认锁定，删除时 read_file_map 会得到空映射，删除函数可能返回成功但未改动凭证文件；系统钥匙串错误也同样被忽略。数据库中的邮件缓存虽会级联删除，授权凭证仍可能作为孤儿留存。

证据：src-tauri/src/commands/mod.rs:150-160；src-tauri/src/credentials.rs:138-168。

### F-14：自定义背景大图可能无法显示

后端允许上传至 10MB 的 JPG/PNG；前端将文件字节转成 data URL 时使用 btoa(String.fromCharCode(...bytes))。把大数组展开为函数实参会超过 JavaScript 引擎参数数量限制并抛出 RangeError。在 Node/V8 中 256KiB 已可复现；10MB 上限远超安全范围，Tauri WebView 也不能依赖更高的实现上限。

证据：src/App.tsx:55-61,77-82；上传上限 src-tauri/src/commands/mod.rs:442-457。

### F-15：文件凭证模式重复执行 PBKDF2，延迟很高

每次解密先通过 verify_passphrase 派生一次密钥，再由 decrypt_b64_with_passphrase 再派生一次；每次写入生成 verifier 和密文时也各自派生。PBKDF2 配置为 210,000 次迭代，凭证操作会重复执行昂贵计算。当前 pnpm verify 中 file_mode_roundtrip_and_delete 单项耗时约 98.79 秒；多账号文件模式同步和迁移会进一步放大延迟。

证据：src-tauri/src/credentials.rs:48-60,95-107；src-tauri/src/crypto.rs:114-145,168-179；本轮 cargo test 输出。

### F-16：损坏的十六进制字段可触发 panic

hex_decode 以两字符步长切片 s[i..i+2]，没有先校验长度为偶数。凭证文件的 salt/verifier 被截断成奇数长度时，切片越界会 panic，而不是返回格式错误。该输入属于本地损坏文件场景，但解锁和凭证读取应具备健壮错误处理。

证据：src-tauri/src/credentials.rs:49-55,72-77。

### F-17：迁移 SQL 路径未转义

数据库迁移将临时文件路径直接插入单引号包围的 SQL 字符串。用户目录路径含单引号时，ATTACH DATABASE 语句会语法错误，旧库无法自动迁移；路径应通过安全参数机制或正确转义处理。

证据：src-tauri/src/store.rs:221-226。

### F-18：未读统计没有覆盖范围标记

未读计数只统计本地 messages 表。由于首轮同步受数量上限限制，服务端更旧范围的未读邮件不在本地表中；侧栏却直接展示为账号/全部未读总数，没有说明这是已同步范围内的计数，容易让用户把局部数量误认为服务端总数。

证据：src-tauri/src/store.rs:733-749；src/components/AccountSidebar.tsx:84-92,235-257。

### F-19：数据库密钥是机器标识派生值

SQLCipher 的密钥是机器标识与固定用途字符串的 SHA-256 派生结果；机器标识来自系统可查询值，获取失败时还回退到代码内固定常量。这可阻止简单地把数据库复制到不同机器后直接打开，但它不是由系统密钥库保管的秘密，不能等价描述为抵御本机文件窃取的强静态加密。若机器标识读取失败，固定 fallback 还会让不同安装得到相同派生输入。

证据：src-tauri/src/crypto.rs:8-70；SQLCipher 使用处 src-tauri/src/store.rs:168-173。这是安全边界风险，不是 SQLCipher 算法实现错误。

## 已知能力缺口与规划不一致

- 自动收信尚未实现：当前只有添加账号后的首次同步和用户手动刷新；没有后台定时轮询、IDLE 或启动/唤醒补同步逻辑。PLAN.md 描述运行期间自动同步目标，ROADMAP.md 同时将当前实现记为手动同步简化，需明确首版验收承诺。
- 服务商范围不一致：PLAN.md 与 ROADMAP.md 把首版定义为 QQ；技术栈文档描述 QQ/163；代码注册表还把 Gmail 标为可用。需确认当前产品范围，避免测试与权限审核漏掉实际可选服务商。
- Gmail OAuth 申请 https://mail.google.com/ scope，权限覆盖 Gmail 邮箱的广泛 IMAP 操作能力；若继续提供 Gmail，应将授权范围及用户说明纳入安全评审。
- 发布工作流只在 tag 上运行构建并直接发布，没有执行 pnpm test 或 Rust 测试。pnpm build 仅做版本校验、TypeScript 检查和 Vite 构建，不能替代行为测试。
- 前端没有 MessageList、MessageReader、AccountSidebar、SettingsPage 组件级集成测试；测试集中在工具函数、状态和 IPC 参数映射。邮件列表查询被禁用未被当前测试发现。
- QQ/163 真实邮箱冒烟、系统钥匙串测试被标记 ignored；本轮验证共忽略 4 项。没有 Gmail 真实账号集成测试。

## 验证结果

- pnpm verify：通过；前端 36 项、Rust 64 项通过，另有 4 项 ignored。
- pnpm build：通过；版本一致，TypeScript 与 Vite 构建成功。生产 JS bundle 约 1.04MB，Vite 提示 chunk 超过 500kB。
- Rust 编译产生 6 项 unused import/variable 警告。
- cargo fmt --manifest-path src-tauri/Cargo.toml -- --check：失败；多个后端源文件与测试文件未按 rustfmt 格式化。
- cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings：失败；19 项 lint/warning 被 -D warnings 提升为错误。
- 工作区在本轮审计开始前干净；审计后仅更新本报告，产品源码与配置未修改。

## 前轮架构专项审计记录

此前已完成的架构专项问题与处理状态保留如下。该轮验证记录为当时结果；当前全量验证以本报告上方结果为准。

| 编号 | 严重度 | 问题 | 处理 |
| --- | --- | --- | --- |
| A-1 | 高（架构） | AppState 全局 Arc<Mutex<Store>> 导致网络等待期间所有 IPC 命令阻塞 | 已修复：改为每命令短连接，网络期间不持有共享锁 |
| A-2 | 中（隐患） | SQLite 外键依赖 bundled 编译旗标 | 已修复：显式启用 foreign_keys，并补重开库级联测试 |
| A-3 | 中（性能） | 每次连接重建 TLS connector 和根证书 | 已修复：进程内缓存 TLS connector |
| A-4 | 低（整洁） | update_sync_state 存在未使用参数 | 已修复 |
| A-5 | 低（后续） | 每次操作建立短连接，暂无 IDLE/常驻连接 | 记录为后续项；真实账号验证后再评估 |
| A-6 | 低（整洁） | 命令模块承载较多业务编排 | 记录为规模增长后的拆分候选 |
| A-7 | 低（架构） | 批量已读原先在全局锁内执行网络循环 | 随 A-1 修复 |
| A-8 | 高（安全） | 邮件数据库和本地凭证文件明文落盘 | 已引入 SQLCipher 数据库加密与凭证文件加密；密钥威胁模型见 F-19 |

## 建议修复顺序

1. 修复邮件列表首次查询，使主界面能读到本地邮件。
2. 调整同步游标和历史分页，保证上限截断后仍可继续补齐，且不会跳过 UID。
3. 增加已存在 UID 的状态/删除同步和 IMAP 网络超时，验证多账号隔离。
4. 修复已读状态响应处理与 Gmail 正文授权路径。
5. 让凭证文件读写与迁移失败保持原数据可恢复，并在移除账号时处理凭证删除错误。
6. 补充组件级主流程测试、真实 QQ 双账号验证和发布前测试门禁。
