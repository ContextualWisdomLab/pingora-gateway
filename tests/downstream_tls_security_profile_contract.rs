//! Structural contract for the versioned downstream TLS security profile.
//!
//! The edge must not inherit protocol-version or cipher policy only from Pingora/OpenSSL helper
//! defaults because those defaults are supplier implementation detail rather than CWL release
//! authority. Real-wire protocol acceptance remains covered separately.

#[test]
fn downstream_tls_delivery_pins_protocol_versions_and_cipher_policy() {
    let source = include_str!("../src/tls_delivery.rs");

    for required in [
        "DOWNSTREAM_TLS12_CIPHER_LIST",
        "DOWNSTREAM_TLS13_CIPHERSUITES",
        "set_min_proto_version(Some(SslVersion::TLS1_2))",
        "set_max_proto_version(Some(SslVersion::TLS1_3))",
        "set_cipher_list(DOWNSTREAM_TLS12_CIPHER_LIST)",
        "set_ciphersuites(DOWNSTREAM_TLS13_CIPHERSUITES)",
    ] {
        assert!(
            source.contains(required),
            "downstream TLS delivery must explicitly enforce security-profile fragment {required:?}"
        );
    }
}

#[test]
fn tls12_profile_admits_only_ephemeral_ecdhe_aead_suites() {
    let source = include_str!("../src/tls_delivery.rs");
    let declaration = source
        .split("const DOWNSTREAM_TLS12_CIPHER_LIST: &str = ")
        .nth(1)
        .and_then(|tail| tail.split(';').next())
        .expect("TLS 1.2 cipher policy must have one explicit declaration");

    assert!(declaration.contains("ECDHE-RSA-AES128-GCM-SHA256"));
    assert!(declaration.contains("ECDHE-ECDSA-AES128-GCM-SHA256"));

    let normalized: String = declaration
        .chars()
        .filter(|character| !matches!(character, '"' | '\\' | '\n' | ' '))
        .collect();
    let suites: Vec<_> = normalized.split(':').filter(|suite| !suite.is_empty()).collect();
    assert_eq!(suites.len(), 6, "TLS 1.2 profile cardinality changed unexpectedly");

    for suite in suites {
        assert!(
            suite.starts_with("ECDHE-RSA-") || suite.starts_with("ECDHE-ECDSA-"),
            "TLS 1.2 suite must use ephemeral ECDHE authentication: {suite}"
        );
        assert!(
            suite.contains("-GCM-") || suite.contains("-CHACHA20-POLY1305"),
            "TLS 1.2 suite must use an admitted AEAD construction: {suite}"
        );
    }
}
