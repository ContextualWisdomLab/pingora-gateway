//! Executable characterization of supplier semantics that must survive dependency repair.

use pingora::upstreams::peer::PeerOptions;

/// Returns whether the Debug output contains the requested field marker.
fn debug_has_field(debug: &str, field: &str) -> bool {
    let marker = format!("{field}:");
    debug.contains(marker.as_str())
}

/// Proves the supplier-field oracle distinguishes overlapping field names.
#[test]
fn debug_field_match_requires_a_structural_field_boundary() {
    assert!(
        !debug_has_field(
            "PeerOptions { total_connection_timeout: None }",
            "connection_timeout"
        ),
        "connection_timeout must not be satisfied by total_connection_timeout"
    );
}

/// Preserves the public Debug boundary before replacing `derivative` upstream.
#[test]
fn peer_options_debug_keeps_safe_fields_and_omits_hook_fields() {
    let debug = format!("{:?}", PeerOptions::new());

    assert!(
        debug.starts_with("PeerOptions {"),
        "PeerOptions must remain structurally debuggable after supplier macro removal: {debug}"
    );

    for visible_field in [
        "bind_to",
        "connection_timeout",
        "total_connection_timeout",
        "read_timeout",
        "idle_timeout",
        "write_timeout",
        "verify_cert",
        "verify_hostname",
        "alternative_cn",
        "alpn",
        "ca",
        "tcp_keepalive",
        "tcp_recv_buf",
        "dscp",
        "h2_ping_interval",
        "max_h2_streams",
        "h2_stream_window_size",
        "h2_connection_window_size",
        "allow_h1_response_invalid_content_length",
        "http_upstream_request_policy",
        "extra_proxy_headers",
        "curves",
        "second_keyshare",
        "tcp_fast_open",
        "tracer",
        "custom_l4",
    ] {
        assert!(
            debug_has_field(&debug, visible_field),
            "supplier Debug output must retain the current non-hook field {visible_field}: {debug}"
        );
    }

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
