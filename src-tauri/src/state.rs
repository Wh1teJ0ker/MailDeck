use std::path::PathBuf;

/// 数据层句柄:只持路径。
/// 数据库按命令短生命周期开连接(WAL 多读并发 + busy_timeout);
/// 应用设置与(可选的)凭证文件同目录。
pub struct AppState {
    pub db_path: PathBuf,
    pub app_dir: PathBuf,
}
