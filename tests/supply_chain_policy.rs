//! Fail-closed regression contracts for the committed production dependency graph.

/// Returns the version of one exact package record in a Cargo lockfile.
fn locked_package_version(lockfile: &str, package_name: &str) -> Option<String> {
    for block in lockfile.split("[[package]]") {
        let mut name = None;
        let mut version = None;

        for line in block.lines().map(str::trim) {
            if let Some(value) = line
                .strip_prefix("name = \"")
                .and_then(|value| value.strip_suffix('"'))
            {
                name = Some(value);
            }
            if let Some(value) = line
                .strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
            {
                version = Some(value);
            }
        }

        if name == Some(package_name) {
            return version.map(str::to_owned);
        }
    }

    None
}

/// Proves package lookup cannot be satisfied by a dependency mention or a similarly named package.
#[test]
fn lockfile_lookup_matches_exact_package_records() {
    const LOCKFILE: &str = r#"version = 4

[[package]]
name = "consumer"
version = "1.0.0"
dependencies = ["derivative"]

[[package]]
name = "derivative-like"
version = "2.2.0"
"#;

    assert_eq!(
        locked_package_version(LOCKFILE, "consumer"),
        Some("1.0.0".to_owned())
    );
    assert_eq!(locked_package_version(LOCKFILE, "derivative"), None);
}

/// Fails until the immutable Pingora supplier graph removes RUSTSEC-2024-0388.
#[test]
fn rustsec_2024_0388_dependency_is_absent_from_committed_lock() {
    let derivative_version = locked_package_version(include_str!("../Cargo.lock"), "derivative");

    assert!(
        derivative_version.is_none(),
        "RUSTSEC-2024-0388 remains in the committed production graph through derivative {}; consume a reviewed immutable supplier repair instead of suppressing the advisory",
        derivative_version.as_deref().unwrap_or("<unknown-version>")
    );
}
