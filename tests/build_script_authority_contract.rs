//! Locks package-root build-script authority to the reviewed Cargo manifest.
//!
//! Cargo automatically compiles and runs a root `build.rs` when `[package].build` is omitted.
//! This repository does not own a package build script, so leaving that implicit discovery enabled
//! allows an untracked workspace file to become executable build input without changing tracked
//! Rust sources or dependency metadata.

use std::fs;

const MANIFEST: &str = "Cargo.toml";

fn package_disables_implicit_build_script(source: &str) -> bool {
    let mut in_package = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && line == "build = false" {
            return true;
        }
    }

    false
}

#[test]
fn live_manifest_disables_implicit_root_build_script_discovery() {
    let manifest = fs::read_to_string(MANIFEST).expect("Cargo.toml should be readable UTF-8");
    assert!(
        package_disables_implicit_build_script(&manifest),
        "packages without an owned build script must set [package].build = false"
    );
}

#[test]
fn explicit_package_build_false_is_admitted() {
    let manifest = r#"
[package]
name = "example"
build = false

[dependencies]
serde = "1"
"#;
    assert!(package_disables_implicit_build_script(manifest));
}

#[test]
fn omitted_build_key_is_rejected() {
    let manifest = r#"
[package]
name = "example"

[dependencies]
serde = "1"
"#;
    assert!(!package_disables_implicit_build_script(manifest));
}

#[test]
fn commented_build_false_is_rejected() {
    let manifest = r#"
[package]
name = "example"
# build = false
"#;
    assert!(!package_disables_implicit_build_script(manifest));
}

#[test]
fn build_path_is_rejected() {
    let manifest = r#"
[package]
name = "example"
build = "build.rs"
"#;
    assert!(!package_disables_implicit_build_script(manifest));
}

#[test]
fn build_false_outside_package_is_rejected() {
    let manifest = r#"
[package]
name = "example"

[package.metadata]
build = false
"#;
    assert!(!package_disables_implicit_build_script(manifest));
}
