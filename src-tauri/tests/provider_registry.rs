//! 服务商类型注册表测试:按类型添加账号的依据。
//! 运行:`cargo test --test provider_registry`

use maildeck_lib::provider;

#[test]
fn registry_lists_all_supported_types() {
    let reg = provider::registry();
    // v0.1.2:QQ / 163 / Gmail / 126 / 腾讯企业邮 / iCloud / 搜狐 / 新浪 / 阿里企业邮
    assert!(reg.len() >= 9, "应至少登记 9 家服务商,实际 {}", reg.len());
    for id in ["qq", "163", "gmail", "126", "exmail", "icloud", "sohu", "sina", "aliyun"] {
        assert!(reg.iter().any(|p| p.id == id), "缺少服务商 {id}");
    }
}

#[test]
fn all_types_are_available() {
    // v0.1.2 起除 Outlook 预留外全部可用
    for id in ["qq", "163", "gmail", "126", "exmail", "icloud", "sohu", "sina", "aliyun"] {
        let m = provider::meta(id).unwrap();
        assert!(m.available, "{id} 应为可添加状态");
    }
    assert!(!provider::meta("outlook").unwrap().available, "Outlook 仍为预留");
}

#[test]
fn endpoints_and_auth_modes() {
    let qq = provider::meta("qq").unwrap();
    assert_eq!(qq.endpoint.host, "imap.qq.com");
    assert_eq!(qq.auth_mode, "授权码");

    let n163 = provider::meta("163").unwrap();
    assert_eq!(n163.endpoint.host, "imap.163.com");
    assert!(n163.endpoint.requires_id, "163 必须上报 IMAP ID");

    let n126 = provider::meta("126").unwrap();
    assert_eq!(n126.endpoint.host, "imap.126.com");
    assert!(n126.endpoint.requires_id, "126 同为网易系,需要 ID");

    assert_eq!(provider::meta("exmail").unwrap().endpoint.host, "imap.exmail.qq.com");
    assert_eq!(provider::meta("icloud").unwrap().endpoint.host, "imap.mail.me.com");
    assert_eq!(provider::meta("icloud").unwrap().auth_mode, "应用专用密码");
    assert_eq!(provider::meta("sohu").unwrap().endpoint.host, "imap.sohu.com");
    assert_eq!(provider::meta("sina").unwrap().endpoint.host, "imap.sina.com");
    assert_eq!(provider::meta("aliyun").unwrap().endpoint.host, "imap.qiye.aliyun.com");

    let g = provider::meta("gmail").unwrap();
    assert_eq!(g.endpoint.host, "imap.gmail.com");
    assert_eq!(g.auth_mode, "OAuth2");
}

#[test]
fn suffix_fields_present() {
    // 固定后缀可自动补全;自定义域企业邮(企业邮/阿里)suffix 为空
    assert_eq!(provider::meta("qq").unwrap().suffix, "qq.com");
    assert_eq!(provider::meta("163").unwrap().suffix, "163.com");
    assert_eq!(provider::meta("gmail").unwrap().suffix, "gmail.com");
    assert_eq!(provider::meta("126").unwrap().suffix, "126.com");
    assert_eq!(provider::meta("icloud").unwrap().suffix, "icloud.com");
    assert_eq!(provider::meta("sohu").unwrap().suffix, "sohu.com");
    assert_eq!(provider::meta("sina").unwrap().suffix, "sina.com");
    assert_eq!(provider::meta("exmail").unwrap().suffix, "", "企业邮箱地址为用户自有域名");
    assert_eq!(provider::meta("aliyun").unwrap().suffix, "", "阿里企业邮地址为用户自有域名");
}

#[test]
fn every_entry_has_user_hint() {
    assert!(provider::registry().iter().all(|p| !p.hint.is_empty()));
}
