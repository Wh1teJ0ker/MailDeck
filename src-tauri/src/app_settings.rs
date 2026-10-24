//! 应用级设置(后端所有,存于应用数据目录 settings.json)。
//! 凭证存储模式在此登记,凭证读写按它分发;切换时由命令层负责迁移。

use std::path::Path;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialMode {
    Keyring,
    File,
}

impl CredentialMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CredentialMode::Keyring => "keyring",
            CredentialMode::File => "file",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "file" => CredentialMode::File,
            _ => CredentialMode::Keyring, // 默认与兜底:系统凭证库
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub credential_store: CredentialMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings { credential_store: CredentialMode::Keyring }
    }
}

/// 读取设置;文件缺失或损坏时返回默认值(系统凭证库)。
pub fn load(path: &Path) -> AppSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str::<AppSettings>(&text).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save(path: &Path, settings: &AppSettings) -> AppResult<()> {
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| crate::error::err(format!("序列化设置失败:{e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

/// 当前凭证存储模式(便捷入口)。
pub fn credential_mode(path: &Path) -> CredentialMode {
    load(path).credential_store
}

/// 凭证上下文:模式 + 本地文件目录,随调用链透传。
#[derive(Debug, Clone)]
pub struct CredCtx {
    pub mode: CredentialMode,
    pub dir: std::path::PathBuf,
}

pub fn cred_ctx(app_dir: &Path) -> CredCtx {
    CredCtx { mode: credential_mode(&app_dir.join("settings.json")), dir: app_dir.to_path_buf() }
}
