# Release Provenance Traceability

## Decision status

Proposed release-evidence increment. This document covers cryptographically signed provenance for the byte-identical release binaries produced by `release-reproducibility.yml`. It does not declare an immutable GitHub Release, tag/package publication, independent-builder reproducibility, OCI-image reproducibility, deployment, rollback, canary, cutover, or legacy-proxy removal.

## Problem

PR #90 proves that two clean builds from one exact source, lock file, verified Rust 1.98.1 compiler, controlled Cargo configuration hierarchy, stable source timestamp, and canonical build path produce byte-identical gateway binaries. Byte equality alone does not authenticate who built those bytes, which workflow produced them, or which GitHub execution identity signed the evidence.

A promotion candidate therefore still needs provenance whose signature identity is outside ordinary workflow-controlled predicate data. GitHub artifact attestations use an Actions OIDC identity and Sigstore-issued signing certificate, associate the resulting in-toto/SLSA statement with the repository, and support verification of repository, exact certificate SubjectAlternativeName, source-repository digest, signer digest, and runner class.

## Constraints and alternatives

The gateway must not invent a repository-local signing key, store long-lived private signing material, or treat an uploaded checksum file as authenticated provenance. A release workflow compromise can manipulate ordinary files and user-controlled provenance predicate content, so verification must bind certificate-backed actor identity as tightly as the available GitHub verifier permits.

The selected path uses GitHub's current `actions/attest` provenance mode rather than the compatibility `actions/attest-build-provenance` wrapper. The action is pinned to exact commit `1e69f48acb82d1966a394da916b4c1698aa569d6`, corresponding to the current `v4` ref observed during implementation. Mutable action tags are not accepted as release evidence.

The workflow grants only the additional permissions required for artifact attestation: `id-token: write`, `attestations: write`, and `artifact-metadata: write`, while retaining `contents: read`. Attestation happens only after the two independently clean builds have passed SHA-256 and byte-for-byte equality. No failed or mismatched candidate is attested.

## RED and causal repair

Exact `7517c3691be1b4437914bb7498f95f105bf98d88` added `tests/release_provenance_workflow_contract.rs` before changing the workflow. Its hosted runs were cancelled by ordinary-forward development before semantic execution, so that exact supplies source-level RED intent only and is not credited as hosted RED evidence.

The first complete hosted semantic attempt was exact `93b6b3fefc3981fd6dbf687316a9d7ebe6a19fcc`, Release Reproducibility run `34633027944`. Both isolated Rust 1.98.1 builds succeeded, the binaries compared byte-for-byte equal, and `actions/attest` successfully created Build Provenance for both subjects using the Sigstore public-good instance. The attestation was logged in Rekor at log index `2796681697` and uploaded as repository attestation `46932569`. Verification then failed closed with `expected BuildSignerDigest to be 93b6b3f..., got 40bff1c66fb302dba0ed03bdab67934666c63223`.

That failure exposed an identity-model defect in the initial verifier, not an attestation or binary-reproducibility failure. For a `pull_request` workflow, `github.event.pull_request.head.sha` identifies the exact source revision intentionally checked out and built, while `github.sha` identifies the synthetic PR merge commit under which GitHub executes the workflow and issues the OIDC certificate. GitHub's own attestation examples show that, for pull-request attestations, `Source Repository Digest`, `Build Signer Digest`, and the workflow certificate identity are all rooted in that merge-branch execution commit. The default attestation therefore cannot honestly be described as cryptographically setting `Source Repository Digest` to an explicitly checked-out PR-head SHA.

The repair keeps two evidence layers separate. `EXPECTED_SHA=${{ github.event.pull_request.head.sha || github.sha }}` remains the exact checkout/build authority and is enforced by `git rev-parse HEAD == EXPECTED_SHA`, deterministic build inputs, and the receipt. `SIGNER_SHA=${{ github.sha }}` is the GitHub-attested source/signer authority and is enforced by both `--source-digest=$SIGNER_SHA` and `--signer-digest=$SIGNER_SHA`. This does not pretend that the OIDC certificate directly authenticates an alternate checkout SHA on a pull-request run.

A requested technical review also identified a second, valid identity-binding gap: path-only `--signer-workflow` is weaker than an exact certificate SAN policy and can be vulnerable to pattern-matching ambiguity in verifier implementations. The stronger repair removes that path-pattern check and sets `CERT_IDENTITY=https://github.com/${{ github.repository }}/.github/workflows/release-reproducibility.yml@${{ github.ref }}`. Verification uses `--cert-identity=$CERT_IDENTITY`, whose GitHub CLI contract is an exact SubjectAlternativeName match, while independently retaining attested source/signer digest checks and hosted-runner denial.

On protected-branch push runs, exact checkout SHA, attested source SHA, signer SHA, and the protected workflow commit naturally collapse to the same protected commit and the certificate identity ends in `@refs/heads/main`. On PR runs, the receipt deliberately distinguishes the explicitly built PR-head checkout from the GitHub-attested synthetic `refs/pull/<n>/merge` source/signer identity. A PR attestation is therefore candidate evidence only; promotion-grade release provenance is re-established on the protected release source rather than treating a PR certificate as proof of a different checkout digest.

## Selected acceptance

A current exact head is provenance-GREEN only when all of the following hold:

1. exact checkout, Rust 1.98.1 compiler selection, Cargo authority isolation, locked dependency acquisition, and clean byte-reproducible builds remain unchanged from #90;
2. both release binaries are byte-identical before attestation starts;
3. `actions/attest` is pinned to exact commit `1e69f48acb82d1966a394da916b4c1698aa569d6` and receives both release binaries as subjects;
4. GitHub OIDC/Sigstore provenance generation succeeds with `id-token`, `attestations`, and artifact-metadata write authority but read-only repository contents;
5. `gh attestation verify` succeeds separately for both binaries while enforcing this repository, an exact certificate SAN for this workflow and current GitHub ref, `source-digest=$SIGNER_SHA`, `signer-digest=$SIGNER_SHA`, and denial of self-hosted-runner provenance;
6. the receipt records `checkout_sha`, `attested_source_sha`, `signer_sha`, and the exact certificate identity without conflating their meanings;
7. normal CI, Supply Chain, capacity, TLS/H2 performance, reproducibility, review-thread, and governance evidence remain exact-current.

## Risk and interpretation

A GREEN PR result authenticates the candidate binary digests to a GitHub Actions workflow certificate identity and GitHub's PR merge execution digest. It also records and enforces which PR-head commit the attested workflow explicitly checked out and built, but that checkout claim is workflow-enforced evidence rather than a distinct certificate `Source Repository Digest`. This distinction matters because ordinary workflow state and provenance predicate fields can be influenced by the workflow itself.

GitHub CLI documents `--cert-identity` as an exact match against the certificate SubjectAlternativeName. The verifier combines that exact identity with source/signer digest and hosted-runner constraints. GitHub further notes that certificate identity and verified timestamps are the portions not controlled by workflow-authored predicate content. The selected policy therefore bases actor identity on the certificate and does not overstate PR-head checkout metadata as a separate cryptographic certificate claim.

For a protected `main` release execution, `EXPECTED_SHA` and `SIGNER_SHA` become the same protected commit. That protected execution is the point at which the release candidate's exact source checkout and GitHub-attested source/signer digest converge and can be used for immutable release evidence.

This remains candidate provenance. It does not make a mutable PR artifact an immutable product release, does not establish that a second independent builder reproduces the bytes, and does not prove deployment provenance. Version/CHANGELOG/tag/package identity, immutable release publication, release-asset verification, representative NUMA evidence, supplier release qualification, rollback, shadow/canary, cutover, and legacy removal remain later gates.

## Primary-source traceability

- GitHub documents artifact attestations as signed attestations that establish where and how build artifacts were produced. Current guidance uses `actions/attest@v4` and requires OIDC and attestation write permissions.
- The current `actions/attest` documentation states that public repositories use the Sigstore public-good instance and that provenance mode emits SLSA build provenance when no custom predicate or SBOM mode is selected.
- GitHub CLI `gh attestation verify` validates artifact integrity, predicate type, repository/owner identity, and optionally exact certificate identity, signer digest, source digest, OIDC issuer, and runner class. `--cert-identity` requires the certificate SubjectAlternativeName to match the supplied value exactly.
- GitHub Actions defines `GITHUB_SHA` for `pull_request` workflows as the last merge commit on the PR merge branch, while the pull request head SHA is available separately from the event payload.
- Published GitHub attestation examples for pull-request workflows show `Source Repository Digest` and `Build Signer Digest` equal to the PR merge execution commit and workflow identity at `refs/pull/<n>/merge`.
- GitHub CLI warns that ordinary provenance predicate data can be manipulated by the originating workflow; certificate identity and verified timestamps have different trust properties. This lane therefore relies on certificate-bound actor identity rather than custom predicate claims.

## References

GitHub, Inc. (2026). *Artifact attestations*. GitHub Docs. https://docs.github.com/en/actions/concepts/security/artifact-attestations

GitHub, Inc. (2026). *Using artifact attestations to establish provenance for builds*. GitHub Docs. https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations

GitHub, Inc. (2026). *Events that trigger workflows*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows

GitHub, Inc. (2026). *gh attestation verify*. GitHub CLI Manual. https://cli.github.com/manual/gh_attestation_verify

GitHub, Inc. (2026). *actions/attest*. GitHub. https://github.com/actions/attest

Supply-chain Levels for Software Artifacts. (n.d.). *Provenance (v1.2)*. https://slsa.dev/spec/v1.2/provenance
