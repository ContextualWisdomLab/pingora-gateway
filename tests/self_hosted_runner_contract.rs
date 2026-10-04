//! Runner-admission contract: every owned execution job uses the organization's Linux CPU pool.

use serde_yaml::Value;

/// Parses the actual workflow mapping instead of matching comments or sample shell text.
fn assert_cpu_pool(source: &str, expected_jobs: usize) {
    let workflow: Value = serde_yaml::from_str(source).expect("workflow YAML must parse");
    let jobs = workflow["jobs"].as_mapping().expect("workflow needs jobs");
    assert_eq!(
        jobs.len(),
        expected_jobs,
        "every direct job is accounted for"
    );
    for (name, job) in jobs {
        let selector = &job["runs-on"];
        let mapping = selector
            .as_mapping()
            .expect("public PR execution must require the isolated runner group");
        assert_eq!(mapping.len(), 2, "runner authority must have no extra keys");
        assert_eq!(selector["group"].as_str(), Some("CWL CI isolated"));
        let labels = selector["labels"]
            .as_sequence()
            .expect("isolated capability labels must be explicit");
        let actual: Vec<&str> = labels
            .iter()
            .map(|label| {
                label
                    .as_str()
                    .expect("runner labels must be literal strings")
            })
            .collect();
        assert_eq!(
            actual,
            ["self-hosted", "Linux", "X64", "cwlab-ci-isolated"],
            "job {name:?} must use the established organization CPU pool"
        );
    }
}

#[test]
fn owned_ci_and_supply_jobs_use_the_self_hosted_cpu_pool() {
    assert_cpu_pool(include_str!("../.github/workflows/ci.yml"), 3);
    assert_cpu_pool(include_str!("../.github/workflows/supply-chain.yml"), 1);
}
