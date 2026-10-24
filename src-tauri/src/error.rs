use std::fmt;

/// 面向前端的错误类型:序列化为纯字符串,前端直接展示。
#[derive(Debug)]
pub struct AppError(pub String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError(format!("本地数据库错误:{e}"))
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError(format!("系统凭证库错误:{e}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(format!("文件读写错误:{e}"))
    }
}

impl From<async_imap::error::Error> for AppError {
    fn from(e: async_imap::error::Error) -> Self {
        AppError(format!("IMAP 错误:{e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn err(msg: impl Into<String>) -> AppError {
    AppError(msg.into())
}
