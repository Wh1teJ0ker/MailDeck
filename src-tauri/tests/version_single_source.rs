//! 版本一致性测试:单一事实源约束。
//! 运行:`cargo test --test version_single_source`

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // 测试二进制位于 src-tauri/target/debug(deps),向上找仓库根
    let mut p = std::env::current_exe().unwrap();
    for _ in 0..6 {
        p.pop();
        if p.join("package.json").exists() && p.join("src-tauri/Cargo.toml").exists() {
            return p;
        }
    }
    panic!("找不到仓库根");
}

fn cargo_version() -> String {
    let text = std::fs::read_to_string(repo_root().join("src-tauri/Cargo.toml")).unwrap();
    for line in text.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("version") {
            let v = v.trim_start().strip_prefix('=').unwrap().trim();
            return v.trim_matches('"').to_string();
        }
    }
    panic!("Cargo.toml 没有 version 字段");
}

fn conf_version() -> Option<String> {
    let text = std::fs::read_to_string(repo_root().join("src-tauri/tauri.conf.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json.get("version").and_then(|v| v.as_str()).map(String::from)
}

fn pkg_version() -> String {
    let text = std::fs::read_to_string(repo_root().join("package.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json.get("version").and_then(|v| v.as_str()).unwrap().to_string()
}

#[test]
fn cargo_is_the_single_source_of_truth() {
    // tauri.conf.json 若声明 version,必须与 Cargo.toml 一致;
    // 推荐做法是 conf 不写 version(由构建脚本注入)。
    if let Some(conf) = conf_version() {
        assert_eq!(conf, cargo_version(), "tauri.conf.json 的 version 与 Cargo.toml 不一致");
    }
    // package.json 展示用,必须跟随
    assert_eq!(pkg_version(), cargo_version(), "package.json 的 version 与 Cargo.toml 不一致");
}

#[test]
fn cargo_version_is_semver() {
    let v = cargo_version();
    let parts: Vec<&str> = v.split('.').collect();
    assert!(parts.len() == 3, "版本应为 x.y.z:{v}");
    for p in parts {
        assert!(p.chars().all(|c| c.is_ascii_digit()), "版本段应为数字:{v}");
    }
}
