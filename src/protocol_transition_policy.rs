//! Transport-neutral admission policy for connection-wide HTTP protocol transitions.
//!
//! Version 1 does not admit HTTP/1 Upgrade semantics. Consumer-specific WebSocket or other
//! protocol-transition behavior requires a separate versioned contract and realistic traffic
//! evidence before a Pingora delivery adapter may activate it.

/// Returns `true` when an HTTP/1 request attempts a connection-wide protocol transition.
///
/// An `Upgrade` field is sufficient evidence of an attempt. The `Connection` field is also
/// inspected as a comma-delimited, case-insensitive token list so malformed or partial upgrade
/// requests cannot bypass the fail-closed boundary. Unrelated tokens such as `x-upgrade` do not
/// match.
pub fn requests_http1_protocol_transition<'a>(
    upgrade_field_present: bool,
    connection_field_values: impl IntoIterator<Item = &'a [u8]>,
) -> bool {
    upgrade_field_present
        || connection_field_values.into_iter().any(|field_value| {
            field_value.split(|byte| *byte == b',').any(|token| {
                token
                    .trim_ascii()
                    .eq_ignore_ascii_case(b"upgrade")
            })
        })
}
