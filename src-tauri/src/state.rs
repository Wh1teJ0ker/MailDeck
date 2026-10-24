use std::path::PathBuf;

/// 数据层句柄:只持数据库文件路径。
/// 每个命令在 spawn_blocking 内短生命周期打开自己的 SQLite 连接
/// (WAL 多读并发 + busy_timeout),网络等待期间不阻塞其他命令。
pub struct AppState {
    pub db_path: PathBuf,
}
