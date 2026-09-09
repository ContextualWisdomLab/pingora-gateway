use std::fs;

const COMPILE_AND_TEST_STEP: &str = "      - name: Compile and test\n        run: cargo test --all-targets --locked --no-fail-fast\n";
const FAIL_FAST_COMPILE_AND_TEST_STEP: &str =
    "      - name: Compile and test\n        run: cargo test --all-targets --locked\n";

/// Proves the canonical CI test step keeps target-level evidence after an earlier test binary fails.
#[test]
fn ci_runs_every_test_binary_before_failing_the_gate() {
    let workflow =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should be readable");

    assert_eq!(
        workflow.matches("      - name: Compile and test\n").count(),
        1,
        "CI should have one canonical compile-and-test step"
    );
    assert!(
        workflow.contains(COMPILE_AND_TEST_STEP),
        "CI must execute later test binaries even when an earlier supplier RED fails"
    );
    assert!(
        !workflow.contains(FAIL_FAST_COMPILE_AND_TEST_STEP),
        "CI must not regress the canonical compile-and-test step to target-level fail-fast"
    );
}
