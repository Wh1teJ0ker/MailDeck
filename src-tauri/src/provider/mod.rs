//! 服务商类型注册表 + IMAP 文件夹分类 + 按服务商分发连接。
//! 新增服务商时在 REGISTRY 登记(含端点),并在 connect_for 中接线。

pub mod gmail;
pub mod oauth2;
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
    /// 固定邮箱后缀(自动补全用);自定义域名企业邮为空串
    pub suffix: &'static str,
}

pub const REGISTRY: &[ProviderMeta] = &[
    ProviderMeta {
        id: "qq",
        name: "QQ 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "网页版「设置 → 账户」开启 IMAP/SMTP 服务并生成授权码",
        endpoint: Endpoint { host: "imap.qq.com", port: 993, requires_id: false },
        suffix: "qq.com",
    },
    ProviderMeta {
        id: "163",
        name: "163 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "网页版「设置 → POP3/SMTP/IMAP」开启 IMAP 并生成客户端授权密码",
        endpoint: Endpoint { host: "imap.163.com", port: 993, requires_id: true },
        suffix: "163.com",
    },
    ProviderMeta {
        id: "126",
        name: "126 邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "网页版「设置 → POP3/SMTP/IMAP」开启 IMAP 并生成授权密码(与 163 同为网易)",
        endpoint: Endpoint { host: "imap.126.com", port: 993, requires_id: true },
        suffix: "126.com",
    },
    ProviderMeta {
        id: "exmail",
        name: "腾讯企业邮",
        auth_mode: "授权码",
        available: true,
        hint: "管理后台开启 IMAP/SMTP;地址为企业域名,请输入完整邮箱",
        endpoint: Endpoint { host: "imap.exmail.qq.com", port: 993, requires_id: false },
        suffix: "",
    },
    ProviderMeta {
        id: "icloud",
        name: "iCloud 邮件",
        auth_mode: "应用专用密码",
        available: true,
        hint: "appleid.apple.com 开启双重认证后生成 App 专用密码",
        endpoint: Endpoint { host: "imap.mail.me.com", port: 993, requires_id: false },
        suffix: "icloud.com",
    },
    ProviderMeta {
        id: "sohu",
        name: "搜狐邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "网页版邮箱设置中开启 IMAP 并设置客户端密码",
        endpoint: Endpoint { host: "imap.sohu.com", port: 993, requires_id: false },
        suffix: "sohu.com",
    },
    ProviderMeta {
        id: "sina",
        name: "新浪邮箱",
        auth_mode: "授权码",
        available: true,
        hint: "网页版「设置 → 客户端 POP/IMAP/SMTP」开启并设置授权密码",
        endpoint: Endpoint { host: "imap.sina.com", port: 993, requires_id: false },
        suffix: "sina.com",
    },
    ProviderMeta {
        id: "aliyun",
        name: "阿里企业邮",
        auth_mode: "授权码",
        available: true,
        hint: "管理后台开启 IMAP;地址为企业域名,请输入完整邮箱",
        endpoint: Endpoint { host: "imap.qiye.aliyun.com", port: 993, requires_id: false },
        suffix: "",
    },
    ProviderMeta {
        id: "gmail",
        name: "Gmail",
        auth_mode: "OAuth2",
        available: true,
        hint: "需要 Google OAuth 客户端(client_id),在设置中填写后浏览器授权",
        endpoint: Endpoint { host: "imap.gmail.com", port: 993, requires_id: false },
        suffix: "gmail.com",
    },
    ProviderMeta {
        id: "outlook",
        name: "Outlook",
        auth_mode: "OAuth2",
        available: false,
        hint: "即将支持,将使用 Microsoft 账号浏览器授权",
        endpoint: Endpoint { host: "outlook.office365.com", port: 993, requires_id: false },
        suffix: "",
    },
];

pub fn registry() -> &'static [ProviderMeta] {
    REGISTRY
}

pub fn meta(id: &str) -> Option<&'static ProviderMeta> {
    REGISTRY.iter().find(|p| p.id == id)
}

/// 按服务商建立已认证的 IMAP 会话。
/// auth_code 语义随服务商:授权码(QQ/163)或 OAuth2 access token(Gmail)。
pub async fn connect_for(
    provider_id: &str,
    email: &str,
    auth_code: &str,
) -> anyhow::Result<qq::ImapSession> {
    let meta = meta(provider_id).ok_or_else(|| anyhow::anyhow!("未知邮箱类型:{provider_id}"))?;
    match provider_id {
        "gmail" => gmail::connect_oauth(email, auth_code).await,
        _ => {
            qq::connect_with(
                meta.endpoint.host,
                meta.endpoint.port,
                email,
                auth_code,
                meta.endpoint.requires_id,
            )
            .await
        }
    }
}

/// OAuth 型服务商判定(Gmail 等;新增 OAuth 服务商在此登记)。
pub fn is_oauth_provider(provider_id: &str) -> bool {
    matches!(provider_id, "gmail")
}

/// 按账号建立会话:密码型读授权码,OAuth 型读 refresh token 并自动续期。
pub async fn connect_account(
    cred: &crate::app_settings::CredCtx,
    gmail_client_id: &str,
    provider_id: &str,
    email: &str,
) -> anyhow::Result<qq::ImapSession> {
    if is_oauth_provider(provider_id) {
        if gmail_client_id.is_empty() {
            return Err(anyhow::anyhow!("未配置 Google OAuth 客户端 ID,请在设置中填写"));
        }
        let refresh = crate::credentials::load_auth_code(&cred.mode, &cred.dir, email)
            .map_err(|e| anyhow::anyhow!("读取 Gmail 凭证失败:{e}"))?;
        let (access, rotated) = gmail::access_token(gmail_client_id, email, &refresh).await?;
        // Google 令牌轮换:返回了新 refresh token 时写回凭证库
        if let Some(new_refresh) = rotated {
            if new_refresh != refresh {
                let _ = crate::credentials::save_auth_code(&cred.mode, &cred.dir, email, &new_refresh);
            }
        }
        gmail::connect_oauth(email, &access).await
    } else {
        let code = crate::credentials::load_auth_code(&cred.mode, &cred.dir, email)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        connect_for(provider_id, email, &code).await
    }
}

/// 把服务器返回的登录拒绝信息归因为可操作的中文排查清单,
/// 并保留原始信息便于进一步排查;无法识别的错误原样透传。
pub fn explain_login_failure(raw: &str) -> String {
    if raw.contains("AUTHENTICATIONFAILED") {
        format!(
            "邮箱服务器拒绝了登录。请依次检查:\n\
             1. 网页版邮箱设置中 IMAP/SMTP 服务已开启;\n\
             2. 填的是 16 位授权码(不是邮箱密码),且复制完整;\n\
             3. 最近改过邮箱密码,授权码已失效,需重新生成;\n\
             4. 短时间多次登录会被临时限制,等待 15~30 分钟或重新生成授权码后重试;\n\
             5. 账号状态正常(可先在网页版登录一次确认)。\n\
             ——服务器原始信息:{raw}"
        )
    } else if raw.contains("Unsafe Login") || raw.contains("unsafe login") {
        format!(
            "163 等网易邮箱要求登录后先上报客户端 ID(MailDeck 会自动处理)。\n\
             若反复出现,请更新到最新版本,并把以下信息反馈给开发者。——服务器原始信息:{raw}"
        )
    } else {
        raw.to_string()
    }
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
