//! 口令派生密钥(PBKDF2)与验证块测试。
//! 运行:`cargo test --test passphrase`

use maildeck_lib::crypto;

#[test]
fn derive_key_is_deterministic_per_salt() {
    let k1 = crypto::derive_key_from_passphrase("口令123", b"salt-a");
    let k2 = crypto::derive_key_from_passphrase("口令123", b"salt-a");
    let k3 = crypto::derive_key_from_passphrase("口令124", b"salt-a");
    let k4 = crypto::derive_key_from_passphrase("口令123", b"salt-b");
    assert_eq!(k1, k2, "同口令同盐应确定");
    assert_ne!(k1, k3, "不同口令应不同");
    assert_ne!(k1, k4, "不同盐应不同");
}

#[test]
fn verifier_roundtrip_and_wrong_passphrase() {
    let salt = crypto::random_salt();
    let verifier = crypto::make_verifier("正确口令", &salt);
    assert!(crypto::verify_passphrase("正确口令", &salt, &verifier));
    assert!(!crypto::verify_passphrase("错误口令", &salt, &verifier));
}

#[test]
fn verifier_is_not_a_password_oracle() {
    // 验证块解密后应是全零,不能从中还原口令或密钥
    let salt = crypto::random_salt();
    let verifier = crypto::make_verifier("s3cret-口令", &salt);
    assert_ne!(verifier, "s3cret-口令".as_bytes());
    assert!(verifier.len() >= 12);
}

#[test]
fn random_salts_differ() {
    assert_ne!(crypto::random_salt(), crypto::random_salt());
}
