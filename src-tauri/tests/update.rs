//! 更新检查纯逻辑测试:版本号比较。
//! 运行:`cargo test --test update`

use maildeck_lib::updater::is_newer_version;

#[test]
fn detects_newer_minor_and_patch() {
    assert!(is_newer_version("0.2.0", "0.1.0"));
    assert!(is_newer_version("0.1.1", "0.1.0"));
    assert!(is_newer_version("1.0.0", "0.9.9"));
}

#[test]
fn same_or_older_is_not_newer() {
    assert!(!is_newer_version("0.1.0", "0.1.0"));
    assert!(!is_newer_version("0.1.0", "0.2.0"));
    assert!(!is_newer_version("0.1.9", "0.2.0"));
}

#[test]
fn tolerates_v_prefix_and_missing_patch() {
    assert!(is_newer_version("v0.2.0", "0.1.0"));
    assert!(is_newer_version("V0.2.0", "0.1.0"));
    assert!(is_newer_version("0.2", "0.1.9"));
    assert!(!is_newer_version("0.1", "0.1.0"));
}

#[test]
fn invalid_input_is_not_newer() {
    assert!(!is_newer_version("", "0.1.0"));
    assert!(!is_newer_version("abc", "0.1.0"));
    assert!(!is_newer_version("0.2.0", ""));
    assert!(!is_newer_version("0.2.0", "unknown"));
    assert!(!is_newer_version("1.2.3.4", "1.2.3"));
}
