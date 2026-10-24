//! QQ 邮箱真实账号冒烟测试(M1 验证的一部分)。不进默认测试集。
//! 运行前设置环境变量:
//!   export MAILDECK_QQ_EMAIL=xxx@qq.com
//!   export MAILDECK_QQ_AUTH_CODE=授权码
//!   export MAILDECK_QQ_UID=收件箱中某封邮件的UID   # 仅 fetch 用例需要
//! 运行:`cargo test --test provider_qq -- --ignored`

use maildeck_lib::provider::qq;

fn env_or_panic(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("缺少环境变量 {key},冒烟测试需要真实账号"))
}

#[tokio::test]
#[ignore = "需要真实 QQ 账号,手动执行"]
async fn connect_select_and_logout() {
    let email = env_or_panic("MAILDECK_QQ_EMAIL");
    let code = env_or_panic("MAILDECK_QQ_AUTH_CODE");

    let mut sess = qq::connect(&email, &code).await.expect("连接与登录失败");
    let mailbox = sess.select(qq::INBOX).await.expect("选择收件箱失败");
    assert!(mailbox.exists > 0, "收件箱应为非空测试账号");
    println!(
        "INBOX exists={} uidvalidity={:?} uidnext={:?}",
        mailbox.exists, mailbox.uid_validity, mailbox.uid_next
    );
    qq::logout(sess).await;
}

#[tokio::test]
#[ignore = "需要真实 QQ 账号与指定 UID,手动执行"]
async fn fetch_raw_message_by_uid() {
    let email = env_or_panic("MAILDECK_QQ_EMAIL");
    let code = env_or_panic("MAILDECK_QQ_AUTH_CODE");
    let uid: u32 = env_or_panic("MAILDECK_QQ_UID").parse().expect("UID 应为数字");

    let mut sess = qq::connect(&email, &code).await.expect("连接与登录失败");
    let raw = qq::fetch_raw_message(&mut sess, qq::INBOX, uid).await.expect("拉取原始邮件失败");
    assert!(raw.len() > 10, "原始邮件不应为空");

    // 拉取后邮件在本地视角应仍为未读(使用 BODY.PEEK 而非 BODY)
    qq::logout(sess).await;
}
