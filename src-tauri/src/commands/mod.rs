//! Tauri IPC 命令:薄层,串联存储、凭证与 IMAP provider。
//!
//! async-imap 的响应流不是 Send,无法直接进入 tauri 异步命令的 Future 约束;
//! 因此所有命令体统一放进 `spawn_blocking + block_on` 的阻塞线程执行,
//! 对前端而言仍是普通的异步调用。

use crate::error::{err, AppResult};
use crate::state::AppState;
use crate::store::{self, AccountRow, AttachmentRow};
use crate::{credentials, mail, provider, sync};

use std::sync::Arc;
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
async fn run_blocking<T: Send + 'static>(
    db: Arc<tokio::sync::Mutex<store::Store>>,
    f: impl FnOnce(Arc<tokio::sync::Mutex<store::Store>>) -> futures::future::LocalBoxFuture<'static, AppResult<T>> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(f(db))
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
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
        {
            let db = db.lock().await;
            if db.find_account_by_email(&email)?.is_some() {
                return Err(err("该邮箱已添加过,不能重复添加"));
            }
        }

        // 先验证连接与授权码,再落任何本地数据(v0.1 只有 QQ 一种连接实现)
        let mut sess = provider::qq::connect(&email, &auth_code).await?;
        sess.select(provider::qq::INBOX).await?;
        provider::qq::logout(sess).await;

        let display = display_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| email.split('@').next().unwrap_or("账号").to_string());
        let color = color
            .filter(|c| c.starts_with('#') && c.len() >= 4)
            .unwrap_or_else(|| "#1677ff".to_string());

        credentials::save_auth_code(&email, &auth_code)?;

        let db = db.lock().await;
        let id = db.insert_account(&email, &display, &color, ptype)?;
        let account = db.get_account(id)?;

        // 首次同步;失败不影响账号落库,错误展示在账号状态里
        let _ = sync::sync_account(&db, &account).await;
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        Ok(db.list_mailboxes(account_id)?)
    }))
    .await
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<AccountRow>> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        Ok(db.list_accounts()?)
    }))
    .await
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let account = db.get_account(id)?;
        db.delete_account(id)?;
        drop(db);
        let _ = credentials::delete_auth_code(&account.email);
        Ok(())
    }))
    .await
}

#[tauri::command]
pub async fn set_account_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> AppResult<()> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let account = db.get_account(id)?;
        Ok(sync::sync_account(&db, &account).await.into())
    }))
    .await
}

#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> AppResult<Vec<SyncOutcomeDto>> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let accounts = db.list_accounts()?;
        let mut outcomes = Vec::new();
        for account in accounts.iter().filter(|a| a.enabled) {
            outcomes.push(sync::sync_account(&db, account).await.into());
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let msg = db.get_message(id)?;
        let mailbox = db.get_mailbox(msg.mailbox_id)?;
        let account = db.get_account(mailbox.account_id)?;

        let cached = db.get_cached_body(id)?;
        let (text, html) = match cached {
            Some(v) if v.0.is_some() || v.1.is_some() => v,
            _ => {
                let code = credentials::load_auth_code(&account.email)?;
                let mut sess = provider::qq::connect(&account.email, &code).await?;
                let raw = provider::qq::fetch_raw_message(&mut sess, &mailbox.name_raw, msg.uid.max(0) as u32).await?;
                provider::qq::logout(sess).await;

                let parsed = mail::extract_body(&raw);
                db.cache_body(id, parsed.text.as_deref(), parsed.html.as_deref())?;
                db.replace_attachments(id, &parsed.attachments)?;
                db.set_message_attachment_flag(id, parsed.has_attachment)?;
                (parsed.text, parsed.html)
            }
        };

        if !msg.seen && mailbox.role == "inbox" {
            match mark_seen_on_server(&account, &mailbox.name_raw, msg.uid.max(0) as u32, true).await {
                Ok(()) => {
                    db.set_seen_local(id, true)?;
                }
                Err(e) => {
                    db.set_mailbox_error(mailbox.id, Some(&e.to_string()))?;
                }
            }
        }

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
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let msg = db.get_message(id)?;
        let mailbox = db.get_mailbox(msg.mailbox_id)?;
        let account = db.get_account(mailbox.account_id)?;

        mark_seen_on_server(&account, &mailbox.name_raw, msg.uid.max(0) as u32, seen).await?;
        db.set_seen_local(id, seen)?;
        Ok(())
    }))
    .await
}

#[tauri::command]
pub async fn unread_counts(state: State<'_, AppState>) -> AppResult<Vec<store::MailboxUnread>> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        Ok(db.mailbox_unread_counts()?)
    }))
    .await
}

async fn mark_seen_on_server(account: &AccountRow, mailbox_raw: &str, uid: u32, seen: bool) -> AppResult<()> {
    let code = credentials::load_auth_code(&account.email)?;
    let mut sess = provider::qq::connect(&account.email, &code).await?;
    provider::qq::set_seen_flag(&mut sess, mailbox_raw, &uid.to_string(), seen).await?;
    provider::qq::logout(sess).await;
    Ok(())
}

/// 一键已读:把当前视图(mailbox_id 为空 = 全部收件箱)的全部未读在服务端与本地同时标已读。
/// 按账号复用连接、按邮箱分组、UID 分块批写;单个账号失败不影响其余账号。
#[tauri::command]
pub async fn mark_all_seen(state: State<'_, AppState>, mailbox_id: Option<i64>) -> AppResult<u32> {
    run_blocking(state.db.clone(), move |db| Box::pin(async move {
        let db = db.lock().await;
        let rows = db.list_unseen_for_scope(mailbox_id)?;
        if rows.is_empty() {
            return Ok(0);
        }

        // 按 (账号, 邮箱) 分组
        use std::collections::BTreeMap;
        let mut grouped: BTreeMap<(i64, i64), Vec<(i64, u32)>> = BTreeMap::new();
        for r in &rows {
            grouped
                .entry((r.account_id, r.mailbox_id))
                .or_default()
                .push((r.message_id, r.uid));
        }

        let mut done: u32 = 0;
        let mut current_account: Option<i64> = None;
        let mut sess: Option<provider::qq::ImapSession> = None;

        for ((account_id, mailbox_id), items) in grouped {
            // 换账号时重建连接
            if current_account != Some(account_id) {
                if let Some(s) = sess.take() {
                    provider::qq::logout(s).await;
                }
                let account = db.get_account(account_id)?;
                let code = credentials::load_auth_code(&account.email)?;
                sess = Some(provider::qq::connect(&account.email, &code).await?);
                current_account = Some(account_id);
            }

            let mailbox = db.get_mailbox(mailbox_id)?;
            let uids: Vec<u32> = items.iter().map(|(_, uid)| *uid).collect();
            let mut succeeded: Vec<i64> = Vec::new();
            let mut last_err: Option<String> = None;

            for chunk in sync::uid_set_chunks(&uids, 500) {
                let result = match sess.as_mut() {
                    Some(s) => provider::qq::set_seen_flag(s, &mailbox.name_raw, &chunk, true).await,
                    None => continue,
                };
                match result {
                    Ok(()) => {
                        let chunk_uids: Vec<u32> =
                            chunk.split(',').filter_map(|p| p.parse().ok()).collect();
                        succeeded.extend(
                            items
                                .iter()
                                .filter(|(_, uid)| chunk_uids.contains(uid))
                                .map(|(id, _)| *id),
                        );
                    }
                    Err(e) => last_err = Some(e.to_string()),
                }
            }

            for id in &succeeded {
                db.set_seen_local(*id, true)?;
            }
            done += succeeded.len() as u32;
            if let Some(err) = last_err {
                db.set_mailbox_error(mailbox_id, Some(&err))?;
            }
        }

        if let Some(s) = sess.take() {
            provider::qq::logout(s).await;
        }
        Ok(done)
    }))
    .await
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
