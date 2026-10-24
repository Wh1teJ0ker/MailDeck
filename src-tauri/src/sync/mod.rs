//! 账号同步:发现邮箱(文件夹)、按邮箱增量拉取邮件头并入库。
//! MVP 每次同步建立短连接;常驻连接与 IDLE 在后续版本引入。

use crate::app_settings::CredCtx;
use crate::credentials;
use crate::mail;
use crate::provider::{self, MailboxRole};
use crate::store::{MailboxRow, NewMessage, Store};

/// 首次同步与单次增量同步的每邮箱邮件数上限。
pub const SYNC_LIMIT: usize = 200;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncOutcome {
    pub account_id: i64,
    pub new_messages: usize,
    pub error: Option<String>,
}

/// 构造 UID SEARCH 命令:无游标时全量,否则从游标之后开始。
pub fn uid_search_command(last_synced: u32) -> String {
    if last_synced == 0 {
        "ALL".to_string()
    } else {
        format!("UID {}:*", last_synced + 1)
    }
}

/// 从 UID 集合中选取需要同步的窗口:
/// 过滤掉已同步过的 UID,升序排列,超过上限时只保留最新的一端。
pub fn pick_uid_window(uids: &[u32], last_synced: u32, cap: usize) -> Vec<u32> {
    let mut newer: Vec<u32> = uids.iter().copied().filter(|u| *u > last_synced).collect();
    newer.sort_unstable();
    newer.dedup();
    if newer.len() > cap {
        newer.drain(..newer.len() - cap);
    }
    newer
}

/// 把 UID 列表切成逗号分隔的 uid-set,每块最多 cap 个(避免命令行过长)。
pub fn uid_set_chunks(uids: &[u32], cap: usize) -> Vec<String> {
    uids.chunks(cap)
        .map(|c| c.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(","))
        .collect()
}

/// IMAP modified UTF-7(如 "&XfJT0ZAB-")→ Unicode 显示名。
/// 已是 UTF-8 的名字原样透传;非法段原样保留,不 panic。
pub fn decode_utf7(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            if i + 1 < chars.len() && chars[i + 1] == '-' {
                out.push('&');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '-' {
                j += 1;
            }
            if j >= chars.len() {
                // 没有结束符:原样保留
                out.push('&');
                i += 1;
                continue;
            }
            let seg: String = chars[i + 1..j].iter().collect();
            match decode_utf7_segment(&seg) {
                Some(s) => out.push_str(&s),
                None => {
                    out.push('&');
                    out.push_str(&seg);
                    out.push('-');
                }
            }
            i = j + 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn utf7_b64(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'+' => Some(62),
        b',' => Some(63),
        _ => None,
    }
}

fn decode_utf7_segment(seg: &str) -> Option<String> {
    let bytes = seg.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut bits: u32 = 0;
    let mut nbits: u32 = 0;
    let mut out: Vec<u8> = Vec::new();
    for &c in bytes {
        let v = utf7_b64(c)?;
        bits = (bits << 6) | v;
        nbits += 6;
        while nbits >= 8 {
            nbits -= 8;
            out.push(((bits >> nbits) & 0xFF) as u8);
        }
    }
    if out.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = out.chunks(2).map(|p| u16::from_be_bytes([p[0], p[1]])).collect();
    String::from_utf16(&units).ok()
}

/// 同步单个账号:发现邮箱,然后逐个同步收件箱与常用文件夹。
pub async fn sync_account(store: &Store, cred: &CredCtx, account: &crate::store::AccountRow) -> SyncOutcome {
    match sync_inner(store, cred, account).await {
        Ok(n) => SyncOutcome { account_id: account.id, new_messages: n, error: None },
        Err(e) => {
            let msg = e.to_string();
            let _ = store.set_account_error(account.id, Some(&msg));
            SyncOutcome { account_id: account.id, new_messages: 0, error: Some(msg) }
        }
    }
}

async fn sync_inner(store: &Store, cred: &CredCtx, account: &crate::store::AccountRow) -> anyhow::Result<usize> {
    let code = credentials::load_auth_code(&cred.mode, &cred.dir, &account.email)?;
    let mut sess = provider::connect_for(account.provider.as_str(), &account.email, &code).await?;

    // 1. 发现邮箱(LIST),登记进本地库
    use futures::StreamExt;
    {
        let mut names = sess.list(Some(""), Some("*")).await?;
        while let Some(item) = names.next().await {
            let name = item?;
            let raw = name.name().to_string();
            if raw.is_empty() {
                continue;
            }
            let mut flags: Vec<String> = Vec::new();
            let mut noselect = false;
            for a in name.attributes() {
                match a {
                    async_imap::types::NameAttribute::NoSelect => noselect = true,
                    async_imap::types::NameAttribute::Sent => flags.push("\\Sent".into()),
                    async_imap::types::NameAttribute::Drafts => flags.push("\\Drafts".into()),
                    async_imap::types::NameAttribute::Trash => flags.push("\\Trash".into()),
                    async_imap::types::NameAttribute::Junk => flags.push("\\Junk".into()),
                    _ => {}
                }
            }
            if noselect {
                continue;
            }
            let display = decode_utf7(&raw);
            // 分类必须用解码后的名字:163 的 UTF-7 原始名(&XfJT0ZAB-)匹配不到中文启发式
            let role = provider::classify_mailbox(&display, &flags).as_str();
            let delim = name.delimiter().unwrap_or("/").to_string();
            store.upsert_mailbox(account.id, &raw, &display, role, &delim)?;
        }
    }

    // 2. 逐个同步常用文件夹(other 类不同步,避免全库拉取)
    let mut total = 0usize;
    let mut first_error: Option<anyhow::Error> = None;
    for mailbox in store.list_mailboxes(account.id)? {
        if mailbox.role == MailboxRole::Other.as_str() {
            continue;
        }
        match sync_mailbox(store, &mailbox, &mut sess).await {
            Ok(n) => total += n,
            Err(e) => {
                let _ = store.set_mailbox_error(mailbox.id, Some(&e.to_string()));
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }

    provider::qq::logout(sess).await;
    let _ = store.update_sync_state(account.id, first_error.as_ref().map(|e| e.to_string()).as_deref());
    match first_error {
        Some(e) => Err(e),
        None => Ok(total),
    }
}

async fn sync_mailbox(store: &Store, mailbox: &MailboxRow, sess: &mut provider::qq::ImapSession) -> anyhow::Result<usize> {
    use futures::StreamExt;

    let selected = sess.select(&mailbox.name_raw).await?;
    let uidvalidity = selected.uid_validity.unwrap_or(0) as i64;

    let (stored_uidvalidity, stored_last) = store.mailbox_cursor(mailbox.id)?;

    // UIDVALIDITY 变化:该邮箱的 UID 索引作废,重建(P1 要求)
    if let Some(prev) = stored_uidvalidity {
        if prev != uidvalidity {
            store.clear_mailbox_messages(mailbox.id)?;
        }
    }
    let base_last = if stored_uidvalidity == Some(uidvalidity) {
        stored_last.unwrap_or(0).max(0) as u32
    } else {
        0
    };

    let all_uids: Vec<u32> = sess
        .uid_search(uid_search_command(base_last))
        .await?
        .into_iter()
        .collect();
    let window = pick_uid_window(&all_uids, base_last, SYNC_LIMIT);

    if window.is_empty() {
        store.update_mailbox_sync(mailbox.id, Some(uidvalidity), Some(base_last as i64))?;
        return Ok(0);
    }

    let set = window.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    // 必须取完整 HEADER 段:HEADER.FIELDS 的响应段 async-imap 的 Fetch::header()/body() 都不解析
    let mut fetches = sess.uid_fetch(set, "(UID FLAGS BODY.PEEK[HEADER])").await?;

    let mut count = 0usize;
    let mut max_uid = base_last;
    while let Some(item) = fetches.next().await {
        let f = item?;
        let Some(uid) = f.uid else { continue };
        let seen = f.flags().any(|fl| matches!(fl, async_imap::types::Flag::Seen));
        let summary = f.header().or_else(|| f.body()).map(|b| mail::parse_header_summary(b));

        let (subject, sender, sender_addr, recipient, message_id, date) = match summary {
            Some(s) => (s.subject, s.from_name, s.from_addr, s.recipient, s.message_id, s.timestamp),
            None => (None, None, None, None, None, None),
        };

        store.upsert_message(&NewMessage {
            mailbox_id: mailbox.id,
            uid: uid as i64,
            uidvalidity,
            message_id,
            subject,
            sender,
            sender_addr,
            recipient,
            date,
            seen,
        })?;
        if uid > max_uid {
            max_uid = uid;
        }
        count += 1;
    }
    drop(fetches);

    store.update_mailbox_sync(mailbox.id, Some(uidvalidity), Some(max_uid as i64))?;
    Ok(count)
}
