//! OAuth2(XOAUTH2)与 Gmail 适配的单元测试。
//! 运行:`cargo test --test oauth2`

use maildeck_lib::provider::{gmail, meta, oauth2};

#[test]
fn xoauth2_string_format() {
    // RFC 4616:user=邮箱 \x01 auth=Bearer token \x01 \x01,整体 base64
    let s = oauth2::xoauth2_string("user@gmail.com", "token-abc");
    assert_eq!(s, "dXNlcj11c2VyQGdtYWlsLmNvbQFhdXRoPUJlYXJlciB0b2tlbi1hYmMBAQ==");
    // 解码回验控制字符结构
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(&s).unwrap();
    assert_eq!(
        String::from_utf8(raw).unwrap(),
        "user=user@gmail.com\u{1}auth=Bearer token-abc\u{1}\u{1}"
    );
}

#[test]
fn meta_and_endpoint() {
    let g = meta("gmail").expect("gmail 应存在");
    assert!(g.available);
    assert_eq!(g.auth_mode, "OAuth2");
    assert_eq!(g.endpoint.host, "imap.gmail.com");
    assert_eq!(g.endpoint.port, 993);
    assert!(g.endpoint.requires_id == false);
}

#[test]
fn scopes_are_readonly_imap() {
    assert!(gmail::SCOPES.contains(&"https://mail.google.com/"));
    assert!(gmail::SCOPES.contains(&"https://www.googleapis.com/auth/userinfo.email"));
}

#[test]
fn authorize_url_contains_required_params() {
    let url = gmail::authorize_url("client-123", "http://127.0.0.1:0", "challenge-xyz", "state-abc");
    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
    assert!(url.contains("client_id=client-123"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("code_challenge=challenge-xyz"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("state=state-abc"));
    assert!(url.contains("redirect_uri="));
    assert!(url.contains("scope="));
    assert!(url.contains("access_type=offline"), "必须请求 refresh_token");
    assert!(url.contains("prompt=consent"));
}
