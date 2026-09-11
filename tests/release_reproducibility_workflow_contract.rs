const RELEASE_REPRODUCIBILITY_WORKFLOW: &str =
    include_str!("../.github/workflows/release-reproducibility.yml");

fn contains_cargo_toolchain_selector(source: &str) -> bool {
    let logical_source = source.replace("\\\r\n", "").replace("\\\n", "");

    logical_source.lines().any(|line| {
        let mut previous_was_cargo = false;
        for token in line.split(|character: char| {
            character.is_ascii_whitespace() || matches!(character, ';' | '&' | '|' | '(' | ')')
        }) {
            if token.is_empty() {
                continue;
            }
            if previous_was_cargo && token.starts_with('+') {
                return true;
            }
            previous_was_cargo = token == "cargo";
        }
        false
    })
}

#[test]
fn release_reproducibility_lane_builds_twice_with_the_release_compiler() {
    let install = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("rustup toolchain install 1.98.1 --profile minimal")
        .expect("release reproducibility workflow must install Rust 1.98.1");
    let select = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("rustup default 1.98.1")
        .expect("release reproducibility workflow must select Rust 1.98.1");
    let verify = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("rustc --version --verbose | grep -Fx 'release: 1.98.1'")
        .expect("release reproducibility workflow must verify Rust 1.98.1");
    let fetch = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("cargo fetch --locked")
        .expect("release reproducibility workflow must fetch the locked graph once");
    let first_build = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("CARGO_TARGET_DIR=\"/tmp/cwl-pingora-repro-a-${EXPECTED_SHA}\"")
        .expect("first release build must use an isolated target directory");
    let second_build = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("CARGO_TARGET_DIR=\"/tmp/cwl-pingora-repro-b-${EXPECTED_SHA}\"")
        .expect("second release build must use a distinct isolated target directory");

    assert!(install < select && select < verify && verify < fetch);
    assert!(fetch < first_build && first_build < second_build);
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("cargo build --release --locked")
            .count(),
        2
    );
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("--bin cwl-pingora-gateway")
            .count(),
        2
    );
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("--bin cwl-pingora-pg-erd-migration")
            .count(),
        2
    );
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("CARGO_NET_OFFLINE=true")
            .count(),
        2
    );
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("CARGO_INCREMENTAL=0")
            .count(),
        2
    );
}

#[test]
fn release_reproducibility_lane_fails_closed_on_compiler_or_artifact_drift() {
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${RUSTUP_TOOLCHAIN:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${RUSTFLAGS:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${CARGO_ENCODED_RUSTFLAGS:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${RUSTC_WRAPPER:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${RUSTC_WORKSPACE_WRAPPER:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test ! -e rust-toolchain"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test ! -e rust-toolchain.toml"));
    assert!(!RELEASE_REPRODUCIBILITY_WORKFLOW.contains("1.98.0"));
    assert!(!contains_cargo_toolchain_selector(
        RELEASE_REPRODUCIBILITY_WORKFLOW
    ));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains(
        "if [[ \"$digest_a\" != \"$digest_b\" ]] || ! cmp --silent \"$candidate_a\" \"$candidate_b\"; then"
    ));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("printf 'result=byte-mismatch\\n'"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("exit 1"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("printf 'result=byte-identical\\n'"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("evidence_kind=unreleased-same-platform-release-binary-reproducibility"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("name: release-reproducibility-${{ env.EXPECTED_SHA }}"));
}

#[test]
fn byte_mismatch_keeps_bounded_root_cause_evidence_without_weakening_the_gate() {
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("cmp -l \"$candidate_a\" \"$candidate_b\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("head -n 256"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("readelf -n \"$candidate_a\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("readelf -n \"$candidate_b\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("strings -a \"$candidate_a\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("strings -a \"$candidate_b\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("head -n 512"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("if: ${{ failure() }}"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("name: release-reproducibility-diagnostics-${{ env.EXPECTED_SHA }}"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("retention-days: 7"));
}

#[test]
fn cargo_toolchain_selector_detection_covers_shell_layout_variants() {
    assert!(contains_cargo_toolchain_selector("cargo +1.98.0 build"));
    assert!(contains_cargo_toolchain_selector("cargo  +1.98.0 build"));
    assert!(contains_cargo_toolchain_selector("cargo\t+nightly test"));
    assert!(contains_cargo_toolchain_selector(
        "cargo build;cargo +nightly test"
    ));
    assert!(contains_cargo_toolchain_selector(
        "cargo build&&cargo +1.98.0 test"
    ));
    assert!(contains_cargo_toolchain_selector(
        "cargo \\\n  +nightly test"
    ));
    assert!(!contains_cargo_toolchain_selector("cargo build --release"));
}
