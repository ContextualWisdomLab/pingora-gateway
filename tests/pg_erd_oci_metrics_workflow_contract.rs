//! Regression contract for proving that pg-erd OCI metrics authority is the Prometheus service.
//!
//! A bare HTTP 200 on the metrics port is insufficient because a mistakenly bound proxy service
//! could also answer `/metrics`. The OCI acceptance must therefore verify the supplier Prometheus
//! response media type without requiring any metric family before application traffic emits one.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const OCI_JOB: &str = "oci-runtime";
const PG_ERD_RUNTIME_STEP: &str = "Exercise pg-erd read-only least-privilege runtime";

/// Returns the shell body for the pg-erd OCI runtime acceptance step.
fn pg_erd_runtime_script() -> String {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    let document: Value =
        serde_yaml::from_str(&source).expect("CI workflow YAML should parse before validation");
    document
        .get("jobs")
        .and_then(|jobs| jobs.get(OCI_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step.get("name").and_then(Value::as_str) == Some(PG_ERD_RUNTIME_STEP))
        })
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .expect("pg-erd OCI runtime step should define a shell script")
        .to_string()
}

/// Requires the pg-erd OCI gate to prove Prometheus media-type identity, not only HTTP reachability.
#[test]
fn pg_erd_metrics_acceptance_proves_prometheus_media_type() {
    let script = pg_erd_runtime_script();

    assert!(
        script.contains("http://127.0.0.1:6289/metrics"),
        "OCI acceptance must exercise the separately published metrics listener"
    );
    assert!(
        script.contains("--write-out")
            && script.contains("%{content_type}")
            && script.contains("metrics_content_type")
            && script.contains("[[ \"${metrics_content_type%%;*}\" == \"text/plain\" ]]"),
        "a bare 200 or text/plain-prefixed invalid media type can false-green a misbound listener; acceptance must capture curl's response content type and normalize only semicolon parameters in the actual equality predicate before requiring exact text/plain"
    );
    assert!(
        !script.contains("== text/plain*"),
        "prefix-wildcard media-type acceptance would admit invalid values such as text/plainfoo"
    );
}
