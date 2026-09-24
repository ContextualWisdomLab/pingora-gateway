//! Fail-closed contract for the native tools that build the routed candidate.
//!
//! Sanitizing environment variables is not sufficient when the hosted Ubuntu image itself can move.
//! The routed release build compiles vendored OpenSSL and other native dependencies with the system
//! C toolchain, assembler, GNU make, Perl, CMake, pkg-config, and binutils. Exact evidence therefore
//! verifies the versions documented by the admitted Ubuntu 24.04 runner image before rebuilding the
//! candidate.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const REQUIRED_ASSERTIONS: &[&str] = &[
    "[[ \"$(cc -dumpfullversion -dumpversion)\" == \"13.3.0\" ]]",
    "[[ \"$(as --version | head -n1)\" == \"GNU assembler (GNU Binutils for Ubuntu) 2.42\" ]]",
    "[[ \"$(make --version | head -n1)\" == \"GNU Make 4.3\" ]]",
    "[[ \"$(perl -e 'print $^V')\" == \"v5.38.2\" ]]",
    "[[ \"$(cmake --version | head -n1)\" == \"cmake version 3.31.6\" ]]",
    "[[ \"$(pkg-config --version)\" == \"1.8.1\" ]]",
    "[[ \"$(ar --version | head -n1)\" == \"GNU ar (GNU Binutils for Ubuntu) 2.42\" ]]",
    "[[ \"$(ranlib --version | head -n1)\" == \"GNU ranlib (GNU Binutils for Ubuntu) 2.42\" ]]",
    "[[ \"$(nm --version | head -n1)\" == \"GNU nm (GNU Binutils for Ubuntu) 2.42\" ]]",
];

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn native_toolchain_is_verified_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };
    if clean_index >= build_index {
        return false;
    }

    REQUIRED_ASSERTIONS.iter().all(|required| {
        let matches: Vec<_> = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| (*line == *required).then_some(index))
            .collect();
        matches.len() == 1 && matches[0] < clean_index
    })
}

#[test]
fn live_routed_rebuild_verifies_native_toolchain_identity() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        native_toolchain_is_verified_before_rebuild(&source),
        "routed evidence must verify the admitted native toolchain versions before rebuilding native dependencies"
    );
}

#[test]
fn missing_native_tool_assertions_are_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!native_toolchain_is_verified_before_rebuild(&source));
}

#[test]
fn partial_native_tool_assertions_are_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n",
        REQUIRED_ASSERTIONS[0]
    );
    assert!(!native_toolchain_is_verified_before_rebuild(&source));
}

#[test]
fn native_tool_assertions_after_build_are_rejected() {
    let assertions = REQUIRED_ASSERTIONS.join("\n          ");
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {assertions}\n"
    );
    assert!(!native_toolchain_is_verified_before_rebuild(&source));
}

#[test]
fn complete_native_tool_assertions_before_clean_rebuild_are_admitted() {
    let assertions = REQUIRED_ASSERTIONS.join("\n          ");
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {assertions}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(native_toolchain_is_verified_before_rebuild(&source));
}
