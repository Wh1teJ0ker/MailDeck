//! 服务商类型注册表测试:按类型添加账号的依据。
//! 运行:`cargo test --test provider_registry`

use maildeck_lib::provider;

#[test]
fn registry_lists_known_types() {
    let reg = provider::registry();
    assert!(reg.len() >= 3, "至少登记 QQ / Outlook / 163 三类");
    assert!(reg.iter().all(|p| !p.name.is_empty() && !p.auth_mode.is_empty()));
    assert!(reg.iter().any(|p| p.id == "qq"), "必须包含 QQ 邮箱");
}

#[test]
fn qq_is_the_only_available_type_in_v010() {
    let available: Vec<&str> = provider::registry()
        .iter()
        .filter(|p| p.available)
        .map(|p| p.id)
        .collect();
    assert_eq!(available, vec!["qq"], "v0.1.0 只有 QQ 可真正添加");
}

#[test]
fn meta_lookup_by_type() {
    let qq = provider::meta("qq").expect("qq 应存在");
    assert!(qq.available);
    assert_eq!(qq.auth_mode, "授权码");

    let outlook = provider::meta("outlook").expect("outlook 应作为预留存在");
    assert!(!outlook.available, "Outlook 预留不可添加");

    assert!(provider::meta("gmail").is_none(), "未登记类型应返回 None");
    assert!(provider::meta("").is_none());
}

#[test]
fn every_entry_has_user_hint() {
    assert!(provider::registry().iter().all(|p| !p.hint.is_empty()));
}
