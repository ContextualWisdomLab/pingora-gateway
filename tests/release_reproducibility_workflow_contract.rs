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

fn candidate_step_has_clean_build_stage_order(step: &str, staging_path: &str) -> bool {
    let clean = step.find("rm -rf \"$REPRO_TARGET_DIR\"");
    let build = step.find("cargo build --release --locked");
    let stage = step.find(staging_path);

    matches!((clean, build, stage), (Some(clean), Some(build), Some(stage)) if clean < build && build < stage)
}

#[test]
fn release_reproducibility_lane_rebuilds_cleanly_with_one_canonical_environment() {
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
    let source_date = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("source_date_epoch=\"$(git show -s --format=%ct \"$EXPECTED_SHA\")\"")
        .expect(
            "release reproducibility workflow must derive SOURCE_DATE_EPOCH from the exact source",
        );
    let canonical_target = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("REPRO_TARGET_DIR=/tmp/cwl-pingora-repro-%s")
        .expect("release reproducibility workflow must bind one canonical target path");
    let first_build = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Build isolated release candidate A")
        .expect("first clean release build must exist");
    let second_build = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Build isolated release candidate B")
        .expect("second clean release build must exist");
    let compare = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Compare release binaries byte for byte")
        .expect("release binary comparison must exist");

    assert!(install < select && select < verify && verify < fetch);
    assert!(fetch < source_date && source_date < canonical_target);
    assert!(canonical_target < first_build && first_build < second_build && second_build < compare);

    let candidate_a = &RELEASE_REPRODUCIBILITY_WORKFLOW[first_build..second_build];
    let candidate_b = &RELEASE_REPRODUCIBILITY_WORKFLOW[second_build..compare];
    assert!(candidate_step_has_clean_build_stage_order(
        candidate_a,
        "release-reproducibility-candidates/a/cwl-pingora-gateway"
    ));
    assert!(candidate_step_has_clean_build_stage_order(
        candidate_b,
        "release-reproducibility-candidates/b/cwl-pingora-gateway"
    ));

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
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("CARGO_TARGET_DIR=\"$REPRO_TARGET_DIR\"")
            .count(),
        2
    );
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("rm -rf \"$REPRO_TARGET_DIR\"")
            .count(),
        2
    );
    assert!(
        RELEASE_REPRODUCIBILITY_WORKFLOW.contains("[[ \"$source_date_epoch\" =~ ^[1-9][0-9]*$ ]]")
    );
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("printf 'SOURCE_DATE_EPOCH=%s\\n' \"$source_date_epoch\" >> \"$GITHUB_ENV\""));
}

#[test]
fn clean_rebuild_order_contract_rejects_misplaced_cleanup() {
    let malformed_candidate = r#"
      rm -rf "$REPRO_TARGET_DIR"
      rm -rf "$REPRO_TARGET_DIR"
      cargo build --release --locked
      cp "$REPRO_TARGET_DIR/release/cwl-pingora-gateway" release-reproducibility-candidates/a/cwl-pingora-gateway
    "#;

    assert!(candidate_step_has_clean_build_stage_order(
        malformed_candidate,
        "release-reproducibility-candidates/a/cwl-pingora-gateway"
    ));
    assert!(!candidate_step_has_clean_build_stage_order(
        "cargo build --release --locked\nrm -rf \"$REPRO_TARGET_DIR\"\nrelease-reproducibility-candidates/b/cwl-pingora-gateway",
        "release-reproducibility-candidates/b/cwl-pingora-gateway"
    ));
}

#[test]
fn release_reproducibility_lane_fails_closed_on_compiler_or_artifact_drift() {
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\""));

    for forbidden in [
        "RUSTC",
        "CARGO_BUILD_RUSTC",
        "RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "RUSTUP_TOOLCHAIN",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        let guard = format!("test -z \"${{{forbidden}:-}}\"");
        assert!(
            RELEASE_REPRODUCIBILITY_WORKFLOW.contains(&guard),
            "release reproducibility workflow must reject compiler/build authority {forbidden}"
        );
    }

    for path in [".cargo/config", ".cargo/config.toml"] {
        let guard = format!("test ! -e {path}");
        assert!(
            RELEASE_REPRODUCIBILITY_WORKFLOW.contains(&guard),
            "release reproducibility workflow must reject repository Cargo compiler config {path}"
        );
    }

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
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("printf 'source_date_epoch=%s\\n' \"$SOURCE_DATE_EPOCH\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("printf 'canonical_target_dir=%s\\n' \"$REPRO_TARGET_DIR\""));
}

#[test]
fn release_reproducibility_lane_isolates_cargo_configuration_hierarchy() {
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("test -z \"${CARGO_HOME:-}\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("cargo_config_dir=\"$PWD\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("test ! -e \"$cargo_config_dir/.cargo/config\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("test ! -e \"$cargo_config_dir/.cargo/config.toml\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("cargo_config_dir=\"$(dirname \"$cargo_config_dir\")\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("[[ \"$cargo_config_dir\" == \"/\" ]] && break"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("controlled_cargo_home=\"$RUNNER_TEMP/cwl-cargo-home-$EXPECTED_SHA\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("rm -rf \"$controlled_cargo_home\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("mkdir -p \"$controlled_cargo_home\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains(
        "printf 'CARGO_HOME=%s\\n' \"$controlled_cargo_home\" >> \"$GITHUB_ENV\""
    ));
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
