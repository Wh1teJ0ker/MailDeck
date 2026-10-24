//! 在线更新:版本比较纯逻辑 + Tauri updater 插件封装(命令层在 commands)。

/// 判断远端版本是否比当前版本新。
/// 只比较语义化版本的主/次/修订三元组,忽略 v 前缀、预发布与构建后缀;
/// 任一侧解析失败按"无更新"处理,保证检查失败不影响使用。
pub fn is_newer_version(remote: &str, current: &str) -> bool {
    match (parse_version(remote), parse_version(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

fn parse_version(input: &str) -> Option<(u64, u64, u64)> {
    let s = input.trim().trim_start_matches(['v', 'V']);
    // 去掉预发布(-alpha.1)与构建(+build)后缀
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    // 出现第四段视为非法
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}
