//! 凭证库集成测试。会真实读写 macOS Keychain,默认忽略;
//! 运行:`cargo test --test credentials -- --ignored`
use maildeck_lib::credentials;

const TEST_EMAIL: &str = "maildeck-test@example.com";

#[test]
#[ignore = "写入真实系统凭证库,手动执行"]
fn auth_code_roundtrip_and_delete() {
    credentials::save_auth_code(TEST_EMAIL, "test-code-123").expect("保存授权码失败");
    let loaded = credentials::load_auth_code(TEST_EMAIL).expect("读取授权码失败");
    assert_eq!(loaded, "test-code-123");

    credentials::delete_auth_code(TEST_EMAIL).expect("删除授权码失败");
    let gone = credentials::load_auth_code(TEST_EMAIL);
    assert!(gone.is_err(), "删除后读取应失败");

    // 幂等:删除不存在的条目不应报错
    credentials::delete_auth_code(TEST_EMAIL).expect("重复删除应幂等");
}
