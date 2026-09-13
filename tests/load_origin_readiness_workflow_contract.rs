//! Regression contract for measured-origin readiness in the load workflow.
//!
//! The gateway health endpoint proves only the proxy process is accepting traffic. It does not
//! prove the measured upstream fixture has bound its socket. The load harness must therefore prove
//! origin readiness directly before starting the gateway candidate or k6 measurement.

use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";

#[test]
fn load_contract_proves_origin_readiness_before_gateway_measurement() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    let origin_start = source
        .find("/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &")
        .expect("load contract should start the bounded Rust upstream fixture");
    let origin_ready = source
        .find("http://127.0.0.1:18081/fixture-ready")
        .expect("load contract must probe the measured origin directly before gateway traffic");
    let origin_liveness = source
        .find("kill -0 \"$upstream_pid\"")
        .expect("load contract must fail when the origin process exits before readiness");
    let gateway_start = source
        .find("target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml")
        .expect("load contract should start the exact gateway candidate");
    let measured_traffic = source
        .find("GATEWAY_URL=http://127.0.0.1:18080 k6 run")
        .expect("load contract should execute measured k6 traffic");

    assert!(
        origin_start < origin_ready
            && origin_ready < origin_liveness
            && origin_liveness < gateway_start
            && gateway_start < measured_traffic,
        "origin readiness and liveness must be established before gateway startup and measured traffic"
    );
}
