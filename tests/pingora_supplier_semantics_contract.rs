//! Executable characterization of supplier semantics that must survive dependency repair.

use pingora::upstreams::peer::PeerOptions;

/// Returns whether a byte position is at a top-level field of the outer Debug struct.
fn is_top_level_debug_field_position(debug: &str, byte_index: usize) -> bool {
    let mut brace_depth = 0_usize;
    let mut bracket_depth = 0_usize;
    let mut parenthesis_depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;

    for ch in debug[..byte_index].chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => parenthesis_depth += 1,
            ')' => parenthesis_depth = parenthesis_depth.saturating_sub(1),
            _ => {}
        }
    }

    quote.is_none() && brace_depth == 1 && bracket_depth == 0 && parenthesis_depth == 0
}

/// Returns whether the Debug output contains the requested top-level structural field name.
fn debug_has_field(debug: &str, field: &str) -> bool {
    debug.match_indices(field).any(|(start, matched)| {
        if !is_top_level_debug_field_position(debug, start) {
            return false;
        }

        let preceding = debug[..start].chars().rev().find(|ch| !ch.is_whitespace());
        let following = debug[start + matched.len()..]
            .chars()
            .find(|ch| !ch.is_whitespace());

        matches!(preceding, Some('{') | Some(',')) && following == Some(':')
    })
}

/// Returns whether a requested scalar field is structurally present.
///
/// RED: field presence alone is intentionally insufficient; the value-sensitive
/// contract below demonstrates the missing acceptance behavior before repair.
fn debug_has_scalar_field_value(debug: &str, field: &str, _expected: &str) -> bool {
    debug_has_field(debug, field)
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

/// Proves nested values cannot impersonate a top-level `PeerOptions` field.
#[test]
fn debug_field_match_rejects_nested_and_quoted_field_like_text() {
    assert!(
        !debug_has_field(
            "PeerOptions { bind_to: Nested { connection_timeout: None } }",
            "connection_timeout"
        ),
        "a nested struct field must not satisfy a top-level PeerOptions field"
    );
    assert!(
        !debug_has_field(
            "PeerOptions { bind_to: \"Nested { connection_timeout: None }\" }",
            "connection_timeout"
        ),
        "field-like text inside a quoted Debug value must not satisfy a top-level PeerOptions field"
    );
}

/// Proves semantic field matching does not depend on incidental Debug whitespace.
#[test]
fn debug_field_match_accepts_equivalent_debug_whitespace() {
    assert!(
        debug_has_field(
            "PeerOptions{connection_timeout: None}",
            "connection_timeout"
        ),
        "a field immediately after the opening brace must remain visible"
    );
    assert!(
        debug_has_field(
            "PeerOptions {\n    connection_timeout: None,\n    read_timeout: None\n}",
            "connection_timeout"
        ),
        "pretty or manually formatted Debug output must not change field-presence semantics"
    );
}

/// Proves field-name preservation cannot hide a stale or fabricated scalar value.
#[test]
fn debug_field_match_requires_scalar_value_identity() {
    assert!(debug_has_scalar_field_value(
        "PeerOptions { verify_cert: false }",
        "verify_cert",
        "false"
    ));
    assert!(
        !debug_has_scalar_field_value(
            "PeerOptions { verify_cert: true }",
            "verify_cert",
            "false"
        ),
        "a present field with the wrong value must not satisfy Debug semantics preservation"
    );
}

/// Preserves representative configured scalar values, not only the Debug field vocabulary.
#[test]
fn peer_options_debug_reflects_configured_scalar_values() {
    let mut options = PeerOptions::new();
    options.verify_cert = false;
    options.verify_hostname = false;
    options.max_h2_streams = 17;
    options.allow_h1_response_invalid_content_length = true;
    options.second_keyshare = false;
    options.tcp_fast_open = true;
    let debug = format!("{options:?}");

    for (field, expected) in [
        ("verify_cert", "false"),
        ("verify_hostname", "false"),
        ("max_h2_streams", "17"),
        ("allow_h1_response_invalid_content_length", "true"),
        ("second_keyshare", "false"),
        ("tcp_fast_open", "true"),
    ] {
        assert!(
            debug_has_scalar_field_value(&debug, field, expected),
            "supplier Debug output must reflect configured {field}={expected}: {debug}"
        );
    }
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
