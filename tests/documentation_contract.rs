//! Executable acceptance for public API documentation completeness.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// The library and hosted CI must fail closed when public Rust API documentation regresses.
#[test]
fn public_rustdoc_is_a_required_exact_head_gate() {
    let library = read_repository_file("src/lib.rs");
    let workflow = read_repository_file(".github/workflows/ci.yml");

    assert!(
        library.contains("#![deny(missing_docs)]"),
        "library crate must deny missing public documentation"
    );
    assert!(
        workflow.contains("RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --locked"),
        "hosted CI must build public documentation with warnings denied"
    );
}
