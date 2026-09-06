//! Executable characterization of supplier semantics that must survive dependency repair.

use pingora::upstreams::peer::PeerOptions;

/// Preserves the public Debug boundary before replacing `derivative` upstream.
#[test]
fn peer_options_debug_keeps_safe_fields_and_omits_hook_fields() {
    let debug = format!("{:?}", PeerOptions::new());

    assert!(
        debug.starts_with("PeerOptions {"),
        "PeerOptions must remain structurally debuggable after supplier macro removal: {debug}"
    );
    assert!(
        debug.contains("connection_timeout: None"),
        "a normal safe field must remain visible so hook omission cannot pass vacuously: {debug}"
    );

    for omitted_hook in [
        "upstream_tcp_sock_tweak_hook",
        "proxy_digest_user_data_hook",
        "upstream_tls_handshake_complete_hook",
    ] {
        assert!(
            !debug.contains(omitted_hook),
            "supplier Debug output must continue to omit callback/hook field {omitted_hook}: {debug}"
        );
    }
}
