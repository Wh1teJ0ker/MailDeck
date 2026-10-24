//! 服务商类型注册表 + IMAP 文件夹分类。

pub mod qq;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub auth_mode: &'static str,
    pub available: bool,
    pub hint: &'static str,
}

pub const REGISTRY: &[ProviderMeta] = &[
    ProviderMeta {
        id: "qq",
        name: "QQ 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "在 QQ 邮箱网页版「设置 → 账户」开启 IMAP 服务并生成授权码",
    },
    ProviderMeta {
        id: "outlook",
        name: "Outlook",
        auth_mode: "OAuth2",
        available: false,
        hint: "即将支持,将使用 Microsoft 账号浏览器授权",
    },
    ProviderMeta {
        id: "163",
        name: "163 邮箱",
        auth_mode: "授权码",
        available: false,
        hint: "即将支持",
    },
];

pub fn registry() -> &'static [ProviderMeta] {
    REGISTRY
}

pub fn meta(id: &str) -> Option<&'static ProviderMeta> {
    REGISTRY.iter().find(|p| p.id == id)
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
