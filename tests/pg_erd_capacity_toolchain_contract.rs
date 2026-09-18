const CAPACITY_WORKFLOW: &str = include_str!("../.github/workflows/pg-erd-capacity.yml");

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
fn capacity_lane_uses_fixed_release_compiler_before_building_candidate() {
    let install = CAPACITY_WORKFLOW
        .find("rustup toolchain install 1.98.1 --profile minimal --component rustfmt")
        .expect("capacity workflow must install Rust 1.98.1");
    let select = CAPACITY_WORKFLOW
        .find("rustup default 1.98.1")
        .expect("capacity workflow must select Rust 1.98.1");
    let verify = CAPACITY_WORKFLOW
        .find("rustc --version --verbose | grep -Fx 'release: 1.98.1'")
        .expect("capacity workflow must verify the selected Rust 1.98.1 compiler");
    let build = CAPACITY_WORKFLOW
        .find("cargo build --release --locked --bin cwl-pingora-pg-erd-migration")
        .expect("capacity workflow must build the exact pg-erd release candidate");

    assert!(install < select && select < verify && verify < build);
    assert_eq!(
        CAPACITY_WORKFLOW
            .matches("rustup toolchain install ")
            .count(),
        1
    );
    assert_eq!(CAPACITY_WORKFLOW.matches("rustup default ").count(), 1);
    assert!(!CAPACITY_WORKFLOW.contains("rustup override"));
    assert!(!CAPACITY_WORKFLOW.contains("1.98.0"));
    assert!(!CAPACITY_WORKFLOW.contains("RUSTUP_TOOLCHAIN"));
    assert!(!contains_cargo_toolchain_selector(CAPACITY_WORKFLOW));
}

#[test]
fn cargo_toolchain_selector_detection_covers_shell_layout_variants() {
    assert!(contains_cargo_toolchain_selector("cargo +1.98.0 build"));
    assert!(contains_cargo_toolchain_selector("cargo  +1.98.0 build"));
    assert!(contains_cargo_toolchain_selector("  cargo\t+nightly test"));
    assert!(contains_cargo_toolchain_selector(
        "cargo build; cargo +nightly test"
    ));
    assert!(contains_cargo_toolchain_selector(
        "cargo build && cargo +1.98.0 test"
    ));
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
