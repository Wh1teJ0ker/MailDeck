//! 邮件解析(mail-parser)与 HTML 安全清理(ammonia)。
//! 邮件 CSS(<style> 块、style 属性)保留——执行与外联均被 iframe 沙箱 + CSP 阻断;
//! cid 内嵌图片转 data URI 直接展示;超大内嵌图跳过。

use base64::Engine;
use mail_parser::{MessageParser, MimeHeaders};

/// 单张内嵌图的嵌入上限
const INLINE_IMAGE_MAX: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct HeaderSummary {
    pub subject: Option<String>,
    pub from_name: Option<String>,
    pub from_addr: Option<String>,
    pub recipient: Option<String>,
    pub message_id: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedBody {
    pub text: Option<String>,
    pub html: Option<String>,
    pub has_attachment: bool,
    /// (文件名, MIME 类型, 大小)
    pub attachments: Vec<(String, String, i64)>,
}

/// 解析头部字段(UID FETCH 的 BODY.PEEK[HEADER] 结果)。
pub fn parse_header_summary(bytes: &[u8]) -> HeaderSummary {
    let Some(msg) = MessageParser::default().parse(bytes) else {
        return HeaderSummary::default();
    };

    let (from_name, from_addr) = msg
        .from()
        .and_then(|list| list.first())
        .map(|a| {
            (
                a.name.as_deref().map(str::to_string),
                a.address.as_deref().map(str::to_string),
            )
        })
        .unwrap_or((None, None));

    HeaderSummary {
        subject: msg.subject().map(str::to_string),
        from_name,
        from_addr,
        recipient: msg
            .to()
            .and_then(|list| list.first())
            .and_then(|a| a.address.as_deref())
            .map(str::to_string),
        message_id: msg.message_id().map(str::to_string),
        timestamp: msg.date().map(|d| d.to_timestamp()),
    }
}

/// 解析完整原始邮件,产出展示用正文与附件元数据。
/// cid 内嵌图片(≤ 单张 4MB)转成 data URI 嵌入 HTML,不进入附件列表。
pub fn extract_body(raw: &[u8]) -> ParsedBody {
    let Some(msg) = MessageParser::default().parse(raw) else {
        return ParsedBody::default();
    };
    let text = msg.body_text(0).map(|c| c.into_owned());

    // 先收集内嵌图:cid → data URI
    let mut inlines: Vec<(String, String)> = Vec::new();
    let mut attachments: Vec<(String, String, i64)> = Vec::new();
    for part in msg.attachments() {
        let mime = part
            .content_type()
            .map(|ct| match &ct.c_subtype {
                Some(sub) => format!("{}/{}", ct.c_type, sub),
                None => ct.c_type.to_string(),
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let bytes = part.contents();
        let cid = part
            .content_id()
            .map(|s| s.trim().trim_matches(['<', '>']).to_string());

        if let Some(cid) = cid {
            if mime.starts_with("image/") && bytes.len() <= INLINE_IMAGE_MAX {
                inlines.push((
                    cid,
                    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)),
                ));
                continue;
            }
        }
        let name = part.attachment_name().unwrap_or("未命名附件").to_string();
        attachments.push((name, mime, bytes.len() as i64));
    }

    let mut html = msg.body_html(0).map(|h| sanitize_html(&h));
    if let Some(page) = html.as_mut() {
        for (cid, uri) in &inlines {
            *page = page.replace(&format!("cid:{cid}"), uri);
        }
    }

    ParsedBody {
        text,
        html,
        has_attachment: !attachments.is_empty(),
        attachments,
    }
}

/// 白名单式清理:剥除脚本、iframe、事件属性与主动导航;
/// 保留邮件 CSS(<style> 块、style/class 属性)与 cid/mailto 链接,
/// 远程资源因 URL scheme 受限被移除(沙箱 + CSP 双重兜底)。
pub fn sanitize_html(input: &str) -> String {
    ammonia::Builder::new()
        .add_tags(["style"])
        .rm_clean_content_tags(["style"])
        .add_generic_attributes(["style", "class", "dir"])
        .url_schemes(["mailto", "cid"].into_iter().collect())
        .link_rel(Some("noopener noreferrer"))
        .clean(input)
        .to_string()
}
