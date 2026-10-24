//! 加密原语测试:密钥派生、AES-GCM 往返、防篡改、机器标识稳定性。
//! 运行:`cargo test --test crypto`

use maildeck_lib::crypto;

#[test]
fn machine_uuid_is_nonempty_and_stable() {
    let a = crypto::machine_uuid();
    let b = crypto::machine_uuid();
    assert!(!a.is_empty());
    assert_eq!(a, b, "同一台机器两次读取应一致");
}

#[test]
fn derive_key_is_deterministic_and_purpose_separated() {
    let k1 = crypto::derive_key("sqlcipher");
    let k2 = crypto::derive_key("sqlcipher");
    let k3 = crypto::derive_key("credentials-v1");
    assert_eq!(k1, k2, "同用途派生应确定");
    assert_ne!(k1, k3, "不同用途盐应产生不同密钥");
}

#[test]
fn aes_gcm_roundtrip() {
    let data = "授权码-secret-123".as_bytes();
    let sealed = crypto::encrypt_b64(data);
    assert_ne!(sealed.as_bytes(), data);
    assert_eq!(crypto::decrypt_b64(&sealed).unwrap(), data);
}

#[test]
fn tampered_ciphertext_fails_closed() {
    let sealed = crypto::encrypt_b64(b"hello");
    let mut bytes = sealed.into_bytes();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    assert!(crypto::decrypt_b64(&String::from_utf8(bytes).unwrap()).is_none());
}

#[test]
fn garbage_input_fails_closed() {
    assert!(crypto::decrypt_b64("not-base64!!!").is_none());
    assert!(crypto::decrypt_b64("QUJD").is_none()); // "ABC" 太短
}
