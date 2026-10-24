//! 邮件解析与 HTML 安全清理的集成测试。
//! 运行:`cargo test --test mail`

use maildeck_lib::mail;

#[test]
fn sanitize_strips_script_form_and_remote_images() {
    let dirty = "<p>hello</p><script>alert(1)</script><form action=\"https://x\"></form>\
                 <img src=\"https://evil.com/a.png\">\
                 <img src=\"cid:part1\">\
                 <a href=\"https://evil.com\">link</a>\
                 <a href=\"mailto:a@qq.com\">mail</a>";
    let clean = mail::sanitize_html(dirty);
    assert!(clean.contains("<p>hello</p>"), "正文段落应保留:{clean}");
    assert!(!clean.contains("script"), "脚本应被剥除:{clean}");
    assert!(!clean.contains("form"), "表单应被剥除:{clean}");
    assert!(!clean.contains("evil.com"), "远程链接与图片应被移除:{clean}");
    assert!(clean.contains("cid:part1"), "内嵌 cid 图片应保留:{clean}");
    assert!(clean.contains("mailto:a@qq.com"), "mailto 链接应保留:{clean}");
}

#[test]
fn sanitize_strips_iframe_and_event_handlers() {
    let dirty = "<div onclick=\"evil()\">x</div><iframe src=\"https://evil.com\"></iframe>";
    let clean = mail::sanitize_html(dirty);
    assert!(!clean.contains("onclick"), "事件属性应被剥除:{clean}");
    assert!(!clean.contains("iframe"), "iframe 应被剥除:{clean}");
    assert!(clean.contains(">x</div>"), "正文应保留:{clean}");
}

#[test]
fn sanitize_keeps_email_css() {
    // 邮件 HTML 依赖 <style> 块与 style 属性,清理后必须保留(脚本仍被剥除)
    let dirty = "<style>p{color:red}</style><p style=\"font-size:14px\">hi</p>\
                 <style>@import url(https://evil.com/x.css)</style>";
    let clean = mail::sanitize_html(dirty);
    assert!(clean.contains("p{color:red}"), "<style> 内容应保留:{clean}");
    assert!(clean.contains("font-size:14px"), "style 属性应保留:{clean}");
    assert!(clean.contains("@import"), "CSS 文本原样保留(沙箱+CSP 阻止其生效)");
    assert!(!clean.contains("<script"), "脚本仍应剥除:{clean}");
}

#[test]
fn cid_inline_images_are_embedded_as_data_uris() {
    let png: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let raw = build_multipart_with_inline(png, "image/png", "part1.abc@qq.com", "logo");
    let body = mail::extract_body(&raw);
    let html = body.html.expect("html 应存在");
    assert!(
        html.contains("data:image/png;base64,"),
        "cid 内嵌图应转成 data URI:{html}"
    );
    assert!(!html.contains("cid:"), "不应残留 cid 引用:{html}");
    // 内嵌图片不算附件
    assert!(!body.has_attachment, "内嵌图不应出现在附件列表");
    assert!(body.attachments.is_empty());
}

#[test]
fn oversize_inline_image_is_skipped() {
    // 超过单张上限的内嵌图不内嵌,html 保留占位不 panic
    let big = vec![0x89u8; 5 * 1024 * 1024 + 1];
    let raw = build_multipart_with_inline(&big, "image/png", "big@qq.com", "big");
    let body = mail::extract_body(&raw);
    let html = body.html.expect("html 应存在");
    assert!(!html.contains("data:image"), "超大内嵌图不应转 data URI:{html}");
}

/// 构造 multipart/related + multipart/alternative + 内嵌图混排的样例邮件。
fn build_multipart_with_inline(
    img: &[u8],
    mime: &str,
    cid: &str,
    name: &str,
) -> Vec<u8> {
    let mut raw = Vec::new();
    raw.extend_from_slice(
        b"From: a@qq.com\r\nSubject: inline\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/related; boundary=\"OUTER\"\r\n\r\n\
--OUTER\r\n\
Content-Type: multipart/alternative; boundary=\"INNER\"\r\n\r\n\
--INNER\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nplain\r\n\
--INNER\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
<html><body><p>hi</p><img src=\"cid:",
    );
    raw.extend_from_slice(cid.as_bytes());
    raw.extend_from_slice(
        b"\"></body></html>\r\n\
--INNER--\r\n\
--OUTER\r\nContent-Type: ",
    );
    raw.extend_from_slice(mime.as_bytes());
    raw.extend_from_slice(b"; name=\"");
    raw.extend_from_slice(name.as_bytes());
    raw.extend_from_slice(b"\"\r\nContent-ID: <");
    raw.extend_from_slice(cid.as_bytes());
    raw.extend_from_slice(b">\r\nContent-Disposition: inline\r\n\r\n");
    raw.extend_from_slice(img);
    raw.extend_from_slice(b"\r\n--OUTER--\r\n");
    raw
}

#[test]
fn decodes_rfc2047_chinese_subject_and_sender() {
    // From 名与 Subject 均为 UTF-8 base64 编码词(主题:你好,世界)
    let raw: &[u8] = b"From: =?utf-8?B?6YOR5bee?=<friend@qq.com>\r\n\
Subject: =?utf-8?B?5L2g5aW977yM5LiW55WM?=\r\n\
Date: Sat, 24 Oct 2026 10:00:00 +0800\r\n\
To: me@qq.com\r\n\
Message-ID: <abc@qq.com>\r\n\r\n";
    let s = mail::parse_header_summary(raw);
    assert_eq!(s.subject.as_deref(), Some("你好，世界"));
    assert!(s.from_name.is_some(), "发件人显示名应解码");
    assert_eq!(s.from_addr.as_deref(), Some("friend@qq.com"));
    assert!(s.timestamp.is_some(), "头部日期应解析为时间戳");
    // mail-parser 返回的 Message-ID 不含尖括号
    assert_eq!(s.message_id.as_deref(), Some("abc@qq.com"));
}

#[test]
fn extracts_multipart_text_and_html() {
    let raw: &[u8] = b"From: a@qq.com\r\nSubject: t\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; boundary=\"BOUND\"\r\n\r\n\
--BOUND\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nhello plain\r\n\
--BOUND\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<b>hello html</b><script>x()</script>\r\n\
--BOUND--\r\n";
    let body = mail::extract_body(raw);
    assert_eq!(body.text.as_deref(), Some("hello plain"));
    let html = body.html.expect("html 正文应存在");
    assert!(html.contains("hello html"));
    assert!(!html.contains("script"), "html 正文必须经安全清理:{html}");
    assert!(!body.has_attachment);
}

#[test]
fn extracts_attachment_metadata() {
    let raw: &[u8] = b"From: a@qq.com\r\nSubject: with file\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"MX\"\r\n\r\n\
--MX\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nbody\r\n\
--MX\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\n\
Content-Disposition: attachment; filename=\"report.pdf\"\r\n\r\nPDFDATA\r\n\
--MX--\r\n";
    let body = mail::extract_body(raw);
    assert!(body.has_attachment);
    assert_eq!(body.attachments.len(), 1);
    let (name, mime, size) = &body.attachments[0];
    assert_eq!(name, "report.pdf");
    assert!(mime.starts_with("application/pdf"));
    assert_eq!(*size, 7, "附件大小按内容字节数计算");
}

#[test]
fn malformed_input_does_not_panic() {
    let s = mail::parse_header_summary(&[]);
    assert!(s.subject.is_none());
    let b = mail::extract_body(&[0xff, 0xfe, 0x00]);
    assert!(b.text.is_none() && b.html.is_none());
    // 空字符串清理不应产生内容
    assert!(mail::sanitize_html("").is_empty());
}
