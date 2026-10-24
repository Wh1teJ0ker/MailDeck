//! 服务商类型注册表 + IMAP 文件夹分类 + 按服务商分发连接。
//! 新增服务商时在 REGISTRY 登记(含端点),并在 connect_for 中接线。

pub mod qq;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Endpoint {
    pub host: &'static str,
    pub port: u16,
    /// 登录后是否必须先上报 IMAP ID(网易系要求,否则拒绝操作)
    pub requires_id: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub auth_mode: &'static str,
    pub available: bool,
    pub hint: &'static str,
    pub endpoint: Endpoint,
}

pub const REGISTRY: &[ProviderMeta] = &[
    ProviderMeta {
        id: "qq",
        name: "QQ 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "在 QQ 邮箱网页版「设置 → 账户」开启 IMAP 服务并生成授权码",
        endpoint: Endpoint { host: "imap.qq.com", port: 993, requires_id: false },
    },
    ProviderMeta {
        id: "163",
        name: "163 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "在 163 邮箱网页版「设置 → POP3/SMTP/IMAP」开启 IMAP 并生成客户端授权密码",
        endpoint: Endpoint { host: "imap.163.com", port: 993, requires_id: true },
    },
    ProviderMeta {
        id: "outlook",
        name: "Outlook",
        auth_mode: "OAuth2",
        available: false,
        hint: "即将支持,将使用 Microsoft 账号浏览器授权",
        endpoint: Endpoint { host: "outlook.office365.com", port: 993, requires_id: false },
    },
];

pub fn registry() -> &'static [ProviderMeta] {
    REGISTRY
}

pub fn meta(id: &str) -> Option<&'static ProviderMeta> {
    REGISTRY.iter().find(|p| p.id == id)
}

/// 按服务商建立已认证的 IMAP 会话(含 163 等要求的登录后 ID 上报)。
pub async fn connect_for(
    provider_id: &str,
    email: &str,
    auth_code: &str,
) -> anyhow::Result<qq::ImapSession> {
    let meta = meta(provider_id).ok_or_else(|| anyhow::anyhow!("未知邮箱类型:{provider_id}"))?;
    qq::connect_with(
        meta.endpoint.host,
        meta.endpoint.port,
        email,
        auth_code,
        meta.endpoint.requires_id,
    )
    .await
}

/// 文件夹角色:special-use 旗标优先,名字启发式兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MailboxRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Spam,
    Other,
}

impl MailboxRole {
    pub fn as_str(self) -> &'static str {
        match self {
            MailboxRole::Inbox => "inbox",
            MailboxRole::Sent => "sent",
            MailboxRole::Drafts => "drafts",
            MailboxRole::Trash => "trash",
            MailboxRole::Spam => "spam",
            MailboxRole::Other => "other",
        }
    }
}

/// 依据 SPECIAL-USE 旗标与名字启发式判定文件夹角色。
pub fn classify_mailbox(name: &str, flags: &[String]) -> MailboxRole {
    for f in flags {
        match f.as_str() {
            "\\Sent" => return MailboxRole::Sent,
            "\\Drafts" => return MailboxRole::Drafts,
            "\\Trash" => return MailboxRole::Trash,
            "\\Junk" => return MailboxRole::Spam,
            _ => {}
        }
    }
    let lower = name.to_lowercase();
    if lower == "inbox" {
        return MailboxRole::Inbox;
    }
    if lower.contains("sent") || name.contains("已发送") {
        return MailboxRole::Sent;
    }
    if lower.contains("draft") || name.contains("草稿") {
        return MailboxRole::Drafts;
    }
    if lower.contains("trash") || lower.contains("deleted") || name.contains("已删除") || name.contains("废纸") {
        return MailboxRole::Trash;
    }
    if lower.contains("junk") || lower.contains("spam") || name.contains("垃圾") {
        return MailboxRole::Spam;
    }
    MailboxRole::Other
}
