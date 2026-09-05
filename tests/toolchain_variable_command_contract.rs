//! Regression contract for Cargo executable authority hidden behind shell variables.

#[path = "support/toolchain_command_substitution.rs"]
mod toolchain_command_substitution_support;

use toolchain_command_substitution_support::assert_no_hidden_compiler_authority;

/// Release-producing shell must not hide Cargo behind a variable command word.
#[test]
fn variable_cargo_command_is_rejected_without_command_substitution() {
    for shell in [
        "CARGO=cargo; $CARGO build --release --locked",
        "CARGO=/home/runner/.cargo/bin/cargo; \"$CARGO\" +1.98.0 build --release --locked",
        "export CARGO=cargo; ${CARGO:?} build --release --locked",
        "readonly CARGO=/opt/rust-1.98.0/bin/cargo; \"$CARGO\" build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", shell);
        });
        assert!(
            result.is_err(),
            "variable Cargo command must remain visible to compiler authority checks: {shell}"
        );
    }
}

/// A dynamically computed persistent executable must fail closed when later used as a command.
#[test]
fn command_substitution_cargo_alias_is_rejected() {
    for shell in [
        "CARGO=\"$(command -v cargo)\"; \"$CARGO\" build --release --locked",
        "CARGO=\"$(printf /opt/rust-1.98.0/bin/cargo)\"; ${CARGO:?} build --release --locked",
        "export CARGO=\"$(printf /opt/rust-1.98.0/bin/cargo)\"; $CARGO build --release --locked",
        "CARGO=$(command -v cargo); \"$CARGO\" build --release --locked",
        "CARGO+=$(command -v cargo); \"$CARGO\" build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", shell);
        });
        assert!(
            result.is_err(),
            "command-substitution Cargo alias must not hide executable authority: {shell}"
        );
    }
}

/// A command-local assignment prefix must not be remembered as parent-shell Cargo authority.
#[test]
fn command_local_cargo_assignment_does_not_persist() {
    for shell in [
        "CARGO=cargo printf ok; \"$CARGO\" build --release --locked",
        "CARGO=\"$(command -v cargo)\" printf ok; \"$CARGO\" build --release --locked",
    ] {
        assert_no_hidden_compiler_authority("synthetic shell", shell);
    }
}
