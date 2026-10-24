//! IMAP 登录失败的错误归因测试。
//! 运行:`cargo test --test provider_errors`

use maildeck_lib::provider::explain_login_failure;

#[test]
fn authentication_failed_maps_to_checklist() {
    let raw = "no response: code: None, info: Some(\"[AUTHENTICATIONFAILED] Login fail. Account is abnormal, \
               service is not open, password is incorrect, login frequency limited, or system is busy.\")";
    let msg = explain_login_failure(raw);
    assert!(msg.contains("IMAP/SMTP 服务已开启"), "应包含服务开关检查:{msg}");
    assert!(msg.contains("授权码"), "应包含授权码检查:{msg}");
    assert!(msg.contains("15~30 分钟"), "应包含频率限制提示:{msg}");
    assert!(msg.contains(raw), "应保留服务器原始信息以便进一步排查:{msg}");
}

#[test]
fn rate_limit_hint_for_frequency_case() {
    let msg = explain_login_failure("EXAMINE Unsafe Login. Please contact kefu@188.com");
    assert!(msg.contains("163"), "Unsafe Login 应提示 163 特殊要求:{msg}");
}

#[test]
fn other_errors_pass_through_untouched() {
    let raw = "连接 imap.qq.com:993 失败,请检查网络";
    assert_eq!(explain_login_failure(raw), raw);
}
