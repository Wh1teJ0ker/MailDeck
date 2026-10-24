//! SQLite 存储层集成测试(临时数据库):多邮箱模型 + 旧库迁移。
//! 运行:`cargo test --test store`

use maildeck_lib::store::{NewMessage, Store};
use std::path::PathBuf;

fn temp_store(tag: &str) -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!("maildeck-test-{}-{}.db", tag, std::process::id()));
    cleanup(&path);
    let store = Store::open(&path).expect("打开临时数据库失败");
    (store, path)
}

fn cleanup(path: &PathBuf) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
    }
}

/// 建账号 + 收件箱邮箱,返回 (store, path, account_id, inbox_id)。
fn setup_account(s: &Store, email: &str) -> (i64, i64) {
    let a = s.insert_account(email, email, "#111111", "qq").unwrap();
    let inbox = s
        .upsert_mailbox(a, "INBOX", "INBOX", "inbox", "/")
        .unwrap();
    (a, inbox)
}

fn msg(mailbox_id: i64, uid: i64, subject: &str, sender: &str, date: i64, seen: bool) -> NewMessage {
    NewMessage {
        mailbox_id,
        uid,
        uidvalidity: 1,
        message_id: Some(format!("<m{uid}@x>")),
        subject: Some(subject.to_string()),
        sender: Some(sender.to_string()),
        sender_addr: Some(format!("s{uid}@x.com")),
        recipient: Some("me@qq.com".to_string()),
        date: Some(date),
        seen,
    }
}

#[test]
fn account_lifecycle_with_mailbox_cascade() {
    let (s, p) = temp_store("acct");

    let id = s.insert_account("a@qq.com", "工作", "#1677ff", "qq").expect("插入账号");
    let found = s.find_account_by_email("a@qq.com").expect("查询失败").expect("应找到账号");
    assert_eq!(found.id, id);
    assert_eq!(found.display_name, "工作");
    assert_eq!(found.provider, "qq", "账号应记录所属服务商类型");
    assert!(found.enabled);

    let inbox = s.upsert_mailbox(id, "INBOX", "INBOX", "inbox", "/").unwrap();
    // 重复 upsert 同一 raw 名:更新而非新增
    assert_eq!(s.upsert_mailbox(id, "INBOX", "INBOX", "inbox", "/").unwrap(), inbox);
    s.upsert_mailbox(id, "Sent Messages", "已发送", "sent", "/").unwrap();
    let boxes = s.list_mailboxes(id).unwrap();
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].role, "inbox");

    // 账号下的邮件与邮箱随账号级联删除
    s.upsert_message(&msg(inbox, 5, "hello", "张三", 100, false)).unwrap();
    s.delete_account(id).unwrap();
    assert!(s.find_account_by_email("a@qq.com").unwrap().is_none());
    assert!(s.list_mailboxes(id).unwrap().is_empty(), "邮箱应随账号级联删除");
    assert!(s.list_messages(None, None, false, "", 0, 50).unwrap().is_empty());

    cleanup(&p);
}

#[test]
fn message_upsert_dedup_by_mailbox_uidvalidity_uid() {
    let (s, p) = temp_store("dedup");

    let (_, inbox_a) = setup_account(&s, "a@qq.com");
    let (_, inbox_b) = setup_account(&s, "b@qq.com");

    let id1 = s.upsert_message(&msg(inbox_a, 7, "旧主题", "发件人", 10, false)).unwrap();
    // 同一 (mailbox, uidvalidity, uid) 再写入:更新而非新增
    let id2 = s.upsert_message(&msg(inbox_a, 7, "新主题", "发件人", 10, true)).unwrap();
    assert_eq!(id1, id2, "同一封邮件应只保留一行");
    let m = s.get_message(id1).unwrap();
    assert_eq!(m.subject.as_deref(), Some("新主题"));
    assert!(m.seen, "seen 状态应随服务器旗标更新");

    // 同 UID 不同邮箱:两行
    s.upsert_message(&msg(inbox_b, 7, "B 的邮件", "发件人", 11, false)).unwrap();
    assert_eq!(s.list_messages(None, None, false, "", 0, 50).unwrap().len(), 2);

    cleanup(&p);
}

#[test]
fn list_messages_views_and_filters() {
    let (s, p) = temp_store("list");

    let (a, inbox_a) = setup_account(&s, "a@qq.com");
    let (_, inbox_b) = setup_account(&s, "b@qq.com");
    let sent_a = s.upsert_mailbox(a, "Sent Messages", "已发送", "sent", "/").unwrap();

    s.upsert_message(&msg(inbox_a, 1, "会议纪要", "张三", 100, false)).unwrap();
    s.upsert_message(&msg(inbox_a, 2, "Weekly Report", "bob", 200, true)).unwrap();
    s.upsert_message(&msg(inbox_b, 1, "会议通知", "李四", 150, false)).unwrap();
    s.upsert_message(&msg(sent_a, 9, "我发出的", "我", 300, true)).unwrap();

    // 默认视图(全部邮件)= 所有 inbox role
    let all = s.list_messages(None, None, false, "", 0, 50).unwrap();
    let subjects: Vec<_> = all.iter().filter_map(|m| m.subject.clone()).collect();
    assert_eq!(subjects, vec!["Weekly Report", "会议通知", "会议纪要"], "默认视图只含收件箱,按时间倒序");

    // 指定邮箱视图(已发送)
    let sent = s.list_messages(None, Some(sent_a), false, "", 0, 50).unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].subject.as_deref(), Some("我发出的"));

    // 账号视图 = 该账号收件箱
    assert_eq!(s.list_messages(Some(a), None, false, "", 0, 50).unwrap().len(), 2);

    // 未读过滤
    let unread = s.list_messages(None, None, true, "", 0, 50).unwrap();
    assert_eq!(unread.len(), 2);
    assert!(unread.iter().all(|m| !m.seen));

    // 中文搜索 + LIKE 转义
    assert_eq!(s.list_messages(None, None, false, "会议", 0, 50).unwrap().len(), 2);
    assert_eq!(s.list_messages(None, None, false, "张三", 0, 50).unwrap().len(), 1);
    assert_eq!(s.list_messages(None, None, false, "%", 0, 50).unwrap().len(), 0);

    // 分页:offset 跳过最新一条
    let page = s.list_messages(None, None, false, "", 1, 2).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].subject.as_deref(), Some("会议通知"));

    cleanup(&p);
}

#[test]
fn list_unseen_for_scope_respects_view() {
    let (s, p) = temp_store("unseen");

    let (a, inbox_a) = setup_account(&s, "a@qq.com");
    let (_, inbox_b) = setup_account(&s, "b@qq.com");
    let sent_a = s.upsert_mailbox(a, "Sent Messages", "已发送", "sent", "/").unwrap();

    let m1 = s.upsert_message(&msg(inbox_a, 1, "一", "s", 100, false)).unwrap();
    let _m2 = s.upsert_message(&msg(inbox_a, 2, "二", "s", 200, true)).unwrap();
    let _m3 = s.upsert_message(&msg(inbox_b, 1, "三", "s", 150, false)).unwrap();
    let m4 = s.upsert_message(&msg(sent_a, 9, "发出", "我", 300, false)).unwrap();

    // 全部邮件视图:只含 inbox role 的未读
    let all = s.list_unseen_for_scope(None).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|r| r.uid == 1), "只返回未读邮件");
    assert!(all.iter().all(|r| r.mailbox_id != sent_a));

    // 指定邮箱视图:含该邮箱全部未读(含已发送)
    let sent = s.list_unseen_for_scope(Some(sent_a)).unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].message_id, m4);
    assert_eq!(sent[0].account_id, a);

    // 已读后不再出现
    s.set_seen_local(m1, true).unwrap();
    assert_eq!(s.list_unseen_for_scope(Some(inbox_a)).unwrap().len(), 0);

    cleanup(&p);
}

#[test]
fn body_cache_attachments_and_mailbox_unread_counts() {
    let (s, p) = temp_store("cache");

    let (a, inbox_a) = setup_account(&s, "a@qq.com");
    let sent_a = s.upsert_mailbox(a, "Sent Messages", "已发送", "sent", "/").unwrap();
    let m1 = s.upsert_message(&msg(inbox_a, 1, "一", "s", 100, false)).unwrap();
    let m2 = s.upsert_message(&msg(inbox_a, 2, "二", "s", 200, false)).unwrap();
    s.upsert_message(&msg(inbox_a, 3, "三", "s", 300, true)).unwrap();
    s.upsert_message(&msg(sent_a, 9, "发出", "我", 400, true)).unwrap();

    // 正文缓存:覆盖写
    s.cache_body(m1, Some("纯文本"), Some("<p>html</p>")).unwrap();
    s.cache_body(m1, Some("纯文本2"), None).unwrap();
    let cached = s.get_cached_body(m1).unwrap().expect("应命中缓存");
    assert_eq!(cached.0.as_deref(), Some("纯文本2"));
    assert!(cached.1.is_none());
    assert!(s.get_cached_body(m2).unwrap().is_none());

    // 附件:整体替换
    s.replace_attachments(m1, &[]).unwrap();
    s.replace_attachments(
        m1,
        &[("a.pdf".into(), "application/pdf".into(), 10), ("b.zip".into(), "application/zip".into(), 20)],
    )
    .unwrap();
    let atts = s.list_attachments(m1).unwrap();
    assert_eq!(atts.len(), 2);
    assert_eq!(atts[0].name, "a.pdf");

    // 按邮箱的未读统计:只有存在未读的邮箱才出现
    let counts = s.mailbox_unread_counts().unwrap();
    assert_eq!(counts.len(), 1);
    let inbox_count = &counts[0];
    assert_eq!(inbox_count.mailbox_id, inbox_a);
    assert_eq!(inbox_count.unread, 2);
    assert_eq!(inbox_count.role, "inbox");

    // 同步游标存于邮箱行
    s.update_mailbox_sync(inbox_a, Some(1), Some(99)).unwrap();
    assert_eq!(s.mailbox_cursor(inbox_a).unwrap(), (Some(1), Some(99)));

    cleanup(&p);
}

#[test]
fn foreign_keys_cascade_survives_reopen() {
    // 回归:PRAGMA foreign_keys 是每连接设置,重开库后必须仍然生效,
    // 否则删除账号不会级联清理邮件与邮箱(垃圾数据残留)。
    let path = std::env::temp_dir().join(format!("maildeck-test-fk-{}.db", std::process::id()));
    cleanup(&path);

    let inbox;
    {
        let s = Store::open(&path).expect("首次打开");
        let a = s.insert_account("a@qq.com", "A", "#111111", "qq").unwrap();
        inbox = s.upsert_mailbox(a, "INBOX", "INBOX", "inbox", "/").unwrap();
        s.upsert_message(&msg(inbox, 1, "一", "s", 100, false)).unwrap();
        s.upsert_message(&msg(inbox, 2, "二", "s", 200, false)).unwrap();
    } // drop:连接关闭

    // 重开(模拟应用重启),此时连接上 FK 必须仍是 ON
    let s = Store::open(&path).expect("重开");
    let accounts = s.list_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    s.delete_account(accounts[0].id).unwrap();

    assert!(s.list_accounts().unwrap().is_empty());
    assert!(s.list_mailboxes(accounts[0].id).unwrap().is_empty(), "邮箱应随账号级联删除");
    assert!(
        s.list_messages(None, None, false, "", 0, 50).unwrap().is_empty(),
        "邮件应随账号级联删除,不能残留孤儿行"
    );

    cleanup(&path);
}

#[test]
fn db_file_is_encrypted_at_rest() {
    // 落盘的数据库文件不得泄露内容:不能有 SQLite 明文魔数,也不能直接搜到正文
    let path = std::env::temp_dir().join(format!("maildeck-test-enc-{}.db", std::process::id()));
    cleanup(&path);

    let marker_body = "机密正文标记-SECRET-BODY-9527";
    {
        let s = Store::open(&path).unwrap();
        let a = s.insert_account("a@qq.com", "A", "#111111", "qq").unwrap();
        let inbox = s.upsert_mailbox(a, "INBOX", "INBOX", "inbox", "/").unwrap();
        s.upsert_message(&msg(inbox, 1, marker_body, "s", 100, false)).unwrap();
        s.cache_body(s.list_messages(None, None, false, "", 0, 1).unwrap()[0].id,
                      Some(marker_body), None).unwrap();
    }

    let bytes = std::fs::read(&path).unwrap();
    assert!(
        !bytes.starts_with(b"SQLite format 3\0"),
        "数据库文件不应是明文 SQLite"
    );
    let hay = bytes.windows(marker_body.len())
        .any(|w| w == marker_body.as_bytes());
    assert!(!hay, "正文明文不应直接出现在数据库文件中");
    let hay_subj = bytes.windows(6).any(|w| w == b"SECRET");
    assert!(!hay_subj, "主题明文不应直接出现在数据库文件中");

    cleanup(&path);
}

#[test]
fn legacy_plaintext_db_auto_migrates() {
    // v0.1.0 的明文库:用 open_plain 构造,新版本 open 时应自动迁入加密库且数据完好
    let path = std::env::temp_dir().join(format!("maildeck-test-legacy-{}.db", std::process::id()));
    cleanup(&path);

    {
        let s = Store::open_plain(&path).expect("构造旧明文库");
        let a = s.insert_account("old@qq.com", "旧", "#000000", "qq").unwrap();
        let inbox = s.upsert_mailbox(a, "INBOX", "INBOX", "inbox", "/").unwrap();
        s.upsert_message(&msg(inbox, 7, "迁移邮件", "s", 100, false)).unwrap();
    }

    let s = Store::open(&path).expect("加密方式打开应触发迁移");
    let accounts = s.list_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    let rows = s.list_messages(Some(accounts[0].id), None, false, "", 0, 50).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].subject.as_deref(), Some("迁移邮件"));

    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3\0"), "迁移后文件应为加密格式");
    assert!(
        !bytes.windows(12).any(|w| w == b"SQLite form"),
        "迁移后不应残留明文头"
    );

    cleanup(&path);
}

#[test]
fn v1_database_is_migrated_to_mailbox_model() {
    let path = std::env::temp_dir().join(format!("maildeck-test-mig-{}.db", std::process::id()));
    cleanup(&path);

    // 用 v1 结构建库并写入一条邮件
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
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
            CREATE TABLE messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
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
                UNIQUE(account_id, uidvalidity, uid)
            );
            INSERT INTO accounts (email, display_name, color, created_at) VALUES ('old@qq.com', '旧库', '#000000', 1);
            INSERT INTO messages (account_id, uid, uidvalidity, subject, date, seen) VALUES (1, 7, 1, '迁移邮件', 100, 0);
            "#,
        )
        .unwrap();
    }

    // 新代码打开旧库:应自动迁移,数据不丢
    let s = Store::open(&path).unwrap();
    let account = s.find_account_by_email("old@qq.com").unwrap().expect("账号应保留");
    let mailboxes = s.list_mailboxes(account.id).unwrap();
    assert_eq!(mailboxes.len(), 1, "应自动建立收件箱邮箱");
    assert_eq!(mailboxes[0].role, "inbox");
    let migrated = s.list_messages(Some(account.id), None, false, "", 0, 50).unwrap();
    assert_eq!(migrated.len(), 1, "旧邮件应迁移进收件箱邮箱");
    assert_eq!(migrated[0].subject.as_deref(), Some("迁移邮件"));
    assert_eq!(migrated[0].uid, 7);

    cleanup(&path);
}
