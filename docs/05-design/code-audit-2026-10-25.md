# 代码审计与架构优化(v0.1.1 开发期)

审计日期:2026-10-25  
范围:全仓(Rust 后端 8 模块 + React 前端 7 组件 + 工作流/文档)  
方法:通读源码 + 面向问题的静态检查(并发/资源/错误处理/数据完整性)+ 测试验证

## 结论总览

| 编号 | 严重度 | 问题 | 处理 |
| --- | --- | --- | --- |
| A-1 | 高(架构) | AppState 用全局 `Arc<Mutex<Store>>`,所有 IPC 命令在整个执行期(含 IMAP 网络等待)持有锁:同步/拉正文期间,列表查询等全部命令被阻塞,UI 后端假死 | **已修复**:AppState 改持 `db_path`,每个命令短生命周期开独立连接(WAL 多读并发 + busy_timeout 写等待),锁彻底移除 |
| A-2 | 中(隐患) | `PRAGMA foreign_keys=ON` 依赖 libsqlite3-sys bundled 编译旗标(`SQLITE_DEFAULT_FOREIGN_KEYS=1`)的隐式行为;换系统 SQLite 会静默失效,级联删除变孤儿数据 | **已修复**:Store::open 显式设置 foreign_keys/busy_timeout/synchronous;新增重开库级联回归测试 |
| A-3 | 中(性能) | TLS 连接器每次连接都重建:重复加载全量系统根证书 | **已修复**:OnceLock 缓存 TlsConnector(Arc 内核,进程级复用) |
| A-4 | 低(整洁) | `update_sync_state` 携带恒不使用的 `ok` 参数(死参数) | **已修复**:签名收敛为 (id, error) |
| A-5 | 低(记录) | 同步为每账号每操作短连接,无连接复用与 IDLE;账号多时同步耗时长 | **记录**:IDLE/常驻连接为既定后续项(ROADMAP),M1 验证后一并做 |
| A-6 | 低(记录) | `commands/mod.rs` 约 470 行,命令层与业务编排混在一起 | **记录**:当前规模可维护;命令数翻倍时拆分 commands/ 子模块 |
| A-7 | 低(记录) | mark_all_seen 在锁内执行多账号网络循环(A-1 的最坏实例) | **已随 A-1 修复**:锁移除后天然消除 |

## 已实施改动

1. `store.rs`:`Store::open` 显式设置 `foreign_keys=ON`、`busy_timeout(5s)`、`synchronous=NORMAL`(WAL 推荐);`update_sync_state` 签名收敛。
2. `state.rs` / `lib.rs`:AppState 持 `db_path: PathBuf`;启动时打开一次完成迁移后即释放。
3. `commands/mod.rs`:全部命令改为「按命令开短连接」,网络阶段不再持有任何共享锁。
4. `provider/qq.rs`:TLS 连接器进程级缓存。
5. 测试:新增重开库级联回归用例(`foreign_keys_cascade_survives_reopen`)。

## 验证

- `cargo test` 全量通过(含新回归用例);
- `pnpm verify` 全绿(前端 36 用例 + 构建 + Rust 全量)。

## 后续追加(同日)

| 编号 | 严重度 | 问题 | 处理 |
| --- | --- | --- | --- |
| A-8 | 高(安全) | 授权码本地文件模式为明文;邮件数据库(正文/主题/发件人)落盘明文 | **已修复**:凭证文件 AES-256-GCM(机器绑定密钥);邮件库切换 SQLCipher 整库加密,含 v0.1.0 明文库自动迁移与校验;落盘不泄露断言入测试 |

## 遗留与后续

- A-5(连接复用与 IDLE)依赖 M1 验证结论;
- 前端未做轮次重构(MessageReader 的失效 effect 写法偏巧,行为正确;组件数翻倍时统一处理);
- 审计发现的修复不改变任何对外行为,现有 44+36 测试为安全网。
