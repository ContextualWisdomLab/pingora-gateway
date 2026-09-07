//! Regression contract: measured gateway latency must not include a Python origin.
//!
//! The load fixture is inside the measured request path, so using Python here would
//! contaminate gateway latency attribution even though Python remains acceptable for
//! non-hot-path tooling when no practical Rust alternative exists.

use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";

#[test]
fn measured_load_paths_use_the_bounded_rust_origin() {
    let workflow = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");

    assert!(
        workflow.contains("rustfmt --edition 2021 --check tests/load/load_origin.rs"),
        "load lane must format-check the Rust origin before measured traffic"
    );
    assert!(
        workflow.contains("rustc --edition 2021 -D warnings --test tests/load/load_origin.rs"),
        "load lane must run the Rust origin's direct fixture contract"
    );
    assert!(
        workflow.contains("rustc --edition 2021 -D warnings -C opt-level=3 -C debuginfo=0"),
        "measured origin must be optimized Rust rather than an interpreter fixture"
    );
    assert!(
        !workflow.contains("python3 tests/load/upstream_fixture.py"),
        "Python must not sit inside generic or pg-erd measured latency paths"
    );
    assert!(
        workflow.matches("/tmp/load_origin").count() >= 3,
        "generic plus backend/frontend pg-erd origins must all use the same bounded Rust fixture"
    );
}
