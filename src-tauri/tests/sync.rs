//! 同步纯逻辑测试:UID 窗口选取、搜索命令构造、文件夹分类、IMAP UTF-7 解码。
//! 运行:`cargo test --test sync`

use maildeck_lib::provider::classify_mailbox;
use maildeck_lib::provider::MailboxRole;
use maildeck_lib::sync::{decode_utf7, pick_uid_window, uid_search_command, uid_set_chunks};

#[test]
fn first_sync_takes_latest_capped() {
    let uids: Vec<u32> = (1..=1000u32).collect();
    let picked = pick_uid_window(&uids, 0, 200);
    assert_eq!(picked.len(), 200, "首次同步最多取 200 封");
    assert_eq!(*picked.first().unwrap(), 801, "应取最新的一端");
    assert_eq!(*picked.last().unwrap(), 1000);
}

#[test]
fn first_sync_small_mailbox_takes_all() {
    let uids = vec![1u32, 2, 3];
    assert_eq!(pick_uid_window(&uids, 0, 200), vec![1, 2, 3]);
}

#[test]
fn incremental_returns_only_newer_uids() {
    let uids: Vec<u32> = (1..=100u32).collect();
    let picked = pick_uid_window(&uids, 90, 200);
    assert_eq!(picked, vec![91, 92, 93, 94, 95, 96, 97, 98, 99, 100]);
}

#[test]
fn incremental_empty_when_nothing_new() {
    let uids: Vec<u32> = (1..=50u32).collect();
    assert!(pick_uid_window(&uids, 50, 200).is_empty());
    assert!(pick_uid_window(&[], 0, 200).is_empty());
}

#[test]
fn incremental_backlog_is_capped_to_latest() {
    // 长时间未同步导致新邮件堆积超过上限时,只取最新的 200 封
    let uids: Vec<u32> = (1..=500u32).collect();
    let picked = pick_uid_window(&uids, 100, 200);
    assert_eq!(picked.len(), 200);
    assert_eq!(*picked.first().unwrap(), 301);
    assert_eq!(*picked.last().unwrap(), 500);
}

#[test]
fn uid_search_command_by_cursor() {
    assert_eq!(uid_search_command(0), "ALL");
    assert_eq!(uid_search_command(1), "UID 2:*");
    assert_eq!(uid_search_command(90), "UID 91:*");
}

#[test]
fn uid_set_chunks_splits_and_joins() {
    let uids: Vec<u32> = (1..=7).collect();
    assert_eq!(uid_set_chunks(&uids, 3), vec!["1,2,3", "4,5,6", "7"]);
    assert_eq!(uid_set_chunks(&uids, 7), vec!["1,2,3,4,5,6,7"]);
    assert_eq!(uid_set_chunks(&uids, 100), vec!["1,2,3,4,5,6,7"]);
}

#[test]
fn uid_set_chunks_empty_is_empty() {
    assert!(uid_set_chunks(&[], 10).is_empty());
}

#[test]
fn picked_window_preserves_ascending_order() {
    // IMAP UID SEARCH 返回无序集合,窗口函数应输出升序
    let uids = vec![30u32, 5, 12, 25, 40, 8];
    let picked = pick_uid_window(&uids, 10, 10);
    assert_eq!(picked, vec![12, 25, 30, 40], "输出应升序且过滤掉旧 UID");
}

#[test]
fn classify_by_special_use_flags_first() {
    // SPECIAL-USE 旗标优先于名字启发式
    assert_eq!(
        classify_mailbox("whatever", &["\\Sent".to_string()]),
        MailboxRole::Sent
    );
    assert_eq!(
        classify_mailbox("whatever", &["\\Drafts".to_string()]),
        MailboxRole::Drafts
    );
    assert_eq!(
        classify_mailbox("whatever", &["\\Trash".to_string()]),
        MailboxRole::Trash
    );
    assert_eq!(
        classify_mailbox("whatever", &["\\Junk".to_string()]),
        MailboxRole::Spam
    );
}

#[test]
fn classify_by_name_heuristics() {
    assert_eq!(classify_mailbox("INBOX", &[]), MailboxRole::Inbox);
    assert_eq!(classify_mailbox("Sent Messages", &[]), MailboxRole::Sent);
    assert_eq!(classify_mailbox("已发送", &[]), MailboxRole::Sent);
    assert_eq!(classify_mailbox("草稿箱", &[]), MailboxRole::Drafts);
    assert_eq!(classify_mailbox("Drafts", &[]), MailboxRole::Drafts);
    assert_eq!(classify_mailbox("Deleted Messages", &[]), MailboxRole::Trash);
    assert_eq!(classify_mailbox("已删除", &[]), MailboxRole::Trash);
    assert_eq!(classify_mailbox("Junk", &[]), MailboxRole::Spam);
    assert_eq!(classify_mailbox("垃圾箱", &[]), MailboxRole::Spam);
    assert_eq!(classify_mailbox("垃圾邮件", &[]), MailboxRole::Spam);
    assert_eq!(classify_mailbox("Notes", &[]), MailboxRole::Other);
    assert_eq!(classify_mailbox("Archive", &[]), MailboxRole::Other);
    // 大小写不敏感
    assert_eq!(classify_mailbox("sent", &[]), MailboxRole::Sent);
}

#[test]
fn decode_imap_utf7_folder_names() {
    // 163 等服务商的中文文件夹名是 modified UTF-7
    assert_eq!(decode_utf7("&XfJT0ZAB-"), "已发送");
    assert_eq!(decode_utf7("INBOX"), "INBOX");
    // "&-" 转义表示字面 '&'
    assert_eq!(decode_utf7("a&-b"), "a&b");
    // 连续段与普通文本混合
    assert_eq!(decode_utf7("abc&XfJT0ZAB-123"), "abc已发送123");
    // 已是 UTF-8 的名字原样透传
    assert_eq!(decode_utf7("已删除"), "已删除");
    // 非法段不 panic,原样保留
    assert_eq!(decode_utf7("&???-"), "&???-");
}

#[test]
fn netease_163_folder_pipeline() {
    // 163 真实场景:UTF-7 文件夹名 → 解码 → 分类,一条流水线
    let raw = "&XfJT0ZAB-";
    assert_eq!(
        classify_mailbox(raw, &[]),
        MailboxRole::Other,
        "用 UTF-7 原始名分类必然落到 Other——同步管线必须先解码"
    );
    let decoded = decode_utf7(raw);
    assert_eq!(decoded, "已发送");
    assert_eq!(classify_mailbox(&decoded, &[]), MailboxRole::Sent);
    assert_eq!(classify_mailbox("INBOX", &[]), MailboxRole::Inbox);
}
