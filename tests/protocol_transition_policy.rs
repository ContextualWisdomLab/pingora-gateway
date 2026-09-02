use cwl_pingora_gateway::protocol_transition_policy::requests_http1_protocol_transition;

#[test]
fn ordinary_http_without_upgrade_evidence_is_admitted() {
    assert!(!requests_http1_protocol_transition(
        false,
        [b"keep-alive".as_slice()]
    ));
}

#[test]
fn upgrade_field_is_rejected_even_without_connection_token() {
    assert!(requests_http1_protocol_transition(
        true,
        std::iter::empty::<&[u8]>()
    ));
}

#[test]
fn connection_upgrade_token_is_case_insensitive_and_comma_delimited() {
    assert!(requests_http1_protocol_transition(
        false,
        [b"keep-alive, UpGrAdE".as_slice()]
    ));
}

#[test]
fn any_connection_field_value_can_signal_upgrade() {
    assert!(requests_http1_protocol_transition(
        false,
        [b"keep-alive".as_slice(), b" upgrade ".as_slice()]
    ));
}

#[test]
fn unrelated_connection_tokens_do_not_false_positive() {
    assert!(!requests_http1_protocol_transition(
        false,
        [b"keep-alive, x-upgrade".as_slice()]
    ));
}
