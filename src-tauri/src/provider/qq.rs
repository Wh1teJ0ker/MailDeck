//! QQ 邮箱 IMAP 接入:imap.qq.com:993,IMAPS + 授权码登录。
//! MVP 阶段每次操作建立短连接,后续版本再引入常驻连接与 IDLE。

use anyhow::{anyhow, Context, Result};
use async_imap::Session;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

pub const HOST: &str = "imap.qq.com";
pub const PORT: u16 = 993;
pub const INBOX: &str = "INBOX";

pub type ImapSession = Session<TlsStream<TcpStream>>;

/// 进程级缓存:避免每次连接重复加载全量系统根证书
static TLS_CONNECTOR: std::sync::OnceLock<tokio_rustls::TlsConnector> = std::sync::OnceLock::new();

fn tls_connector() -> Result<tokio_rustls::TlsConnector> {
    if let Some(c) = TLS_CONNECTOR.get() {
        return Ok(c.clone());
    }
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    // 部分系统证书解析失败不致命,只要仍有可用根证书即可;一个都没有时在下方兜底报错
    let mut added = 0usize;
    for cert in native.certs {
        if roots.add(cert).is_ok() {
            added += 1;
        }
    }
    if added == 0 {
        return Err(anyhow!("系统证书库中没有可用的根证书"));
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector: tokio_rustls::TlsConnector = std::sync::Arc::new(config).into();
    let _ = TLS_CONNECTOR.set(connector.clone());
    Ok(connector)
}

/// 建立 IMAPS 连接并用授权码登录;post_login_id 为 true 时按网易系要求上报 IMAP ID。
pub async fn connect_with(
    host: &str,
    port: u16,
    email: &str,
    auth_code: &str,
    post_login_id: bool,
) -> Result<ImapSession> {
    let tcp = TcpStream::connect((host, port))
        .await
        .with_context(|| format!("连接 {host}:{port} 失败,请检查网络"))?;
    let server_name = rustls_pki_types::ServerName::try_from(host.to_string())
        .map_err(|_| anyhow!("主机名非法"))?;
    let stream = tls_connector()?
        .connect(server_name, tcp)
        .await
        .context("TLS 握手失败")?;
    let client = async_imap::Client::new(stream);
    let mut session = match client.login(email, auth_code).await {
        Ok(session) => session,
        Err((e, _client)) => {
            Err(anyhow!("登录被拒绝:{}", super::explain_login_failure(&e.to_string())))?
        }
    };
    if post_login_id {
        // 网易系(163 等)要求客户端先上报 ID,否则后续命令返回 Unsafe Login
        session
            .id([
                ("name", Some("MailDeck")),
                ("version", Some(env!("CARGO_PKG_VERSION"))),
                ("vendor", Some("MailDeck")),
            ])
            .await
            .context("IMAP ID 上报失败")?;
    }
    Ok(session)
}

/// QQ 邮箱连接(兼容旧调用点)。
pub async fn connect(email: &str, auth_code: &str) -> Result<ImapSession> {
    connect_with(HOST, PORT, email, auth_code, false).await
}

/// 拉取单封邮件的完整原始内容。
/// UID 命令要求先处于选中邮箱状态,这里自行 SELECT,调用方无需关心。
pub async fn fetch_raw_message(sess: &mut ImapSession, mailbox_raw: &str, uid: u32) -> Result<Vec<u8>> {
    sess.select(mailbox_raw).await.context("选择邮箱失败")?;
    let mut fetches = sess
        .uid_fetch(uid.to_string(), "(UID BODY.PEEK[])")
        .await
        .context("请求邮件正文失败")?;
    let mut raw: Option<Vec<u8>> = None;
    while let Some(item) = fetches.next().await {
        let f = item?;
        if f.uid == Some(uid) {
            if let Some(bytes) = f.body() {
                raw = Some(bytes.to_vec());
            }
        }
    }
    drop(fetches);
    raw.ok_or_else(|| anyhow!("服务器上未找到该邮件(UID {uid}),可能已被移出收件箱"))
}

/// 回写已读/未读标记;uids 支持 IMAP uid-set(如 "5,7,9")。
/// 同样自行 SELECT,保持自包含。
pub async fn set_seen_flag(sess: &mut ImapSession, mailbox_raw: &str, uids: &str, seen: bool) -> Result<()> {
    sess.select(mailbox_raw).await.context("选择邮箱失败")?;
    let cmd = if seen {
        "+FLAGS (\\Seen)"
    } else {
        "-FLAGS (\\Seen)"
    };
    let mut stream = sess
        .uid_store(uids, cmd)
        .await
        .context("更新已读状态失败")?;
    while stream.next().await.is_some() {}
    Ok(())
}

pub async fn logout(mut sess: ImapSession) {
    let _ = sess.logout().await;
}
