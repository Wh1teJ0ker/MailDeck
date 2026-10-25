use maildeck_lib::app_settings::{self, AppSettings};
use std::path::PathBuf;

fn temp_path(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("maildeck-settings-{}-{}.json", tag, std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn missing_file_defaults() {
    let p = temp_path("missing");
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store.as_str(), "keyring");
    assert_eq!(s.background, "rain");
    assert_eq!(s.glass_tone, "dark");
}

#[test]
fn roundtrip_save_and_load() {
    let p = temp_path("roundtrip");
    app_settings::save(
        &p,
        &AppSettings {
            credential_store: maildeck_lib::app_settings::CredentialMode::File,
            background: "custom".into(),
            glass_tone: "light".into(),
        },
    )
    .unwrap();
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store.as_str(), "file");
    assert_eq!(s.background, "custom");
    assert_eq!(s.glass_tone, "light");
}

#[test]
fn corrupt_file_falls_back_to_default() {
    let p = temp_path("corrupt");
    std::fs::write(&p, "{ not valid json !!!").unwrap();
    let s = app_settings::load(&p);
    assert_eq!(s.background, "rain");
    assert_eq!(s.glass_tone, "dark");
}

#[test]
fn partial_json_keeps_known_fields() {
    let p = temp_path("partial");
    std::fs::write(&p, r#"{"credential_store":"file"}"#).unwrap();
    let s = app_settings::load(&p);
    assert_eq!(s.credential_store.as_str(), "file");
    assert_eq!(s.background, "rain", "缺失字段取默认");
}
