//! 静态加密的密钥派生与对称加解密。
//! 密钥由「机器唯一标识 + 用途盐」派生:防止文件被直接拷贝/同步/翻看;
//! 局限(如实告知):无法对抗能以当前用户身份运行的恶意程序。
//! 数据库密钥(SQLCipher 原始密钥)与凭证文件密钥(AES-256-GCM)用途隔离。

use sha2::{Digest, Sha256};

const FALLBACK_MACHINE_ID: &str = "maildeck-fallback-machine-id";

/// 跨平台机器唯一标识。
pub fn machine_uuid() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(v) = line.split('"').nth(3) {
                        return v.to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("MachineGuid") {
                    if let Some(v) = line.split_whitespace().last() {
                        return v.to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        for p in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            if let Ok(id) = std::fs::read_to_string(p) {
                let id = id.trim().to_string();
                if !id.is_empty() {
                    return id;
                }
            }
        }
    }
    FALLBACK_MACHINE_ID.to_string()
}

/// 按用途派生 32 字节密钥:SHA-256(机器标识 || 用途盐)
pub fn derive_key(purpose: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(machine_uuid().as_bytes());
    hasher.update(b"::maildeck::");
    hasher.update(purpose.as_bytes());
    hasher.finalize().into()
}

/// SQLCipher 原始密钥的 PRAGMA 片段:PRAGMA key = "x'<128 hex>'"
pub fn sqlcipher_key_pragma() -> String {
    let key = derive_key("sqlcipher");
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    format!("PRAGMA key = \"x'{hex}'\";")
}

const CREDS_KEY_PURPOSE: &str = "credentials-v1";

/// AES-256-GCM 加密,输出 base64(nonce ‖ ciphertext)
pub fn encrypt_b64(plaintext: &[u8]) -> String {
    use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
    let key = derive_key(CREDS_KEY_PURPOSE);
    let cipher = aes_gcm::Aes256Gcm::new((&key).into());
    let nonce = aes_gcm::Aes256Gcm::generate_nonce(&mut aes_gcm::aead::OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .expect("AES-GCM 加密不会失败(密钥长度固定)");
    let mut packed = nonce.to_vec();
    packed.extend_from_slice(&ct);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(packed)
}

/// 解密 encrypt_b64 的输出;损坏/密钥不匹配返回 None
pub fn decrypt_b64(data: &str) -> Option<Vec<u8>> {
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;
    let packed = base64::engine::general_purpose::STANDARD.decode(data).ok()?;
    if packed.len() < 12 {
        return None;
    }
    let (nonce, ct) = packed.split_at(12);
    let key = derive_key(CREDS_KEY_PURPOSE);
    let cipher = aes_gcm::Aes256Gcm::new((&key).into());
    cipher.decrypt(nonce.into(), ct).ok()
}

// ---------- 凭证文件的用户口令模式 ----------
// 密钥由用户口令 + 随机盐经 PBKDF2 派生;本地只存盐与验证块(加密的全零),
// 不存密钥本身。文件离开本机且没有口令时不可解密。

const PBKDF2_ITERATIONS: u32 = 210_000;

pub fn random_salt() -> [u8; 16] {
    use aes_gcm::aead::OsRng;
    use rand::RngCore;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// PBKDF2-HMAC-SHA256(口令, 盐) → 32 字节密钥
pub fn derive_key_from_passphrase(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
        passphrase.as_bytes(),
        salt,
        PBKDF2_ITERATIONS,
        &mut out,
    );
    out
}

/// 验证块:用派生密钥加密 16 字节全零(nonce ‖ 密文)。本地只存它,不存密钥。
pub fn make_verifier(passphrase: &str, salt: &[u8]) -> Vec<u8> {
    encrypt_with_key(&derive_key_from_passphrase(passphrase, salt), &[0u8; 16])
}

/// 校验口令:能解出全零即正确。
pub fn verify_passphrase(passphrase: &str, salt: &[u8], verifier: &[u8]) -> bool {
    decrypt_with_key(&derive_key_from_passphrase(passphrase, salt), verifier)
        .map(|p| p == [0u8; 16])
        .unwrap_or(false)
}

fn encrypt_with_key(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
    let cipher = aes_gcm::Aes256Gcm::new(key.into());
    let nonce = aes_gcm::Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher.encrypt(&nonce, plaintext).expect("AES-GCM 加密不会失败");
    let mut packed = nonce.to_vec();
    packed.extend_from_slice(&ct);
    packed
}

fn decrypt_with_key(key: &[u8; 32], packed: &[u8]) -> Option<Vec<u8>> {
    use aes_gcm::aead::{Aead, KeyInit};
    if packed.len() < 12 {
        return None;
    }
    let (nonce, ct) = packed.split_at(12);
    let cipher = aes_gcm::Aes256Gcm::new(key.into());
    cipher.decrypt(nonce.into(), ct).ok()
}

/// 用口令派生密钥加密,输出 base64(nonce ‖ 密文)——凭证文件体
pub fn encrypt_b64_with_passphrase(passphrase: &str, salt: &[u8], plaintext: &[u8]) -> String {
    use base64::Engine;
    let key = derive_key_from_passphrase(passphrase, salt);
    base64::engine::general_purpose::STANDARD.encode(encrypt_with_key(&key, plaintext))
}

/// 解密上一函数的输出;口令错误/损坏返回 None
pub fn decrypt_b64_with_passphrase(passphrase: &str, salt: &[u8], data: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let packed = base64::engine::general_purpose::STANDARD.decode(data).ok()?;
    let key = derive_key_from_passphrase(passphrase, salt);
    decrypt_with_key(&key, &packed)
}
