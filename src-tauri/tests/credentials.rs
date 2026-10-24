//! 凭证存储测试:keyring(真实钥匙串,手动)与本地文件模式(自动)。
//! 运行:
//!   cargo test --test credentials            # 只跑文件模式
//!   cargo test --test credentials -- --ignored  # 额外跑 keyring 真实钥匙串

use maildeck_lib::app_settings::CredentialMode;
use maildeck_lib::credentials;
use std::path::PathBuf;

fn file_store() -> (CredentialMode, PathBuf) {
    let dir = std::env::temp_dir().join(format!("maildeck-creds-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    (CredentialMode::File, dir)
}

#[test]
fn file_mode_roundtrip_and_delete() {
    let (mode, path) = file_store();

    credentials::save_auth_code(&mode, &path, "a@163.com", "code-abc").unwrap();
    assert_eq!(
        credentials::load_auth_code(&mode, &path, "a@163.com").unwrap(),
        "code-abc"
    );

    // 同一邮箱覆盖写
    credentials::save_auth_code(&mode, &path, "a@163.com", "code-xyz").unwrap();
    assert_eq!(
        credentials::load_auth_code(&mode, &path, "a@163.com").unwrap(),
        "code-xyz"
    );

    // 第二个账号互不影响
    credentials::save_auth_code(&mode, &path, "b@qq.com", "code-b").unwrap();
    assert_eq!(
        credentials::load_auth_code(&mode, &path, "b@qq.com").unwrap(),
        "code-b"
    );

    // 删除单个账号
    credentials::delete_auth_code(&mode, &path, "a@163.com").unwrap();
    assert!(credentials::load_auth_code(&mode, &path, "a@163.com").is_err());
    assert!(credentials::load_auth_code(&mode, &path, "b@qq.com").is_ok());

    // 删除不存在的条目是幂等的
    credentials::delete_auth_code(&mode, &path, "a@163.com").unwrap();
}

#[test]
#[ignore = "写入真实系统钥匙串,手动执行"]
fn keyring_mode_roundtrip_and_delete() {
    let mode = CredentialMode::Keyring;
    let path = PathBuf::new(); // keyring 模式不使用文件

    credentials::save_auth_code(&mode, &path, "maildeck-test@example.com", "test-code").unwrap();
    assert_eq!(
        credentials::load_auth_code(&mode, &path, "maildeck-test@example.com").unwrap(),
        "test-code"
    );
    credentials::delete_auth_code(&mode, &path, "maildeck-test@example.com").unwrap();
    assert!(credentials::load_auth_code(&mode, &path, "maildeck-test@example.com").is_err());
    credentials::delete_auth_code(&mode, &path, "maildeck-test@example.com").unwrap();
}
