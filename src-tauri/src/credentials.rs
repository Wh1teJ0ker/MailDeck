//! 授权码仅存于系统凭证库(macOS Keychain),不落普通文件。
//! 命名规则:service = 应用标识,user = "qq:<邮箱地址>"。

use crate::error::AppResult;
use keyring::Entry;

const SERVICE: &str = "com.maildeck.desktop";

fn entry(email: &str) -> AppResult<Entry> {
    Ok(Entry::new(SERVICE, &format!("qq:{email}"))?)
}

pub fn save_auth_code(email: &str, code: &str) -> AppResult<()> {
    entry(email)?.set_password(code)?;
    Ok(())
}

pub fn load_auth_code(email: &str) -> AppResult<String> {
    Ok(entry(email)?.get_password()?)
}

pub fn delete_auth_code(email: &str) -> AppResult<()> {
    let e = entry(email)?;
    match e.get_password() {
        Ok(_) => {
            e.delete_credential()?;
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(other) => Err(other.into()),
    }
}
