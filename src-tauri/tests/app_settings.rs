//! 应用设置文件(后端所有)读写测试。
//! 运行:`cargo test --test app_settings`

use maildeck_lib::app_settings::{self, CredentialMode, AppSettings};
use std::path::PathBuf;

fn temp_path(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("maildeck-settings-{}-{}.json", tag, std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn missing_file_defaults_to_keyring() {
    let p = temp_path("missing");
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store, CredentialMode::Keyring);
}

#[test]
fn roundtrip_save_and_load() {
    let p = temp_path("roundtrip");
    app_settings::save(&p, &AppSettings { credential_store: CredentialMode::File }).unwrap();
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store, CredentialMode::File);
}

#[test]
fn corrupt_file_falls_back_to_default() {
    let p = temp_path("corrupt");
    std::fs::write(&p, "{ not valid json !!!").unwrap();
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store, CredentialMode::Keyring);
}

#[test]
fn unknown_mode_string_falls_back_to_default() {
    let p = temp_path("unknown");
    std::fs::write(&p, r#"{"credential_store":"magic"}"#).unwrap();
    assert_eq!(app_settings::load(&p).credential_store, CredentialMode::Keyring);
}

#[test]
fn mode_string_roundtrip() {
    assert_eq!(CredentialMode::from_str("keyring"), CredentialMode::Keyring);
    assert_eq!(CredentialMode::from_str("file"), CredentialMode::File);
    assert_eq!(CredentialMode::from_str("其他"), CredentialMode::Keyring);
    assert_eq!(CredentialMode::File.as_str(), "file");
    assert_eq!(CredentialMode::Keyring.as_str(), "keyring");
}
