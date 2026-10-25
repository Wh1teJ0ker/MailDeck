//! 凭证存储测试:keyring(真实钥匙串,手动)与本地口令加密文件模式(自动)。
//! 运行:
//!   cargo test --test credentials               # 只跑文件模式
//!   cargo test --test credentials -- --ignored  # 额外跑 keyring 真实钥匙串

use serial_test::serial;

use maildeck_lib::app_settings::CredentialMode;
use maildeck_lib::credentials;
use std::path::PathBuf;

const TEST_PASS: &str = "测试口令-abc123";

fn file_store() -> (CredentialMode, PathBuf) {
    let dir = std::env::temp_dir().join(format!("maildeck-creds-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 文件模式需要口令:设置运行期口令(测试进程内)
    credentials::set_runtime_passphrase(Some(TEST_PASS.to_string()));
    (CredentialMode::File, dir)
}

#[test]
#[serial]
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

    // 落盘内容必须加密:文件里不能直接搜到授权码或口令
    let bytes = std::fs::read(path.join("credentials.json")).unwrap();
    for needle in ["code-abc".as_bytes(), "code-xyz".as_bytes(), "code-b".as_bytes(), TEST_PASS.as_bytes()] {
        assert!(
            !bytes.windows(needle.len().max(1)).any(|w| w == needle),
            "凭证文件不应包含明文授权码或口令"
        );
    }
}

#[test]
#[serial]
fn file_mode_requires_correct_passphrase() {
    let (mode, path) = file_store();
    credentials::save_auth_code(&mode, &path, "x@qq.com", "code-x").unwrap();

    // 错误口令:能检测到文件存在但解不开
    let bad = credentials::read_file_map_pub(&path, "错误口令!");
    assert!(bad.is_err(), "错误口令必须被验证块拒绝");

    // 未解锁(无运行期口令):明确报错而不是返回空数据
    credentials::set_runtime_passphrase(None);
    let locked = credentials::load_auth_code(&mode, &path, "x@qq.com");
    assert!(locked.is_err());

    // 正确口令解锁后恢复正常
    credentials::set_runtime_passphrase(Some(TEST_PASS.to_string()));
    assert_eq!(
        credentials::load_auth_code(&mode, &path, "x@qq.com").unwrap(),
        "code-x"
    );
}

#[test]
#[serial]
fn v1_machine_bound_file_still_readable() {
    // v1(口令功能前)格式:{"v":1,"data":<机器绑定 AES-GCM base64>}
    // 新代码必须兼容读取,不能报"缺少必要字段"
    let dir = std::env::temp_dir().join(format!("maildeck-creds-v1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    credentials::set_runtime_passphrase(None);

    let map = serde_json::json!({ "old@qq.com": "code-old", "legacy@163.com": "code-163" });
    let plain = serde_json::to_string(&map).unwrap();
    let wrapper = serde_json::json!({
        "v": 1,
        "data": maildeck_lib::crypto::encrypt_b64(plain.as_bytes()),
    });
    std::fs::write(dir.join("credentials.json"), serde_json::to_string_pretty(&wrapper).unwrap()).unwrap();

    // 无口令也可读(v1 是机器绑定加密)
    let mode = CredentialMode::File;
    assert_eq!(
        credentials::load_auth_code(&mode, &dir, "old@qq.com").unwrap(),
        "code-old"
    );
    assert_eq!(
        credentials::load_auth_code(&mode, &dir, "legacy@163.com").unwrap(),
        "code-163"
    );
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
