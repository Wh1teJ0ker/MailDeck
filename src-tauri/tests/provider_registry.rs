//! 服务商类型注册表测试:按类型添加账号的依据。
//! 运行:`cargo test --test provider_registry`

use maildeck_lib::provider;

#[test]
fn registry_lists_known_types() {
    let reg = provider::registry();
    assert!(reg.len() >= 3, "至少登记 QQ / Outlook / 163 三类");
    assert!(reg.iter().all(|p| !p.name.is_empty() && !p.auth_mode.is_empty()));
    assert!(reg.iter().any(|p| p.id == "qq"), "必须包含 QQ 邮箱");
    assert!(reg.iter().any(|p| p.id == "163"), "必须包含 163 邮箱");
}

#[test]
fn qq_and_163_are_available_in_v011() {
    let available: Vec<&str> = provider::registry()
        .iter()
        .filter(|p| p.available)
        .map(|p| p.id)
        .collect();
    assert!(
        available.contains(&"qq") && available.contains(&"163"),
        "v0.1.1 应支持 QQ 与 163:实际 {:?}",
        available
    );
    assert!(!available.contains(&"outlook"), "Outlook 仍为预留");
}

#[test]
fn meta_lookup_by_type() {
    let qq = provider::meta("qq").expect("qq 应存在");
    assert!(qq.available);
    assert_eq!(qq.auth_mode, "授权码");

    let n163 = provider::meta("163").expect("163 应存在");
    assert!(n163.available);
    assert_eq!(n163.auth_mode, "授权码");
    assert_eq!(n163.endpoint.host, "imap.163.com");
    assert_eq!(n163.endpoint.port, 993);
    assert!(n163.endpoint.requires_id, "163 登录后必须先发 IMAP ID,否则拒绝操作");

    let outlook = provider::meta("outlook").expect("outlook 应作为预留存在");
    assert!(!outlook.available, "Outlook 预留不可添加");

    assert!(provider::meta("不存在").is_none(), "未登记类型应返回 None");
    assert!(provider::meta("").is_none());
}

#[test]
fn every_entry_has_user_hint() {
    assert!(provider::registry().iter().all(|p| !p.hint.is_empty()));
}
