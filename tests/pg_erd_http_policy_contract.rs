use cwl_pingora_gateway::http_policy::{
    HeaderPolicyError, ResponseHeaderPolicy, ResponseHeaderRule,
};

fn pg_erd_cloud_security_headers() -> Vec<ResponseHeaderRule> {
    vec![
        ResponseHeaderRule {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        },
        ResponseHeaderRule {
            name: "X-Frame-Options".to_string(),
            value: "DENY".to_string(),
        },
        ResponseHeaderRule {
            name: "Referrer-Policy".to_string(),
            value: "no-referrer".to_string(),
        },
        ResponseHeaderRule {
            name: "Permissions-Policy".to_string(),
            value: "geolocation=(), microphone=(), camera=()".to_string(),
        },
    ]
}

#[test]
fn pg_erd_cloud_security_headers_preserve_live_traefik_contract() {
    let policy = ResponseHeaderPolicy::try_new(pg_erd_cloud_security_headers())
        .expect("live pg-erd-cloud security header contract is valid");

    assert_eq!(
        policy.value_for("x-content-type-options"),
        Some("nosniff")
    );
    assert_eq!(policy.value_for("X-Frame-Options"), Some("DENY"));
    assert_eq!(policy.value_for("referrer-policy"), Some("no-referrer"));
    assert_eq!(
        policy.value_for("permissions-policy"),
        Some("geolocation=(), microphone=(), camera=()")
    );
    assert_eq!(policy.value_for("server"), None);
    assert_eq!(policy.len(), 4);
    assert!(!policy.is_empty());
}

#[test]
fn header_lookup_is_ascii_case_insensitive() {
    let policy = ResponseHeaderPolicy::try_new(vec![ResponseHeaderRule {
        name: "X-Content-Type-Options".to_string(),
        value: "nosniff".to_string(),
    }])
    .expect("single response header is valid");

    assert_eq!(policy.value_for("X-CONTENT-TYPE-OPTIONS"), Some("nosniff"));
    assert_eq!(policy.value_for("x-content-type-options"), Some("nosniff"));
}

#[test]
fn duplicate_header_names_fail_closed_case_insensitively() {
    let error = ResponseHeaderPolicy::try_new(vec![
        ResponseHeaderRule {
            name: "X-Frame-Options".to_string(),
            value: "DENY".to_string(),
        },
        ResponseHeaderRule {
            name: "x-frame-options".to_string(),
            value: "SAMEORIGIN".to_string(),
        },
    ])
    .expect_err("duplicate response authority must fail closed");

    assert_eq!(
        error,
        HeaderPolicyError::DuplicateHeaderName {
            header_name: "x-frame-options".to_string(),
        }
    );
}

#[test]
fn malformed_header_authority_is_rejected_before_activation() {
    assert_eq!(
        ResponseHeaderPolicy::try_new(Vec::new()).expect_err("empty policy must fail"),
        HeaderPolicyError::NoHeaders
    );

    for (rule, expected) in [
        (
            ResponseHeaderRule {
                name: "".to_string(),
                value: "value".to_string(),
            },
            HeaderPolicyError::InvalidHeaderName {
                header_name: "".to_string(),
            },
        ),
        (
            ResponseHeaderRule {
                name: "Bad Header".to_string(),
                value: "value".to_string(),
            },
            HeaderPolicyError::InvalidHeaderName {
                header_name: "Bad Header".to_string(),
            },
        ),
        (
            ResponseHeaderRule {
                name: "X-Test".to_string(),
                value: "".to_string(),
            },
            HeaderPolicyError::EmptyHeaderValue {
                header_name: "X-Test".to_string(),
            },
        ),
        (
            ResponseHeaderRule {
                name: "X-Test".to_string(),
                value: "safe\rInjected: yes".to_string(),
            },
            HeaderPolicyError::InvalidHeaderValue {
                header_name: "X-Test".to_string(),
            },
        ),
        (
            ResponseHeaderRule {
                name: "X-Test".to_string(),
                value: "safe\nInjected: yes".to_string(),
            },
            HeaderPolicyError::InvalidHeaderValue {
                header_name: "X-Test".to_string(),
            },
        ),
    ] {
        assert_eq!(
            ResponseHeaderPolicy::try_new(vec![rule]).expect_err("invalid header must fail"),
            expected
        );
    }
}
