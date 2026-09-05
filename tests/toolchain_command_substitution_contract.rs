//! Fail-closed acceptance for compiler authority hidden in shell command substitution.

#[path = "support/toolchain_command_substitution.rs"]
mod toolchain_command_substitution_support;

use std::fs;
use toolchain_command_substitution_support::assert_no_hidden_compiler_authority;

/// Production release workflows and the OCI build must not hide compiler selection inside `$(...)`.
#[test]
fn release_paths_reject_hidden_compiler_authority_in_command_substitution() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
        "Dockerfile",
    ] {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"));
        assert_no_hidden_compiler_authority(path, &source);
    }
}

/// A verified default compiler must not be bypassable from an executable `$(...)` sub-shell.
#[test]
fn command_substitution_guard_rejects_alternate_compiler_authority() {
    for shell in [
        "echo $(RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked)",
        "printf '%s\\n' \"$(RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked)\"",
        "echo $(CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked)",
        "echo $(cargo +1.98.0 build --release --locked)",
        "echo $(rustup run 1.98.0 cargo build --release --locked)",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", shell);
        });
        assert!(
            result.is_err(),
            "command substitution must not hide alternate compiler authority: {shell}"
        );
    }
}

/// Ordinary command substitution remains available when it does not select Cargo compiler authority.
#[test]
fn command_substitution_guard_allows_non_compiler_subshells() {
    for shell in [
        "version=$(git rev-parse HEAD)",
        "artifact=$(cargo metadata --no-deps --format-version 1)",
        "printf '%s\\n' \"$(uname -m)\"",
        "literal=$(printf '%s' \"(not syntax)\")",
        "nested=$(printf '%s' \"$(uname -m)\")",
    ] {
        assert_no_hidden_compiler_authority("synthetic shell", shell);
    }
}
