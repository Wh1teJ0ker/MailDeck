//! Tauri IPC 命令:薄层,串联存储、凭证与 IMAP provider。
//!
//! async-imap 的响应流不是 Send,无法直接进入 tauri 异步命令的 Future 约束;
//! 因此所有命令体统一放进 `spawn_blocking + block_on` 的阻塞线程执行,
//! 对前端而言仍是普通的异步调用。

use crate::app_settings::{self, AppSettings, CredentialMode};
use crate::error::{err, AppError, AppResult};
use crate::state::AppState;
use crate::store::{self, AccountRow, AttachmentRow};
use crate::{credentials, mail, provider, sync};

use tauri::State;

#[derive(Debug, serde::Serialize)]
pub struct SyncOutcomeDto {
    pub account_id: i64,
    pub new_messages: usize,
    pub error: Option<String>,
}

impl From<sync::SyncOutcome> for SyncOutcomeDto {
    fn from(o: sync::SyncOutcome) -> Self {
        Self {
            account_id: o.account_id,
            new_messages: o.new_messages,
            error: o.error,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BodyDto {
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub sender_addr: Option<String>,
    pub recipient: Option<String>,
    pub date: Option<i64>,
    pub seen: bool,
    pub mailbox_name: String,
    pub mailbox_role: String,
    pub account_name: String,
    pub account_color: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Vec<AttachmentRow>,
}

/// 在阻塞线程上运行包含 !Send 组件(如 async-imap 响应流)的异步任务。
/// 注意装箱类型用 LocalBoxFuture:内部 Future 不要求 Send,
/// Send 边界只落在闭包与返回值上,由 spawn_blocking 的线程负责执行。
/// 每个命令短生命周期开自己的 SQLite 连接(WAL 并发读 + busy_timeout),
/// 网络等待期间不持有任何共享锁。
async fn run_blocking<T: Send + 'static>(
    db_path: std::path::PathBuf,
    f: impl FnOnce(std::path::PathBuf) -> futures::future::LocalBoxFuture<'static, AppResult<T>> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(f(db_path))
    })
    .await
    .map_err(|e| err(format!("后台任务执行失败:{e}")))?
}

#[tauri::command]
pub async fn add_account(
    state: State<'_, AppState>,
    provider_type: Option<String>,
    email: String,
    auth_code: String,
    display_name: Option<String>,
    color: Option<String>,
) -> AppResult<AccountRow> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        // 先按类型校验:只有注册表中 available 的类型可添加
        let ptype = provider_type.as_deref().unwrap_or("qq");
        let meta = crate::provider::meta(ptype).ok_or_else(|| err(format!("未知邮箱类型:{ptype}")))?;
        if !meta.available {
            return Err(err(format!("{}暂未支持,敬请期待", meta.name)));
        }

        let email = email.trim().to_lowercase();
        if !email.contains('@') || email.len() < 5 {
            return Err(err("邮箱地址格式不正确"));
        }
        if db.find_account_by_email(&email)?.is_some() {
            return Err(err("该邮箱已添加过,不能重复添加"));
        }

        // 先验证连接再落任何本地数据;Gmail 走 OAuth,其余走授权码
        if ptype == "gmail" {
            let mut sess = provider::connect_for(ptype, &email, &auth_code).await?;
            sess.select(provider::qq::INBOX).await?;
            provider::qq::logout(sess).await;
        } else {
            let mut sess = provider::connect_for(ptype, &email, &auth_code).await?;
            sess.select(provider::qq::INBOX).await?;
            provider::qq::logout(sess).await;
        }

        let display = display_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| email.split('@').next().unwrap_or("账号").to_string());
        let color = color
            .filter(|c| c.starts_with('#') && c.len() >= 4)
            .unwrap_or_else(|| "#1677ff".to_string());

        credentials::save_auth_code(&cred.mode, &cred.dir, &email, &auth_code)?;

        let id = db.insert_account(&email, &display, &color, ptype)?;
        let account = db.get_account(id)?;

        // 首次同步;失败不影响账号落库,错误展示在账号状态里
        let _ = sync::sync_account(&db, &cred, &account).await;
        Ok(db.get_account(id)?)
    }))
    .await
}

#[tauri::command]
pub async fn list_providers() -> AppResult<Vec<crate::provider::ProviderMeta>> {
    Ok(crate::provider::registry().to_vec())
}

#[tauri::command]
pub async fn list_mailboxes(
    state: State<'_, AppState>,
    account_id: i64,
) -> AppResult<Vec<store::MailboxRow>> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        Ok(db.list_mailboxes(account_id)?)
    }))
    .await
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<AccountRow>> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        Ok(db.list_accounts()?)
    }))
    .await
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let account = db.get_account(id)?;
        db.delete_account(id)?;
        drop(db);
        let _ = credentials::delete_auth_code(&cred.mode, &cred.dir, &account.email);
        Ok(())
    }))
    .await
}

#[tauri::command]
pub async fn set_account_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> AppResult<()> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        db.set_account_enabled(id, enabled)?;
        Ok(())
    }))
    .await
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    id: i64,
    display_name: String,
    color: String,
) -> AppResult<AccountRow> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        db.get_account(id)?;
        let name = display_name.trim().to_string();
        if name.is_empty() {
            return Err(err("显示名称不能为空"));
        }
        if !color.starts_with('#') || color.len() < 4 {
            return Err(err("颜色格式不正确"));
        }
        db.update_account(id, &name, &color)?;
        Ok(db.get_account(id)?)
    }))
    .await
}

#[tauri::command]
pub async fn sync_account(state: State<'_, AppState>, id: i64) -> AppResult<SyncOutcomeDto> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let account = db.get_account(id)?;
        Ok(sync::sync_account(&db, &cred, &account).await.into())
    }))
    .await
}

#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> AppResult<Vec<SyncOutcomeDto>> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let accounts = db.list_accounts()?;
        let mut outcomes = Vec::new();
        for account in accounts.iter().filter(|a| a.enabled) {
            outcomes.push(sync::sync_account(&db, &cred, account).await.into());
        }
        Ok(outcomes)
    }))
    .await
}

#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    account_id: Option<i64>,
    mailbox_id: Option<i64>,
    unread_only: Option<bool>,
    query: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
) -> AppResult<Vec<store::MessageRow>> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        Ok(db.list_messages(
            account_id,
            mailbox_id,
            unread_only.unwrap_or(false),
            query.as_deref().unwrap_or(""),
            offset.unwrap_or(0),
            limit.unwrap_or(100).clamp(1, 500),
        )?)
    }))
    .await
}

/// 打开邮件:优先读缓存,否则按邮件所属邮箱经 IMAP 拉取并解析;
/// 收件箱内未读邮件在正文成功取得后回写已读(失败记录到邮箱状态,不阻塞阅读)。
#[tauri::command]
pub async fn open_message(state: State<'_, AppState>, id: i64) -> AppResult<BodyDto> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let msg = db.get_message(id)?;
        let mailbox = db.get_mailbox(msg.mailbox_id)?;
        let account = db.get_account(mailbox.account_id)?;

        let cached = db.get_cached_body(id)?;
        let has_cache = matches!(&cached, Some((t, h)) if t.is_some() || h.is_some());
        // 收件箱未读邮件:打开即需回写已读。拉正文与回写共用一条连接,
        // 避免一次操作触发两次 IMAP 登录(QQ 对登录频率有限制)。
        let needs_fetch = !has_cache;
        let needs_seen = !msg.seen && mailbox.role == "inbox";

        let (text, html) = if needs_fetch || needs_seen {
            let code = credentials::load_auth_code(&cred.mode, &cred.dir, &account.email)?;
            let mut sess = provider::connect_for(account.provider.as_str(), &account.email, &code).await?;

            let out = if needs_fetch {
                let raw = provider::qq::fetch_raw_message(&mut sess, &mailbox.name_raw, msg.uid.max(0) as u32).await?;
                let parsed = mail::extract_body(&raw);
                db.cache_body(id, parsed.text.as_deref(), parsed.html.as_deref())?;
                db.replace_attachments(id, &parsed.attachments)?;
                db.set_message_attachment_flag(id, parsed.has_attachment)?;
                (parsed.text, parsed.html)
            } else {
                cached.expect("已判定存在缓存")
            };

            if needs_seen {
                match provider::qq::set_seen_flag(&mut sess, &mailbox.name_raw, &msg.uid.max(0).to_string(), true).await {
                    Ok(()) => {
                        db.set_seen_local(id, true)?;
                    }
                    Err(e) => {
                        db.set_mailbox_error(mailbox.id, Some(&e.to_string()))?;
                    }
                }
            }
            provider::qq::logout(sess).await;
            out
        } else {
            cached.expect("已判定存在缓存")
        };

        let seen_now = db.get_message(id)?.seen;
        Ok(BodyDto {
            subject: msg.subject,
            sender: msg.sender,
            sender_addr: msg.sender_addr,
            recipient: msg.recipient,
            date: msg.date,
            seen: seen_now,
            mailbox_name: mailbox.name,
            mailbox_role: mailbox.role,
            account_name: account.display_name,
            account_color: account.color,
            text,
            html,
            attachments: db.list_attachments(id)?,
        })
    }))
    .await
}

#[tauri::command]
pub async fn set_seen(state: State<'_, AppState>, id: i64, seen: bool) -> AppResult<()> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let msg = db.get_message(id)?;
        let mailbox = db.get_mailbox(msg.mailbox_id)?;
        let account = db.get_account(mailbox.account_id)?;

        mark_seen_on_server(&cred, &account, &mailbox.name_raw, msg.uid.max(0) as u32, seen).await?;
        db.set_seen_local(id, seen)?;
        Ok(())
    }))
    .await
}

#[tauri::command]
pub async fn unread_counts(state: State<'_, AppState>) -> AppResult<Vec<store::MailboxUnread>> {
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        Ok(db.mailbox_unread_counts()?)
    }))
    .await
}

// ---------- 设置:凭证存储方式 ----------

#[derive(Debug, serde::Serialize)]
pub struct AppSettingsDto {
    pub credential_store: String,
    pub background: String,
    pub glass_tone: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CredentialSwitchResultDto {
    pub migrated: u32,
    pub failed: Vec<String>,
}

// ---------- 凭证口令(文件模式) ----------

#[derive(Debug, serde::Serialize)]
pub struct CredFileStatusDto {
    /// 文件是否存在
    pub exists: bool,
    /// 是否已在本会话中解锁(口令在内存)
    pub unlocked: bool,
}

/// 查询凭证文件状态与解锁状态。
#[tauri::command]
pub fn cred_file_status(state: State<'_, AppState>) -> CredFileStatusDto {
    let exists = state.app_dir.join("credentials.json").exists();
    CredFileStatusDto { exists, unlocked: credentials::runtime_passphrase().is_some() }
}

/// 用口令解锁凭证文件(验证块校验);成功后口令驻留内存,本会话免再输。
#[tauri::command]
pub fn unlock_cred_file(state: State<'_, AppState>, passphrase: String) -> AppResult<u32> {
    let path = state.app_dir.join("credentials.json");
    let map = credentials::read_file_map_pub(&state.app_dir, &passphrase)?;
    credentials::set_runtime_passphrase(Some(passphrase));
    Ok(map.len() as u32)
}

/// 忘记运行期口令(手动锁定)。
#[tauri::command]
pub fn lock_cred_file() {
    credentials::set_runtime_passphrase(None);
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppSettingsDto {
    let settings_path = state.app_dir.join("settings.json");
    let s = app_settings::load(&settings_path);
    AppSettingsDto {
        credential_store: s.credential_store.as_str().to_string(),
        background: s.background.clone(),
        glass_tone: s.glass_tone.clone(),
    }
}

/// 切换内置背景(或 custom:要求已上传过)。
#[tauri::command]
pub fn set_background(state: State<'_, AppState>, mode: String) -> AppResult<()> {
    if !app_settings::valid_background(&mode) {
        return Err(err(format!("未知背景模式:{mode}")));
    }
    if mode == "custom" && !state.app_dir.join("background.img").exists() {
        return Err(err("尚未上传自定义背景图片"));
    }
    let settings_path = state.app_dir.join("settings.json");
    let mut s = app_settings::load(&settings_path);
    s.background = mode;
    app_settings::save(&settings_path, &s)?;
    Ok(())
}

/// 上传自定义背景图片(jpg/png,≤10MB),写入应用数据目录。
#[tauri::command]
pub fn upload_background(state: State<'_, AppState>, data: Vec<u8>) -> AppResult<()> {
    if data.len() > 10 * 1024 * 1024 {
        return Err(err("图片超过 10MB,请压缩后重试"));
    }
    // 魔数校验:jpg(ff d8)或 png(89 50 4e 47)
    let ok = data.starts_with(&[0xff, 0xd8]) || data.starts_with(&[0x89, 0x50, 0x4e, 0x47]);
    if !ok {
        return Err(err("仅支持 JPG / PNG 图片"));
    }
    std::fs::write(state.app_dir.join("background.img"), &data)?;
    let settings_path = state.app_dir.join("settings.json");
    let mut s = app_settings::load(&settings_path);
    s.background = "custom".into();
    app_settings::save(&settings_path, &s)?;
    Ok(())
}

// ---------- Gmail(OAuth2) ----------

#[tauri::command]
pub fn get_gmail_client_id(state: State<'_, AppState>) -> String {
    app_settings::load(&state.app_dir.join("settings.json")).gmail_client_id
}

#[tauri::command]
pub fn set_gmail_client_id(state: State<'_, AppState>, client_id: String) -> AppResult<()> {
    let settings_path = state.app_dir.join("settings.json");
    let mut s = app_settings::load(&settings_path);
    s.gmail_client_id = client_id.trim().to_string();
    app_settings::save(&settings_path, &s)?;
    Ok(())
}

/// Gmail 完整添加流程:浏览器授权 → PKCE 换令牌 → 读取账号地址 →
/// refresh token 入凭证库 → 建号 → 首次同步。全程一个命令,前端等待即可。
#[tauri::command]
pub async fn add_account_gmail(state: State<'_, AppState>) -> AppResult<AccountRow> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    let client_id = cred.gmail_client_id.clone();
    if client_id.is_empty() {
        return Err(err("请先在设置中填写 Google OAuth 客户端 ID"));
    }
    run_blocking(state.app_dir.clone(), move |app_dir| {
        Box::pin(async move {
            let (access, refresh, email) = provider::gmail::authorize_and_exchange(&client_id).await?;
            let email = email.to_lowercase();

            let db = store::Store::open(&app_dir.join("maildeck.db"))?;
            if db.find_account_by_email(&email)?.is_some() {
                return Err(err("该邮箱已添加过,不能重复添加"));
            }
            credentials::save_auth_code(&cred.mode, &cred.dir, &email, &refresh)?;
            // access token 入运行期缓存,首次同步直接用
            provider::gmail::cache_put(
                &email,
                &access,
                std::time::Duration::from_secs(3000),
            );

            let display = email.split('@').next().unwrap_or("账号").to_string();
            let id = db.insert_account(&email, &display, "#188038", "gmail")?;
            let account = db.get_account(id)?;
            let _ = sync::sync_account(&db, &cred, &account).await;
            Ok(db.get_account(id)?)
        })
    })
    .await
}

/// 切换玻璃色调(dark/light),与背景解耦。
#[tauri::command]
pub fn set_glass_tone(state: State<'_, AppState>, tone: String) -> AppResult<()> {
    if !app_settings::valid_glass_tone(&tone) {
        return Err(err(format!("未知玻璃色调:{tone}")));
    }
    let settings_path = state.app_dir.join("settings.json");
    let mut s = app_settings::load(&settings_path);
    s.glass_tone = tone;
    app_settings::save(&settings_path, &s)?;
    Ok(())
}

/// 读取自定义背景图片字节(前端转 data URL)。
#[tauri::command]
pub fn read_custom_background(state: State<'_, AppState>) -> AppResult<Option<Vec<u8>>> {
    let p = state.app_dir.join("background.img");
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(std::fs::read(&p)?))
}

/// 切换凭证存储方式,并把所有已有账号的授权码迁移到新模式。
/// 单个账号迁移失败不影响其余(记入 failed,授权码留在原处)。
#[tauri::command]
pub async fn set_credential_store(
    state: State<'_, AppState>,
    mode: String,
    passphrase: Option<String>,
) -> AppResult<CredentialSwitchResultDto> {
    if mode != "keyring" && mode != "file" {
        return Err(err("未知的凭证存储方式"));
    }
    let new_mode = CredentialMode::from_str(&mode);
    run_blocking(state.app_dir.clone(), move |app_dir| {
        Box::pin(async move {
            let settings_path = app_dir.join("settings.json");
            let old = app_settings::load(&settings_path);
            let ctx = app_settings::cred_ctx(&app_dir);
            let db = store::Store::open(&app_dir.join("maildeck.db"))?;

            let mut migrated = 0u32;
            let mut failed: Vec<String> = Vec::new();
            if old.credential_store != new_mode {
                // 切到文件模式:先落口令(供文件加解密),切换完成前不落盘设置
                if new_mode == CredentialMode::File {
                    let pass = passphrase
                        .clone()
                        .ok_or_else(|| err("使用本地加密文件需先设置口令"))?;
                    if pass.len() < 6 {
                        return Err(err("口令至少 6 个字符"));
                    }
                    credentials::set_runtime_passphrase(Some(pass));
                }
                for account in db.list_accounts()? {
                    // 闭包已在阻塞线程上执行,不能嵌套 block_on;凭证三步迁移是纯同步操作
                    let result = (|| -> AppResult<()> {
                        let code = credentials::load_auth_code(&old.credential_store, &app_dir, &account.email)?;
                        credentials::save_auth_code(&new_mode, &app_dir, &account.email, &code)?;
                        credentials::delete_auth_code(&old.credential_store, &app_dir, &account.email)?;
                        Ok(())
                    })();
                    match result {
                        Ok(()) => migrated += 1,
                        Err(_) => failed.push(account.email),
                    }
                }
                // 切回 keyring 且全部迁移成功后清除口令;有失败则保留(以便用户处理)
                if new_mode == CredentialMode::Keyring && failed.is_empty() {
                    credentials::set_runtime_passphrase(None);
                    let _ = std::fs::remove_file(app_dir.join("credentials.json"));
                }
            }
            let mut persisted = app_settings::load(&settings_path);
            persisted.credential_store = new_mode;
            app_settings::save(&settings_path, &persisted)?;
            Ok(CredentialSwitchResultDto { migrated, failed })
        })
    })
    .await
}

async fn mark_seen_on_server(cred: &app_settings::CredCtx, account: &AccountRow, mailbox_raw: &str, uid: u32, seen: bool) -> AppResult<()> {
    let mut sess = provider::connect_account(&cred, &cred.gmail_client_id, account.provider.as_str(), &account.email).await?;
    provider::qq::set_seen_flag(&mut sess, mailbox_raw, &uid.to_string(), seen).await?;
    provider::qq::logout(sess).await;
    Ok(())
}

/// 一键已读:把当前视图(mailbox_id 为空 = 全部收件箱)的全部未读在服务端与本地同时标已读。
/// 按账号复用连接、按邮箱分组、UID 分块批写;单个账号失败不影响其余账号。
#[tauri::command]
pub async fn mark_all_seen(state: State<'_, AppState>, mailbox_id: Option<i64>) -> AppResult<u32> {
    let cred = app_settings::cred_ctx(&state.app_dir);
    run_blocking(state.db_path.clone(), move |path| Box::pin(async move {
        let db = store::Store::open(&path)?;
        let rows = db.list_unseen_for_scope(mailbox_id)?;
        if rows.is_empty() {
            return Ok(0);
        }

        // 按账号分组,账号内再按邮箱分组;单账号失败只记到该账号,不中断其他账号
        use std::collections::BTreeMap;
        let mut by_account: BTreeMap<i64, BTreeMap<i64, Vec<(i64, u32)>>> = BTreeMap::new();
        for r in &rows {
            by_account
                .entry(r.account_id)
                .or_default()
                .entry(r.mailbox_id)
                .or_default()
                .push((r.message_id, r.uid));
        }

        let mut done: u32 = 0;
        for (account_id, mailboxes) in by_account {
            match mark_account_seen(&db, &cred, account_id, mailboxes).await {
                Ok(n) => done += n,
                Err(e) => {
                    let _ = db.set_account_error(account_id, Some(&e.to_string()));
                }
            }
        }
        Ok(done)
    }))
    .await
}

/// 单账号的一键已读:一次连接,逐邮箱分块批写。
async fn mark_account_seen(
    db: &store::Store,
    cred: &app_settings::CredCtx,
    account_id: i64,
    mailboxes: std::collections::BTreeMap<i64, Vec<(i64, u32)>>,
) -> AppResult<u32> {
    let account = db.get_account(account_id)?;
    let mut sess = provider::connect_account(&cred, &cred.gmail_client_id, account.provider.as_str(), &account.email).await?;

    let mut done: u32 = 0;
    for (mailbox_id, items) in mailboxes {
        let mailbox = db.get_mailbox(mailbox_id)?;
        let uids: Vec<u32> = items.iter().map(|(_, uid)| *uid).collect();
        let mut succeeded: Vec<i64> = Vec::new();

        for chunk in sync::uid_set_chunks(&uids, 500) {
            if let Ok(()) =
                provider::qq::set_seen_flag(&mut sess, &mailbox.name_raw, &chunk, true).await
            {
                let set: std::collections::HashSet<u32> =
                    chunk.split(',').filter_map(|p| p.parse().ok()).collect();
                succeeded.extend(
                    items
                        .iter()
                        .filter(|(_, uid)| set.contains(uid))
                        .map(|(id, _)| *id),
                );
            }
        }

        for id in &succeeded {
            db.set_seen_local(*id, true)?;
        }
        done += succeeded.len() as u32;
    }

    provider::qq::logout(sess).await;
    Ok(done)
}

// ---------- 设置:检查更新与在线更新 ----------

#[derive(Debug, serde::Serialize)]
pub struct UpdateInfoDto {
    pub available: bool,
    pub version: String,
    pub notes: String,
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 检查更新;更新源未配置或网络失败时返回可读错误,前端原样展示。
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> AppResult<UpdateInfoDto> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| err(format!("初始化更新器失败:{e}")))?;

    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfoDto {
            available: true,
            version: update.version.clone(),
            notes: update.body.clone().unwrap_or_default(),
        }),
        Ok(None) => Ok(UpdateInfoDto {
            available: false,
            version: String::new(),
            notes: String::new(),
        }),
        Err(e) => Err(err(format!("检查更新失败:{e}"))),
    }
}

/// 下载并安装更新;安装完成后由前端引导重启。
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| err(format!("初始化更新器失败:{e}")))?;

    let update = updater
        .check()
        .await
        .map_err(|e| err(format!("检查更新失败:{e}")))?
        .ok_or_else(|| err("当前已是最新版本"))?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| err(format!("下载安装失败:{e}")))?;
    Ok(())
}

#[tauri::command]
pub fn relaunch(app: tauri::AppHandle) {
    app.restart();
}
