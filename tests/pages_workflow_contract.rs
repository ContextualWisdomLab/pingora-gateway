use std::fs;

const WORKFLOW: &str = ".github/workflows/pages.yml";

fn workflow() -> String {
    fs::read_to_string(WORKFLOW).expect("Pages workflow must exist")
}

#[test]
fn pages_publication_is_manual_and_protected_main_only() {
    let yaml = workflow();
    assert!(yaml.contains("workflow_dispatch:"));
    assert!(!yaml.contains("pull_request:"));
    assert!(!yaml.contains("push:"));
    assert!(yaml.contains("test \"$GITHUB_REF\" = \"refs/heads/main\""));
    assert!(yaml.contains("ref: ${{ github.sha }}"));
    assert!(yaml.contains("test \"$(git rev-parse HEAD)\" = \"$GITHUB_SHA\""));
}

#[test]
fn pages_actions_are_immutable_and_permissions_are_job_scoped() {
    let yaml = workflow();
    for expected in [
        "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803",
        "actions/configure-pages@45bfe0192ca1faeb007ade9deae92b16b8254a0d",
        "actions/jekyll-build-pages@44a6e6beabd48582f863aeeb6cb2151cc1716697",
        "actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9",
        "actions/deploy-pages@368f82528645a54fb793d4d04e342629a3f51346",
        "permissions: {}",
        "build:\n    name: build-pages\n    runs-on: ubuntu-latest\n    permissions:\n      contents: read\n      pages: read",
        "deploy:\n    name: deploy-pages\n    runs-on: ubuntu-latest\n    needs: build\n    permissions:\n      pages: write\n      id-token: write",
        "persist-credentials: false",
    ] {
        assert!(
            yaml.contains(expected),
            "missing Pages contract: {expected}"
        );
    }
}

#[test]
fn pages_artifact_and_public_site_are_bound_to_exact_source() {
    let yaml = workflow();
    for expected in [
        "source: ./docs",
        "destination: ./_site",
        "build_revision: ${{ github.sha }}",
        "printf '%s\\n' \"$GITHUB_SHA\" > _site/source-sha.txt",
        "path: ./_site",
        "name: github-pages",
        "url: ${{ steps.deployment.outputs.page_url }}",
        "EXPECTED_SHA: ${{ github.sha }}",
        "source-sha.txt",
        "Published Pages source identity did not converge",
        "marker_file=\"$(mktemp)\"",
        "trap 'rm -f \"$marker_file\"' EXIT",
        "marker_status=\"$(curl",
        "root_status=\"$(curl",
        "[ \"$marker_status\" = \"200\" ]",
        "[ \"$(cat \"$marker_file\")\" = \"$EXPECTED_SHA\" ]",
        "[ \"$root_status\" = \"200\" ]",
    ] {
        assert!(
            yaml.contains(expected),
            "missing source-identity contract: {expected}"
        );
    }

    assert_eq!(
        yaml.matches("curl --fail --silent --show-error --location")
            .count(),
        2,
        "both public verification requests must stay structurally visible"
    );
    assert_eq!(
        yaml.matches("--proto '=https'").count(),
        2,
        "both public verification requests must allow only HTTPS"
    );
    assert_eq!(
        yaml.matches("--proto-redir '=https'").count(),
        2,
        "both public verification requests must reject redirect downgrade"
    );
    assert_eq!(
        yaml.matches("--max-redirs 0").count(),
        2,
        "both public verification requests must reject cross-origin redirect substitution"
    );
    assert_eq!(
        yaml.matches("--write-out '%{http_code}'").count(),
        2,
        "both public verification requests must prove an explicit HTTP 200 response"
    );
    assert!(
        !yaml.contains("2>/dev/null || true"),
        "marker verification must not mask a failed transfer while retaining its response body"
    );
}

#[test]
fn pages_deployments_are_serialized_without_cancelling_in_flight_publish() {
    let yaml = workflow();
    assert!(yaml.contains("group: pages"));
    assert!(yaml.contains("cancel-in-progress: false"));
    assert!(yaml.contains("needs: build"));
}
