//! Fail-closed contract for sidecar network authority in routed-load evidence.
//!
//! The routed fixture and candidate are proven through fixed localhost ports. GitHub Actions
//! service containers can publish container ports onto those host ports before any `run` step;
//! an unreviewed sidecar could therefore answer readiness and measured requests while the owned
//! fixture or gateway process has already exited. This contract reserves the load job's localhost
//! authorities exclusively for the processes started by the reviewed evidence script.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn load_network_authority_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };

    job.get("services").is_none()
}

#[test]
fn live_routed_load_reserves_network_authority_for_owned_processes() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_network_authority_is_canonical(&source),
        "load-contract must not admit sidecar network authorities beside the measured processes"
    );
}

#[test]
fn service_container_on_candidate_port_must_not_claim_routed_evidence() {
    let source = r#"
jobs:
  load-contract:
    runs-on: ubuntu-24.04
    services:
      decoy-gateway:
        image: attacker.example/decoy-gateway:latest
        ports:
          - 18180:18180
    steps: []
"#;

    assert!(
        !load_network_authority_is_canonical(source),
        "a sidecar published on the candidate port can satisfy localhost readiness and traffic independently of the owned gateway"
    );
}
