const TLS_H2_PERFORMANCE_WORKFLOW: &str =
    include_str!("../.github/workflows/tls-h2-performance.yml");

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
fn tls_h2_performance_lane_uses_release_compiler_before_building_candidate() {
    let install = TLS_H2_PERFORMANCE_WORKFLOW
        .find("rustup toolchain install 1.98.1 --profile minimal --component rustfmt")
        .expect("TLS/H2 performance workflow must install release Rust 1.98.1");
    let select = TLS_H2_PERFORMANCE_WORKFLOW
        .find("rustup default 1.98.1")
        .expect("TLS/H2 performance workflow must select release Rust 1.98.1");
    let verify = TLS_H2_PERFORMANCE_WORKFLOW
        .find("rustc --version --verbose | grep -Fx 'release: 1.98.1'")
        .expect("TLS/H2 performance workflow must verify release Rust 1.98.1");
    let build = TLS_H2_PERFORMANCE_WORKFLOW
        .find("cargo build --release --locked --bin cwl-pingora-gateway")
        .expect("TLS/H2 performance workflow must build the exact release gateway candidate");

    assert!(install < select && select < verify && verify < build);
    assert_eq!(
        TLS_H2_PERFORMANCE_WORKFLOW
            .matches("rustup toolchain install ")
            .count(),
        1
    );
    assert_eq!(
        TLS_H2_PERFORMANCE_WORKFLOW.matches("rustup default ").count(),
        1
    );
    assert!(!TLS_H2_PERFORMANCE_WORKFLOW.contains("rustup override"));
    assert!(!TLS_H2_PERFORMANCE_WORKFLOW.contains("1.98.0"));
    assert!(!TLS_H2_PERFORMANCE_WORKFLOW.contains("RUSTUP_TOOLCHAIN"));
    assert!(!contains_cargo_toolchain_selector(
        TLS_H2_PERFORMANCE_WORKFLOW
    ));
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
