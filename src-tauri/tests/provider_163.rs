//! 163 邮箱真实账号冒烟测试。不进默认测试集。
//! 运行前设置环境变量:
//!   export MAILDECK_163_EMAIL=xxx@163.com
//!   export MAILDECK_163_AUTH_CODE=客户端授权密码
//! 运行:`cargo test --test provider_163 -- --ignored`

use maildeck_lib::provider;

fn env_or_panic(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("缺少环境变量 {key},冒烟测试需要真实账号"))
}

#[tokio::test]
#[ignore = "需要真实 163 账号,手动执行"]
async fn connect_select_and_list_folders() {
    let email = env_or_panic("MAILDECK_163_EMAIL");
    let code = env_or_panic("MAILDECK_163_AUTH_CODE");

    // connect_for 内部应完成:IMAPS 连接 → 授权码登录 → 163 要求的 IMAP ID 上报
    let mut sess = provider::connect_for("163", &email, &code)
        .await
        .expect("163 连接/登录/ID 上报失败");
    let mailbox = sess.select("INBOX").await.expect("选择收件箱失败");
    println!("INBOX exists={}", mailbox.exists);

    // 列文件夹并确认中文文件夹(UTF-7)可解码、可分类
    use futures::StreamExt;
    let mut names = sess.list(Some(""), Some("*")).await.expect("LIST 失败");
    let mut found_sent = false;
    while let Some(item) = names.next().await {
        let name = item.expect("LIST 条目失败");
        let decoded = maildeck_lib::sync::decode_utf7(name.name());
        println!("folder: raw={} decoded={}", name.name(), decoded);
        if decoded.contains("已发送") {
            found_sent = true;
        }
    }
    drop(names);
    assert!(found_sent, "163 应存在「已发送」文件夹(UTF-7 编码)");
    provider::qq::logout(sess).await;
}
