//! Regression contract for application-level readiness in the measured load lane.
//!
//! A live TCP socket or `/livez` response is insufficient release evidence for the load candidate.
//! The workflow must wait for the versioned `/readyz` response, require exact HTTP 200, and prove
//! its non-cacheable header before measured traffic begins.

use std::fs;

#[test]
fn load_contract_uses_exact_readyz_and_non_cacheable_readiness() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow should be readable UTF-8");
    let load_start = workflow
        .find("  load-contract:\n")
        .expect("CI workflow should define the load-contract job");
    let load_tail = &workflow[load_start..];
    let load_end = load_tail
        .find("\n  oci-runtime:\n")
        .expect("load-contract should end before oci-runtime");
    let load_contract = &load_tail[..load_end];

    assert!(
        !load_contract.contains("/livez"),
        "load-contract must not use process-liveness as application readiness"
    );
    assert_eq!(
        load_contract.matches("/readyz").count(),
        2,
        "load-contract must check /readyz in both the bounded startup loop and final validation"
    );
    assert_eq!(
        load_contract.matches("--write-out '%{http_code}'").count(),
        2,
        "both readiness checks must capture the exact HTTP status"
    );
    assert_eq!(
        load_contract
            .matches("test \"$ready_status\" = \"200\"")
            .count(),
        2,
        "both readiness checks must require exact HTTP 200"
    );
    assert_eq!(
        load_contract
            .matches("--dump-header /tmp/gateway-ready.headers")
            .count(),
        2,
        "both readiness checks must capture response headers"
    );
    assert!(
        !load_contract.contains("no-store\\r?$"),
        "GNU grep ERE does not interpret \\r as a carriage return; the readiness contract must not reject valid curl CRLF headers"
    );
    assert_eq!(
        load_contract
            .matches(
                "grep -Eiq '^cache-control:[[:space:]]*no-store[[:space:]]*$' /tmp/gateway-ready.headers"
            )
            .count(),
        2,
        "both readiness checks must accept the CRLF line ending emitted by curl while proving Cache-Control: no-store"
    );
}
