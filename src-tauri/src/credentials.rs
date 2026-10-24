//! 凭证存储:默认走系统凭证库(keyring,macOS Keychain);
//! 用户可在设置中显式切换为本地文件模式(明文 JSON,权限 600,风险自担)。
//! 两种模式 API 一致,由 CredentialMode 分发。

use std::path::Path;

use keyring::Entry;

use crate::app_settings::CredentialMode;
use crate::error::AppResult;

const SERVICE: &str = "com.maildeck.desktop";
const FILE_NAME: &str = "credentials.json";

fn keyring_entry(email: &str) -> AppResult<Entry> {
    Ok(Entry::new(SERVICE, &format!("qq:{email}"))?)
}

fn file_path(app_dir: &Path) -> PathBuf {
    app_dir.join(FILE_NAME)
}

fn read_file_map(app_dir: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read_to_string(file_path(app_dir))
        .ok()
        .and_then(|text| {
            let wrapper: serde_json::Value = serde_json::from_str(&text).ok()?;
            let data = wrapper.get("data")?.as_str()?;
            let plain = crate::crypto::decrypt_b64(data)?;
            serde_json::from_str(&String::from_utf8(plain).ok()?).ok()
        })
        .unwrap_or_default()
}

fn write_file_map(app_dir: &Path, map: &std::collections::BTreeMap<String, String>) -> AppResult<()> {
    let plain = serde_json::to_string(map)
        .map_err(|e| crate::error::err(format!("序列化凭证失败:{e}")))?;
    let wrapper = serde_json::json!({ "v": 1, "data": crate::crypto::encrypt_b64(plain.as_bytes()) });
    std::fs::write(file_path(app_dir), serde_json::to_string_pretty(&wrapper).unwrap())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file_path(app_dir), std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn save_auth_code(mode: &CredentialMode, app_dir: &Path, email: &str, code: &str) -> AppResult<()> {
    match mode {
        CredentialMode::Keyring => keyring_entry(email)?.set_password(code)?,
        CredentialMode::File => {
            let mut map = read_file_map(app_dir);
            map.insert(email.to_string(), code.to_string());
            write_file_map(app_dir, &map)?;
        }
    }
    Ok(())
}

pub fn load_auth_code(mode: &CredentialMode, app_dir: &Path, email: &str) -> AppResult<String> {
    match mode {
        CredentialMode::Keyring => Ok(keyring_entry(email)?.get_password()?),
        CredentialMode::File => read_file_map(app_dir)
            .get(email)
            .cloned()
            .ok_or_else(|| crate::error::err(format!("本地凭证文件中没有 {email} 的授权码"))),
    }
}

pub fn delete_auth_code(mode: &CredentialMode, app_dir: &Path, email: &str) -> AppResult<()> {
    match mode {
        CredentialMode::Keyring => {
            let e = keyring_entry(email)?;
            match e.get_password() {
                Ok(_) => {
                    e.delete_credential()?;
                    Ok(())
                }
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(other) => Err(other.into()),
            }
        }
        CredentialMode::File => {
            let mut map = read_file_map(app_dir);
            if map.remove(email).is_some() {
                write_file_map(app_dir, &map)?;
            }
            Ok(())
        }
    }
}

use std::path::PathBuf;
