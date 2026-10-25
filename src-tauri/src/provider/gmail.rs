//! Gmail 接入:IMAP XOAUTH2 + OAuth2 浏览器授权(PKCE)。
//! 需要 Google Cloud OAuth 客户端(Desktop 类型),client_id 由用户在设置中填写。
//! access token 仅驻内存(按邮箱缓存,过期自动刷新);本地持久化的是 refresh token。

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::oauth2;
use super::qq::{self, ImapSession};

pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
/// Gmail IMAP(含 SMTP 权限的 scope 集) + 账号地址读取
pub const SCOPES: &[&str] = &[
    "https://mail.google.com/",
    "https://www.googleapis.com/auth/userinfo.email",
];

pub const CLIENT_ID_KEY: &str = "gmail_client_id";

/// 构造授权 URL(PKCE)。
pub fn authorize_url(client_id: &str, redirect_uri: &str, code_challenge: &str, state: &str) -> String {
    let scope = SCOPES.join(" ");
    format!(
        "{}?client_id={}&response_type=code&scope={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        AUTH_ENDPOINT,
        urlenc(client_id),
        urlenc(&scope),
        urlenc(redirect_uri),
        urlenc(code_challenge),
        urlenc(state),
    )
}

fn urlenc(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('?', "%3F")
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn base64_url(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// 预绑定本地回环端口,返回 (端口, 等待回调闭包)。
/// 调用方先用端口构造 redirect_uri 打开浏览器,再 await 等待授权完成。
pub async fn bind_loopback() -> Result<(u16, impl Future<Output = Result<(String, String)>>)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定本地回环端口失败")?;
    let port = listener.local_addr()?.port();

    let fut = async move {
        let (mut sock, _) = tokio::time::timeout(std::time::Duration::from_secs(300), listener.accept())
            .await
            .map_err(|_| anyhow!("授权超时(5 分钟未完成)"))?
            .context("接受回环连接失败")?;

        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.context("读取重定向请求失败")?;
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let first_line = req.lines().next().unwrap_or_default();
        let query = first_line.split_whitespace().nth(1).unwrap_or_default();
        let query = query.split('?').nth(1).unwrap_or_default();

        let mut code = None;
        let mut state = None;
        let mut err: Option<String> = None;
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            match (kv.next().unwrap_or_default(), kv.next()) {
                ("code", Some(v)) => code = Some(urldecode(v)),
                ("state", Some(v)) => state = Some(urldecode(v)),
                ("error", Some(v)) => err = Some(urldecode(v)),
                _ => {}
            }
        }
        let page = |msg: &str| {
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<h2>MailDeck</h2><p>{msg}</p>")
        };
        if let Some(e) = err {
            let _ = sock.write_all(page(&format!("授权失败:{e}")).as_bytes()).await;
            return Err(anyhow!("授权被拒绝:{e}"));
        }
        let (code, state) = match (code, state) {
            (Some(c), Some(s)) => (c, s),
            _ => {
                let _ = sock.write_all(page("授权回调缺少参数").as_bytes()).await;
                return Err(anyhow!("授权回调缺少 code/state"));
            }
        };
        let _ = sock.write_all(page("授权成功,请回到应用").as_bytes()).await;
        let _ = sock.flush().await;
        Ok((code, state))
    };
    Ok((port, fut))
}

use std::future::Future;

/// 完整授权流程:打开浏览器 → 等回调 → PKCE 换令牌。
/// 返回 (access_token, refresh_token, email)。
pub async fn authorize_and_exchange(client_id: &str) -> Result<(String, String, String)> {
    use rand::RngCore;

    let (port, wait) = bind_loopback().await?;
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let (verifier, challenge) = oauth2::pkce_pair();
    let state = {
        let mut b = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut b);
        base64_url(&b)
    };
    let url = authorize_url(client_id, &redirect_uri, &challenge, &state);

    open::that_detached(&url).context("打开系统浏览器失败")?;
    let (code, returned_state) = wait.await?;
    if returned_state != state {
        return Err(anyhow!("授权回调 state 不匹配,疑似跨站请求伪造"));
    }
    let tokens = oauth2::exchange_code(TOKEN_ENDPOINT, client_id, &code, &verifier, &redirect_uri).await?;
    let refresh = tokens
        .refresh_token
        .ok_or_else(|| anyhow!("Google 未返回 refresh_token;请移除本应用的访问授权后重新走一次"))?;
    let email = get_email(&tokens.access_token).await?;
    Ok((tokens.access_token, refresh, email))
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    email: String,
}

/// 用 access token 换账号邮箱地址。
pub async fn get_email(access_token: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await
        .context("读取 Google 账号信息失败")?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("读取账号信息失败({status}):{text}"));
    }
    let info: UserInfo = serde_json::from_str(&text).context("账号信息解析失败")?;
    Ok(info.email)
}

/// XOAUTH2 认证器:async-imap 要求实现 Authenticator trait;
/// XOAUTH2 无服务器挑战,首次即回初始响应。
struct Xoauth2 {
    user: String,
    token: String,
}

impl async_imap::Authenticator for Xoauth2 {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> String {
        oauth2::xoauth2_string(&self.user, &self.token)
    }
}

/// XOAUTH2 登录建立 IMAP 会话。
pub async fn connect_oauth(email: &str, access_token: &str) -> Result<ImapSession> {
    let client = qq::connect_imaps("imap.gmail.com", 993).await?;
    let auth = Xoauth2 { user: email.to_string(), token: access_token.to_string() };
    match client.authenticate("XOAUTH2", auth).await {
        Ok(s) => Ok(s),
        Err((e, _)) => Err(anyhow!(
            "Gmail XOAUTH2 登录失败:{e};令牌可能已过期,请在账号菜单重新授权"
        )),
    }
}

// ---------- access token 运行期缓存(进程内,不落盘) ----------

#[derive(Debug, Clone)]
struct CachedToken {
    access: String,
    expires_at: std::time::Instant,
}

static TOKEN_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, CachedToken>>> =
    std::sync::OnceLock::new();

fn token_cache() -> std::sync::MutexGuard<'static, std::collections::HashMap<String, CachedToken>> {
    TOKEN_CACHE
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn cache_get(email: &str) -> Option<String> {
    let map = token_cache();
    let t = map.get(email)?;
    if std::time::Instant::now() < t.expires_at {
        Some(t.access.clone())
    } else {
        None
    }
}

pub fn cache_put(email: &str, access: &str, ttl: std::time::Duration) {
    token_cache().insert(
        email.to_string(),
        CachedToken { access: access.to_string(), expires_at: std::time::Instant::now() + ttl },
    );
}

pub fn cache_clear(email: &str) {
    token_cache().remove(email);
}

/// 用 refresh token 换新 access token 并入缓存。
/// 若 Google 返回了新的 refresh_token(令牌轮换),一并返回供调用方写回凭证库。
pub async fn refresh_access(
    client_id: &str,
    email: &str,
    refresh_token: &str,
) -> Result<(String, Option<String>)> {
    let tokens = oauth2::refresh_token(TOKEN_ENDPOINT, client_id, refresh_token).await?;
    let ttl = std::time::Duration::from_secs(tokens.expires_in.unwrap_or(3600).saturating_sub(120));
    cache_put(email, &tokens.access_token, ttl);
    Ok((tokens.access_token, tokens.refresh_token))
}

/// 拿可用 access token:优先缓存,过期则用 refresh token 续期。
/// 返回 (access_token, Option<新 refresh_token>)。
pub async fn access_token(
    client_id: &str,
    email: &str,
    refresh_token: &str,
) -> Result<(String, Option<String>)> {
    if let Some(t) = cache_get(email) {
        return Ok((t, None));
    }
    refresh_access(client_id, email, refresh_token).await
}
