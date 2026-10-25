//! OAuth2 引擎(与具体服务商解耦):PKCE、令牌交换与刷新。
//! Gmail 的端点与 scope 在 provider::gmail 中配置。

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// RFC 7636 PKCE:code_verifier → S256 code_challenge
pub fn pkce_pair() -> (String, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = base64_url(&bytes);
    let challenge = base64_url(&sha256(verifier.as_bytes()));
    (verifier, challenge)
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

/// URL-safe base64(无填充),PKCE 规范
fn base64_url(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// RFC 4616 XOAUTH2 初始响应串,base64 编码
pub fn xoauth2_string(user: &str, access_token: &str) -> String {
    let raw = format!("user={user}\x01auth=Bearer {access_token}\x01\x01");
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
}

/// 授权码 + code_verifier 换令牌(返回体含 access/refresh)
pub async fn exchange_code(
    token_endpoint: &str,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(token_endpoint)
        .form(&[
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("请求令牌端点失败")?;
    parse_token(resp).await
}

/// refresh_token 静默续期
pub async fn refresh_token(
    token_endpoint: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(token_endpoint)
        .form(&[
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("刷新令牌失败")?;
    parse_token(resp).await
}

async fn parse_token(resp: reqwest::Response) -> Result<TokenResponse> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("令牌端点返回 {status}:{text}"));
    }
    serde_json::from_str::<TokenResponse>(&text).map_err(|e| anyhow!("令牌响应解析失败:{e}"))
}
