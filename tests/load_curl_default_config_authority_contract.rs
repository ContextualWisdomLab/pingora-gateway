//! Fail-closed contract for curl's ambient default configuration in gateway evidence workflows.
//!
//! curl reads a default configuration file unless `-q`/`--disable` is the first command-line
//! parameter. Load, readiness, artifact-download, and OCI probes must therefore opt out explicitly
//! so runner-owned `.curlrc` state cannot redirect or mutate evidence without a repository delta.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";

fn shell_runs(source: &str) -> Option<Vec<String>> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let jobs = document.get("jobs")?.as_mapping()?;
    let mut runs = Vec::new();
    for job in jobs.values() {
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };
        for step in steps {
            if let Some(run) = step.get("run").and_then(Value::as_str) {
                runs.push(run.to_owned());
            }
        }
    }
    Some(runs)
}

fn is_curl_invocation(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("curl ")
        || line.contains("if curl ")
        || line.contains("$(curl ")
        || line.contains("&& curl ")
        || line.contains("|| curl ")
        || line.contains("; curl ")
}

fn curl_invocation_is_config_isolated(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("curl -q ")
        || line.contains("if curl -q ")
        || line.contains("$(curl -q ")
        || line.contains("&& curl -q ")
        || line.contains("|| curl -q ")
        || line.contains("; curl -q ")
}

fn all_curl_invocations_disable_default_config(source: &str) -> bool {
    let Some(runs) = shell_runs(source) else {
        return false;
    };
    let invocations: Vec<_> = runs
        .iter()
        .flat_map(|run| run.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| is_curl_invocation(line))
        .collect();
    !invocations.is_empty()
        && invocations
            .iter()
            .all(|line| curl_invocation_is_config_isolated(line))
}

#[test]
fn live_evidence_curl_invocations_ignore_ambient_default_config() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        all_curl_invocations_disable_default_config(&source),
        "every evidence-producing curl invocation must use `curl -q` so runner .curlrc state is ignored"
    );
}

#[test]
fn ordinary_curl_invocation_is_rejected() {
    let source = "jobs:\n  test:\n    steps:\n      - run: curl --fail http://127.0.0.1/livez\n";
    assert!(!all_curl_invocations_disable_default_config(source));
}

#[test]
fn disable_after_another_option_is_rejected() {
    let source = "jobs:\n  test:\n    steps:\n      - run: curl --fail -q http://127.0.0.1/livez\n";
    assert!(!all_curl_invocations_disable_default_config(source));
}

#[test]
fn first_parameter_disable_is_admitted_in_condition_and_substitution() {
    let source = "jobs:\n  test:\n    steps:\n      - run: |\n          if curl -q --fail http://127.0.0.1/livez; then echo ready; fi\n          content_type=\"$(curl -q --silent http://127.0.0.1/metrics)\"\n";
    assert!(all_curl_invocations_disable_default_config(source));
}
