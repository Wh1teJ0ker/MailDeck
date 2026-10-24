//! SQLite 本地存储:账号、邮箱(文件夹)、邮件索引、正文缓存、附件元数据。
//! Schema v2:邮件挂在邮箱(mailboxes)下,唯一键为 (mailbox_id, uidvalidity, uid);
//! 同步游标存于邮箱行。v1 旧库(邮件直接挂账号)打开时自动迁移进各账号收件箱。

use std::path::Path;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};

use crate::error::{err, AppResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountRow {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub color: String,
    pub provider: String,
    pub enabled: bool,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MailboxRow {
    pub id: i64,
    pub account_id: i64,
    /// IMAP 原始名(UTF-7 或 ASCII),用于 SELECT 等协议操作
    pub name_raw: String,
    /// 解码后的显示名
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MessageRow {
    pub id: i64,
    pub mailbox_id: i64,
    pub uid: i64,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub sender_addr: Option<String>,
    pub recipient: Option<String>,
    pub date: Option<i64>,
    pub seen: bool,
    pub has_attachment: bool,
}

#[derive(Debug, Clone)]
pub struct NewMessage {
    pub mailbox_id: i64,
    pub uid: i64,
    pub uidvalidity: i64,
    pub message_id: Option<String>,
    pub subject: Option<String>,
    pub sender: Option<String>,
    pub sender_addr: Option<String>,
    pub recipient: Option<String>,
    pub date: Option<i64>,
    pub seen: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AttachmentRow {
    pub id: i64,
    pub name: String,
    pub mime: String,
    pub size: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MailboxUnread {
    pub account_id: i64,
    pub mailbox_id: i64,
    pub role: String,
    pub unread: i64,
}

#[derive(Debug, Clone)]
pub struct UnseenRow {
    pub message_id: i64,
    pub mailbox_id: i64,
    pub account_id: i64,
    pub uid: u32,
}

const FRESH_SQL: &str = r#"
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    color TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'qq',
    enabled INTEGER NOT NULL DEFAULT 1,
    uidvalidity INTEGER,
    last_synced_uid INTEGER,
    last_sync_at INTEGER,
    last_error TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE mailboxes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name_raw TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'other',
    delimiter TEXT,
    uidvalidity INTEGER,
    last_synced_uid INTEGER,
    last_error TEXT,
    UNIQUE(account_id, name_raw)
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mailbox_id INTEGER NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
    uid INTEGER NOT NULL,
    uidvalidity INTEGER NOT NULL DEFAULT 0,
    message_id TEXT,
    subject TEXT,
    sender TEXT,
    sender_addr TEXT,
    recipient TEXT,
    date INTEGER,
    seen INTEGER NOT NULL DEFAULT 0,
    has_attachment INTEGER NOT NULL DEFAULT 0,
    UNIQUE(mailbox_id, uidvalidity, uid)
);

CREATE INDEX IF NOT EXISTS idx_messages_date ON messages(date DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_messages_mailbox ON messages(mailbox_id);

CREATE TABLE IF NOT EXISTS bodies (
    message_id INTEGER PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    text_body TEXT,
    html_body TEXT,
    fetched_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    mime TEXT NOT NULL,
    size INTEGER NOT NULL DEFAULT 0
);
"#;

pub struct Store {
    conn: Connection,
}

impl Store {
    /// 打开加密数据库;检测到 v0.1.0 的明文库时自动迁移(数据保留,原明文文件删除)。
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::open_inner(path, true)
    }

    /// 打开未加密数据库——仅用于构造迁移夹具与迁移读取,业务代码不得使用。
    pub fn open_plain(path: &Path) -> AppResult<Self> {
        Self::open_inner(path, false)
    }

    fn open_inner(path: &Path, encrypted: bool) -> AppResult<Self> {
        if encrypted && Self::is_legacy_plaintext(path)? {
            Self::migrate_plaintext_to_encrypted(path)?;
        }
        let conn = Connection::open(path)?;
        if encrypted {
            conn.execute_batch(&crate::crypto::sqlcipher_key_pragma())?;
            // 探针:密钥不匹配或文件损坏在此暴露,而不是等后续语句莫名失败
            conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
                .map_err(|_| err("本地数据库密钥校验失败:文件可能损坏或来自其他机器"))?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // 显式声明,不依赖 bundled SQLite 的编译旗标(SQLITE_DEFAULT_FOREIGN_KEYS)
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // WAL 下的推荐同步级别;多连接并发写时等待而非立刻报 busy
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    const SQLITE_MAGIC: &'static [u8] = b"SQLite format 3\0";


    fn is_legacy_plaintext(path: &Path) -> AppResult<bool> {
        use std::io::Read;
        if !path.exists() {
            return Ok(false);
        }
        let mut f = std::fs::File::open(path)?;
        let mut head = [0u8; 16];
        let n = f.read(&mut head)?;
        Ok(n >= 16 && &head[..16] == Self::SQLITE_MAGIC)
    }

    /// 明文库 → 加密库:sqlcipher_export 到临时文件,校验账号数一致后换名,明文原件删除。
    fn migrate_plaintext_to_encrypted(path: &Path) -> AppResult<()> {
        let tmp = path.with_extension("enc-tmp");
        let _ = std::fs::remove_file(&tmp);
        let old_count: i64;
        {
            let plain = Self::open_plain(path)?;
            old_count = plain
                .conn
                .query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))
                .unwrap_or(0);
            let key_hex: String = {
                let key = crate::crypto::derive_key("sqlcipher");
                key.iter().map(|b| format!("{b:02x}")).collect()
            };
            // 注意:不能用 SELECT user_version FROM main——SQLCipher 连接上返回假 0,
            // 必须用 PRAGMA main.user_version 读取
            let user_version: i64 = plain
                .conn
                .query_row("PRAGMA main.user_version", [], |r| r.get(0))
                .unwrap_or(0);
            plain.conn.execute_batch(&format!(
                "ATTACH DATABASE '{}' AS enc KEY \"x'{}'\";
                 SELECT sqlcipher_export('enc');",
                tmp.display(),
                key_hex
            ))?;
            plain.conn.execute_batch(&format!("PRAGMA enc.user_version = {user_version};"))?;
            let uv_after: i64 = plain
                .conn
                .query_row("PRAGMA enc.user_version", [], |r| r.get(0))
                .unwrap_or(-1);
            if uv_after != user_version {
                let _ = std::fs::remove_file(&tmp);
                return Err(err(format!(
                    "加密迁移失败:user_version 写入不生效(期望 {user_version},实际 {uv_after})"
                )));
            }
        }

        // 校验:新库能用密钥打开且账号数一致
        {
            let conn = Connection::open(&tmp)?;
            conn.execute_batch(&crate::crypto::sqlcipher_key_pragma())?;
            let new_count: i64 = conn
                .query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))
                .map_err(|e| err(format!("加密迁移校验失败:{e}")))?;
            if new_count != old_count {
                let _ = std::fs::remove_file(&tmp);
                return Err(err(format!(
                    "加密迁移校验失败:账号数不一致(旧 {old_count}/新 {new_count})"
                )));
            }
        }

        // 换名并删除明文原件(校验已通过,明文备份不应保留)
        let bak = path.with_extension("plain-bak");
        std::fs::rename(path, &bak)?;
        std::fs::rename(&tmp, path)?;
        std::fs::remove_file(&bak)?;
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
        Ok(())
    }

    fn migrate(&self) -> AppResult<()> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= 2 {
            return Ok(());
        }
        // 判断是否为 v1 旧库(有 accounts 且 messages 无 mailbox_id 列)
        let has_accounts = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='accounts'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;
        if has_accounts {
            self.migrate_v1_to_v2()?;
        } else {
            self.conn.execute_batch(FRESH_SQL)?;
        }
        self.conn.pragma_update(None, "user_version", 2)?;
        Ok(())
    }

    /// v1(邮件挂账号)→ v2(邮件挂邮箱):旧邮件整体迁入各账号的收件箱邮箱。
    fn migrate_v1_to_v2(&self) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute_batch(
            r#"
            CREATE TABLE mailboxes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                name_raw TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'other',
                delimiter TEXT,
                uidvalidity INTEGER,
                last_synced_uid INTEGER,
                last_error TEXT,
                UNIQUE(account_id, name_raw)
            );
            INSERT INTO mailboxes (account_id, name_raw, name, role, delimiter, uidvalidity, last_synced_uid)
            SELECT id, 'INBOX', 'INBOX', 'inbox', '/', uidvalidity, last_synced_uid FROM accounts;

            CREATE TABLE messages_v2 (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mailbox_id INTEGER NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
                uid INTEGER NOT NULL,
                uidvalidity INTEGER NOT NULL DEFAULT 0,
                message_id TEXT,
                subject TEXT,
                sender TEXT,
                sender_addr TEXT,
                recipient TEXT,
                date INTEGER,
                seen INTEGER NOT NULL DEFAULT 0,
                has_attachment INTEGER NOT NULL DEFAULT 0,
                UNIQUE(mailbox_id, uidvalidity, uid)
            );
            INSERT INTO messages_v2 (id, mailbox_id, uid, uidvalidity, message_id, subject, sender, sender_addr, recipient, date, seen, has_attachment)
            SELECT m.id, mb.id, m.uid, m.uidvalidity, m.message_id, m.subject, m.sender, m.sender_addr, m.recipient, m.date, m.seen, m.has_attachment
            FROM messages m JOIN mailboxes mb ON mb.account_id = m.account_id AND mb.role = 'inbox';
            DROP TABLE messages;
            ALTER TABLE messages_v2 RENAME TO messages;
            CREATE INDEX IF NOT EXISTS idx_messages_date ON messages(date DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_messages_mailbox ON messages(mailbox_id);
            "#,
        )?;
        tx.commit()?;
        Ok(())
    }

    // ---------- 账号 ----------

    fn row_to_account(r: &Row) -> rusqlite::Result<AccountRow> {
        Ok(AccountRow {
            id: r.get(0)?,
            email: r.get(1)?,
            display_name: r.get(2)?,
            color: r.get(3)?,
            provider: r.get(4)?,
            enabled: r.get::<_, i64>(5)? != 0,
            last_sync_at: r.get(7)?,
            last_error: r.get(8)?,
        })
    }

    const ACCOUNT_COLS: &'static str =
        "id, email, display_name, color, provider, enabled, uidvalidity, last_sync_at, last_error";

    pub fn insert_account(&self, email: &str, display_name: &str, color: &str, provider: &str) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO accounts (email, display_name, color, provider, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![email, display_name, color, provider, now()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn find_account_by_email(&self, email: &str) -> AppResult<Option<AccountRow>> {
        let sql = format!("SELECT {} FROM accounts WHERE email = ?1", Self::ACCOUNT_COLS);
        Ok(self.conn.query_row(&sql, params![email], Self::row_to_account).optional()?)
    }

    pub fn list_accounts(&self) -> AppResult<Vec<AccountRow>> {
        let sql = format!("SELECT {} FROM accounts ORDER BY id", Self::ACCOUNT_COLS);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::row_to_account)?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    pub fn get_account(&self, id: i64) -> AppResult<AccountRow> {
        let sql = format!("SELECT {} FROM accounts WHERE id = ?1", Self::ACCOUNT_COLS);
        self.conn
            .query_row(&sql, params![id], Self::row_to_account)
            .optional()?
            .ok_or_else(|| err(format!("账号不存在(id {id})")))
    }

    pub fn delete_account(&self, id: i64) -> AppResult<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_account_enabled(&self, id: i64, enabled: bool) -> AppResult<()> {
        self.conn.execute(
            "UPDATE accounts SET enabled = ?2 WHERE id = ?1",
            params![id, enabled as i64],
        )?;
        Ok(())
    }

    pub fn update_account(&self, id: i64, display_name: &str, color: &str) -> AppResult<()> {
        self.conn.execute(
            "UPDATE accounts SET display_name = ?2, color = ?3 WHERE id = ?1",
            params![id, display_name, color],
        )?;
        Ok(())
    }

    /// 记录账号级同步时间与错误(游标在邮箱行)。
    pub fn update_sync_state(&self, id: i64, error: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "UPDATE accounts SET last_sync_at = ?2, last_error = ?3 WHERE id = ?1",
            params![id, now(), error],
        )?;
        Ok(())
    }

    pub fn set_account_error(&self, id: i64, error: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "UPDATE accounts SET last_error = ?2 WHERE id = ?1",
            params![id, error],
        )?;
        Ok(())
    }

    // ---------- 邮箱(文件夹) ----------

    fn row_to_mailbox(r: &Row) -> rusqlite::Result<MailboxRow> {
        Ok(MailboxRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            name_raw: r.get(2)?,
            name: r.get(3)?,
            role: r.get(4)?,
        })
    }

    const MAILBOX_COLS: &'static str = "id, account_id, name_raw, name, role";

    /// 按原始名 upsert 邮箱;存在则更新显示名与角色。
    pub fn upsert_mailbox(
        &self,
        account_id: i64,
        name_raw: &str,
        name: &str,
        role: &str,
        delimiter: &str,
    ) -> AppResult<i64> {
        self.conn.execute(
            "INSERT INTO mailboxes (account_id, name_raw, name, role, delimiter)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(account_id, name_raw) DO UPDATE SET name = excluded.name, role = excluded.role",
            params![account_id, name_raw, name, role, delimiter],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM mailboxes WHERE account_id = ?1 AND name_raw = ?2",
            params![account_id, name_raw],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn list_mailboxes(&self, account_id: i64) -> AppResult<Vec<MailboxRow>> {
        let sql = format!(
            "SELECT {} FROM mailboxes WHERE account_id = ?1 ORDER BY CASE role
             WHEN 'inbox' THEN 0 WHEN 'sent' THEN 1 WHEN 'drafts' THEN 2
             WHEN 'trash' THEN 3 WHEN 'spam' THEN 4 ELSE 5 END, id",
            Self::MAILBOX_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![account_id], Self::row_to_mailbox)?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    pub fn get_mailbox(&self, id: i64) -> AppResult<MailboxRow> {
        let sql = format!("SELECT {} FROM mailboxes WHERE id = ?1", Self::MAILBOX_COLS);
        self.conn
            .query_row(&sql, params![id], Self::row_to_mailbox)
            .optional()?
            .ok_or_else(|| err(format!("邮箱不存在(id {id})")))
    }

    pub fn update_mailbox_sync(
        &self,
        id: i64,
        uidvalidity: Option<i64>,
        last_synced_uid: Option<i64>,
    ) -> AppResult<()> {
        self.conn.execute(
            "UPDATE mailboxes SET
                uidvalidity = COALESCE(?2, uidvalidity),
                last_synced_uid = COALESCE(?3, last_synced_uid),
                last_error = NULL
             WHERE id = ?1",
            params![id, uidvalidity, last_synced_uid],
        )?;
        Ok(())
    }

    pub fn set_mailbox_error(&self, id: i64, error: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "UPDATE mailboxes SET last_error = ?2 WHERE id = ?1",
            params![id, error],
        )?;
        Ok(())
    }

    pub fn mailbox_cursor(&self, id: i64) -> AppResult<(Option<i64>, Option<i64>)> {
        let row = self
            .conn
            .query_row(
                "SELECT uidvalidity, last_synced_uid FROM mailboxes WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<i64>>(1)?)),
            )
            .optional()?;
        Ok(row.unwrap_or((None, None)))
    }

    // ---------- 邮件 ----------

    fn row_to_message(r: &Row) -> rusqlite::Result<MessageRow> {
        Ok(MessageRow {
            id: r.get(0)?,
            mailbox_id: r.get(1)?,
            uid: r.get(2)?,
            subject: r.get(3)?,
            sender: r.get(4)?,
            sender_addr: r.get(5)?,
            recipient: r.get(6)?,
            date: r.get(7)?,
            seen: r.get::<_, i64>(8)? != 0,
            has_attachment: r.get::<_, i64>(9)? != 0,
        })
    }

    const MESSAGE_COLS: &'static str =
        "id, mailbox_id, uid, subject, sender, sender_addr, recipient, date, seen, has_attachment";

    pub fn upsert_message(&self, m: &NewMessage) -> AppResult<i64> {
        let id = self.conn.query_row(
            "INSERT INTO messages
                (mailbox_id, uid, uidvalidity, message_id, subject, sender, sender_addr, recipient, date, seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(mailbox_id, uidvalidity, uid) DO UPDATE SET
                subject = excluded.subject,
                sender = excluded.sender,
                sender_addr = excluded.sender_addr,
                recipient = excluded.recipient,
                date = excluded.date,
                seen = excluded.seen
             RETURNING id",
            params![
                m.mailbox_id,
                m.uid,
                m.uidvalidity,
                m.message_id,
                m.subject,
                m.sender,
                m.sender_addr,
                m.recipient,
                m.date,
                m.seen as i64
            ],
            |r| r.get::<_, i64>(0),
        )?;
        Ok(id)
    }

    pub fn set_message_attachment_flag(&self, id: i64, has: bool) -> AppResult<()> {
        self.conn.execute(
            "UPDATE messages SET has_attachment = ?2 WHERE id = ?1",
            params![id, has as i64],
        )?;
        Ok(())
    }

    /// 列表视图:mailbox_id 指定 → 该邮箱;否则收件箱视图(可再按账号过滤)。
    pub fn list_messages(
        &self,
        account_id: Option<i64>,
        mailbox_id: Option<i64>,
        unread_only: bool,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> AppResult<Vec<MessageRow>> {
        let mut sql = format!(
            "SELECT {} FROM messages m JOIN mailboxes b ON b.id = m.mailbox_id WHERE 1=1",
            Self::MESSAGE_COLS
                .split(", ")
                .map(|c| format!("m.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();

        match mailbox_id {
            Some(id) => {
                sql.push_str(" AND m.mailbox_id = ?");
                binds.push(rusqlite::types::Value::Integer(id));
            }
            None => {
                sql.push_str(" AND b.role = 'inbox'");
                if let Some(aid) = account_id {
                    sql.push_str(" AND b.account_id = ?");
                    binds.push(rusqlite::types::Value::Integer(aid));
                }
            }
        }
        if unread_only {
            sql.push_str(" AND m.seen = 0");
        }
        let q = query.trim();
        if !q.is_empty() {
            sql.push_str(" AND (m.subject LIKE ? ESCAPE '\\' OR m.sender LIKE ? ESCAPE '\\' OR m.sender_addr LIKE ? ESCAPE '\\')");
            let like = format!("%{}%", escape_like(q));
            for _ in 0..3 {
                binds.push(rusqlite::types::Value::Text(like.clone()));
            }
        }
        sql.push_str(" ORDER BY COALESCE(m.date, 0) DESC, m.id DESC LIMIT ? OFFSET ?");
        binds.push(rusqlite::types::Value::Integer(limit));
        binds.push(rusqlite::types::Value::Integer(offset));

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(binds), Self::row_to_message)?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    pub fn get_message(&self, id: i64) -> AppResult<MessageRow> {
        let sql = format!("SELECT {} FROM messages WHERE id = ?1", Self::MESSAGE_COLS);
        self.conn
            .query_row(&sql, params![id], Self::row_to_message)
            .optional()?
            .ok_or_else(|| err(format!("邮件不存在(id {id})")))
    }

    pub fn set_seen_local(&self, id: i64, seen: bool) -> AppResult<()> {
        self.conn.execute(
            "UPDATE messages SET seen = ?2 WHERE id = ?1",
            params![id, seen as i64],
        )?;
        Ok(())
    }

    /// 一键已读的范围查询:mailbox_id 指定 → 该邮箱全部未读;否则收件箱视图全部未读。
    /// 返回行按账号/邮箱/UID 排序,便于命令层分组批处理。
    pub fn list_unseen_for_scope(&self, mailbox_id: Option<i64>) -> AppResult<Vec<UnseenRow>> {
        let mut sql = String::from(
            "SELECT m.id, m.mailbox_id, b.account_id, m.uid
             FROM messages m JOIN mailboxes b ON b.id = m.mailbox_id
             WHERE m.seen = 0",
        );
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();
        match mailbox_id {
            Some(id) => {
                sql.push_str(" AND m.mailbox_id = ?");
                binds.push(rusqlite::types::Value::Integer(id));
            }
            None => sql.push_str(" AND b.role = 'inbox'"),
        }
        sql.push_str(" ORDER BY b.account_id, m.mailbox_id, m.uid");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(binds), |r| {
            Ok(UnseenRow {
                message_id: r.get(0)?,
                mailbox_id: r.get(1)?,
                account_id: r.get(2)?,
                uid: r.get::<_, i64>(3)? as u32,
            })
        })?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    pub fn clear_mailbox_messages(&self, mailbox_id: i64) -> AppResult<()> {
        self.conn.execute("DELETE FROM messages WHERE mailbox_id = ?1", params![mailbox_id])?;
        Ok(())
    }

    // ---------- 正文 / 附件 / 统计 ----------

    pub fn cache_body(&self, message_id: i64, text: Option<&str>, html: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO bodies (message_id, text_body, html_body, fetched_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(message_id) DO UPDATE SET
                text_body = excluded.text_body,
                html_body = excluded.html_body,
                fetched_at = excluded.fetched_at",
            params![message_id, text, html, now()],
        )?;
        Ok(())
    }

    pub fn get_cached_body(&self, message_id: i64) -> AppResult<Option<(Option<String>, Option<String>)>> {
        let row = self
            .conn
            .query_row(
                "SELECT text_body, html_body FROM bodies WHERE message_id = ?1",
                params![message_id],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn replace_attachments(&self, message_id: i64, items: &[(String, String, i64)]) -> AppResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM attachments WHERE message_id = ?1", params![message_id])?;
        for (name, mime, size) in items {
            tx.execute(
                "INSERT INTO attachments (message_id, name, mime, size) VALUES (?1, ?2, ?3, ?4)",
                params![message_id, name, mime, size],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_attachments(&self, message_id: i64) -> AppResult<Vec<AttachmentRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, mime, size FROM attachments WHERE message_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map(params![message_id], |r| {
            Ok(AttachmentRow { id: r.get(0)?, name: r.get(1)?, mime: r.get(2)?, size: r.get(3)? })
        })?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }

    /// 按邮箱的未读统计(前端聚合出账号/全部视图的未读数)。
    pub fn mailbox_unread_counts(&self) -> AppResult<Vec<MailboxUnread>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.account_id, m.mailbox_id, b.role, COUNT(*)
             FROM messages m JOIN mailboxes b ON b.id = m.mailbox_id
             WHERE m.seen = 0 GROUP BY m.mailbox_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(MailboxUnread {
                account_id: r.get(0)?,
                mailbox_id: r.get(1)?,
                role: r.get(2)?,
                unread: r.get(3)?,
            })
        })?;
        let out = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn escape_like(input: &str) -> String {
    input.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
