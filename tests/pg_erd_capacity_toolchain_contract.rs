const CAPACITY_WORKFLOW: &str = include_str!("../.github/workflows/pg-erd-capacity.yml");

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
    assert!(!CAPACITY_WORKFLOW.contains("1.98.0"));
    assert!(!CAPACITY_WORKFLOW.contains("RUSTUP_TOOLCHAIN"));
    assert!(!CAPACITY_WORKFLOW.contains("cargo +"));
}
