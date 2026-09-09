use std::fs;

#[test]
fn ci_runs_every_test_binary_before_failing_the_gate() {
    let workflow =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should be readable");

    assert!(
        workflow.contains("cargo test --all-targets --locked --no-fail-fast"),
        "CI must execute later test binaries even when an earlier supplier RED fails"
    );
}
