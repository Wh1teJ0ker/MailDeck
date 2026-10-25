//! 凭证存储:默认走系统凭证库(keyring,macOS Keychain);
//! 用户可在设置中显式切换为本地文件模式——密钥由用户口令经 PBKDF2 派生,
//! 本地只存盐 + 验证块,不存密钥。两种模式 API 一致,由 CredentialMode 分发。

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
    read_file_map_with(app_dir, runtime_passphrase().as_deref()).unwrap_or_default()
}

/// 用给定口令解密凭证文件;口令未设置/错误/文件损坏均返回 Err(由调用方决定语义)。
fn read_file_map_with(
    app_dir: &Path,
    passphrase: Option<&str>,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    let text = std::fs::read_to_string(file_path(app_dir))?;
    let wrapper: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| crate::error::err(format!("凭证文件损坏:{e}")))?;
    let data = wrapper
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_bad()?;

    // v1(口令功能前):机器绑定 AES-GCM,无 salt/verifier;保持兼容读取
    if wrapper.get("salt").is_none() {
        let plain = crate::crypto::decrypt_b64(&data)
            .ok_or_else(|| crate::error::err("凭证文件解密失败(可能来自其他机器)"))?;
        return serde_json::from_slice(&plain)
            .map_err(|e| crate::error::err(format!("凭证文件损坏:{e}")));
    }

    // v2:口令派生密钥(盐 + 验证块)
    let salt_hex = wrapper.get("salt").and_then(|v| v.as_str()).ok_or_bad()?;
    let verifier = wrapper.get("verifier").and_then(|v| v.as_str()).ok_or_bad()?;
    let pass = passphrase
        .ok_or_else(|| crate::error::err("凭证文件已启用口令保护,请先在设置中输入口令解锁"))?;
    let salt = hex_decode(&salt_hex)?;
    let verifier = hex_decode(&verifier)?;
    if !crate::crypto::verify_passphrase(pass, &salt, &verifier) {
        return Err(crate::error::err("口令不正确"));
    }
    let plain = crate::crypto::decrypt_b64_with_passphrase(pass, &salt, &data)
        .ok_or_else(|| crate::error::err("凭证解密失败"))?;
    serde_json::from_slice(&plain).map_err(|e| crate::error::err(format!("凭证文件损坏:{e}")))
}

trait OptErr {
    fn ok_or_bad(self) -> AppResult<String>;
}
impl OptErr for Option<&str> {
    fn ok_or_bad(self) -> AppResult<String> {
        self.map(|s| s.to_string()).ok_or_else(|| crate::error::err("凭证文件损坏:缺少必要字段"))
    }
}

fn hex_decode(s: &str) -> AppResult<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| crate::error::err("凭证文件编码损坏"))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 运行期口令:仅存在内存(全局,进程退出即消失),解锁后本会话内免重复输入。
static RUNTIME_PASSPHRASE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn set_runtime_passphrase(pass: Option<String>) {
    *RUNTIME_PASSPHRASE.lock().unwrap() = pass;
}

fn runtime_passphrase_impl() -> Option<String> {
    RUNTIME_PASSPHRASE.lock().unwrap().clone()
}

fn write_file_map(app_dir: &Path, map: &std::collections::BTreeMap<String, String>) -> AppResult<()> {
    let pass = runtime_passphrase()
        .ok_or_else(|| crate::error::err("凭证文件已启用口令保护,请先在设置中输入口令"))?;
    let plain = serde_json::to_string(map)
        .map_err(|e| crate::error::err(format!("序列化凭证失败:{e}")))?;
    let salt = crate::crypto::random_salt();
    let wrapper = serde_json::json!({
        "v": 2,
        "salt": hex_encode(&salt),
        "verifier": hex_encode(&crate::crypto::make_verifier(&pass, &salt)),
        "data": crate::crypto::encrypt_b64_with_passphrase(&pass, &salt, plain.as_bytes()),
    });
    std::fs::write(file_path(app_dir), serde_json::to_string_pretty(&wrapper).unwrap())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file_path(app_dir), std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// 供命令层解锁校验:用口令读取并返回完整映射。
pub fn read_file_map_pub(app_dir: &Path, passphrase: &str) -> AppResult<std::collections::BTreeMap<String, String>> {
    read_file_map_with(app_dir, Some(passphrase))
}

/// 供命令层查询解锁状态。
pub fn runtime_passphrase() -> Option<String> {
    runtime_passphrase_impl()
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
        CredentialMode::File => read_file_map_with(app_dir, runtime_passphrase().as_deref())?
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
