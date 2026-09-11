# Release Provenance Traceability

## Decision status

Proposed release-evidence increment. This document covers cryptographically signed provenance for the byte-identical release binaries produced by `release-reproducibility.yml`. It does not declare an immutable GitHub Release, tag/package publication, independent-builder reproducibility, OCI-image reproducibility, deployment, rollback, canary, cutover, or legacy-proxy removal.

## Problem

PR #90 proves that two clean builds from one exact source, lock file, verified Rust 1.98.1 compiler, controlled Cargo configuration hierarchy, stable source timestamp, and canonical build path produce byte-identical gateway binaries. Byte equality alone does not authenticate who built those bytes, which workflow produced them, or which source digest the builder was executing.

A promotion candidate therefore still needs provenance whose signature identity is outside ordinary workflow-controlled predicate data. GitHub artifact attestations use an Actions OIDC identity and Sigstore-issued signing certificate, associate the resulting in-toto/SLSA statement with the repository, and support verification of repository, signer workflow, source digest, signer digest, and runner class.

## Constraints and alternatives

The gateway must not invent a repository-local signing key, store long-lived private signing material, or treat an uploaded checksum file as authenticated provenance. A release workflow compromise can manipulate ordinary files and user-controlled provenance predicate content, so verification must bind certificate-backed actor identity as tightly as the available GitHub verifier permits.

The selected path uses GitHub's current `actions/attest` provenance mode rather than the compatibility `actions/attest-build-provenance` wrapper. The action is pinned to exact commit `1e69f48acb82d1966a394da916b4c1698aa569d6`, corresponding to the current `v4` ref observed during implementation. Mutable action tags are not accepted as release evidence.

The workflow grants only the additional permissions required for artifact attestation: `id-token: write`, `attestations: write`, and `artifact-metadata: write`, while retaining `contents: read`. Attestation happens only after the two independently clean builds have passed SHA-256 and byte-for-byte equality. No failed or mismatched candidate is attested.

## RED and causal repair

Exact `7517c3691be1b4437914bb7498f95f105bf98d88` added `tests/release_provenance_workflow_contract.rs` before changing the workflow. Its hosted runs were cancelled by ordinary-forward development before semantic execution, so that exact supplies source-level RED intent only and is not credited as hosted RED evidence.

The first complete hosted semantic attempt was exact `93b6b3fefc3981fd6dbf687316a9d7ebe6a19fcc`, Release Reproducibility run `34633027944`. Both isolated Rust 1.98.1 builds succeeded, the binaries compared byte-for-byte equal, and `actions/attest` successfully created Build Provenance for both subjects using the Sigstore public-good instance. The attestation was logged in Rekor at log index `2796681697` and uploaded as repository attestation `46932569`. Verification then failed closed with `expected BuildSignerDigest to be 93b6b3f..., got 40bff1c66fb302dba0ed03bdab67934666c63223`.

That failure exposed an identity-model defect in the initial verifier, not an attestation or binary-reproducibility failure. For a `pull_request` workflow, `github.event.pull_request.head.sha` identifies the exact source revision intentionally checked out and built, while `github.sha` identifies the synthetic PR merge commit whose workflow identity is encoded in the OIDC/Sigstore signing certificate. Binding both `source-digest` and `signer-digest` to the PR head therefore rejects a valid attestation and, more importantly, models two distinct authorities as if they were one.

The causal repair keeps `EXPECTED_SHA=${{ github.event.pull_request.head.sha || github.sha }}` as source authority and introduces `SIGNER_SHA=${{ github.sha }}` as workflow-signer authority. `gh attestation verify` continues to require `source-digest=$EXPECTED_SHA`, but now requires `signer-digest=$SIGNER_SHA`. On protected-branch push runs the two naturally collapse to the same protected commit; on PR runs they intentionally remain distinct. The reproducibility receipt records both identities so later release evidence cannot silently conflate them.

## Selected acceptance

A current exact head is provenance-GREEN only when all of the following hold:

1. exact checkout, Rust 1.98.1 compiler selection, Cargo authority isolation, locked dependency acquisition, and clean byte-reproducible builds remain unchanged from #90;
2. both release binaries are byte-identical before attestation starts;
3. `actions/attest` is pinned to exact commit `1e69f48acb82d1966a394da916b4c1698aa569d6` and receives both release binaries as subjects;
4. GitHub OIDC/Sigstore provenance generation succeeds with `id-token`, `attestations`, and artifact-metadata write authority but read-only repository contents;
5. `gh attestation verify` succeeds separately for both binaries while enforcing this repository, this exact signer workflow, `source-digest=$EXPECTED_SHA`, `signer-digest=$SIGNER_SHA`, and denial of self-hosted-runner provenance;
6. the receipt records the exact source and signer identities used for verification;
7. normal CI, Supply Chain, capacity, TLS/H2 performance, reproducibility, review-thread, and governance evidence remain exact-current.

## Risk and interpretation

A GREEN result authenticates the exact candidate binaries to a GitHub Actions workflow identity and separately binds the built source digest and workflow-signer digest using GitHub's OIDC/Sigstore attestation system. GitHub CLI documentation notes that certificate identity and verified timestamps are the portions not controlled by workflow-authored predicate content; the selected verification therefore binds actor identity rather than trusting predicate fields alone.

For PR evidence, a successful signer-digest check authenticates the synthetic merge commit under which GitHub executed the workflow, while the source-digest check authenticates the explicit PR-head source revision that this workflow checked out and built. Neither identity is substituted for the other.

This remains candidate provenance. It does not make a mutable PR artifact an immutable product release, does not establish that a second independent builder reproduces the bytes, and does not prove deployment provenance. Version/CHANGELOG/tag/package identity, immutable release publication, release-asset verification, representative NUMA evidence, supplier release qualification, rollback, shadow/canary, cutover, and legacy removal remain later gates.

## Primary-source traceability

- GitHub documents artifact attestations as signed attestations that establish where and how build artifacts were produced. Current guidance uses `actions/attest@v4` and requires OIDC and attestation write permissions.
- The current `actions/attest` documentation states that public repositories use the Sigstore public-good instance and that provenance mode emits SLSA build provenance when no custom predicate or SBOM mode is selected.
- GitHub CLI `gh attestation verify` validates artifact integrity, predicate type, repository/owner identity, and optionally signer workflow, signer digest, source digest, OIDC issuer, and runner class. GitHub recommends specifying signer workflow identity as precisely as possible.
- GitHub Actions defines `github.sha` for `pull_request` workflows as the last merge commit on the PR merge branch, while the pull request head SHA is available separately from the event payload. The provenance lane therefore keeps source and signer commit identities distinct on PR runs.
- GitHub CLI warns that ordinary provenance predicate data can be manipulated by the originating workflow; certificate identity and verified timestamps have different trust properties. This lane therefore relies on certificate-bound actor/source verification rather than custom predicate claims.

## References

GitHub, Inc. (2026). *Using artifact attestations to establish provenance for builds*. GitHub Docs. https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations

GitHub, Inc. (2026). *Contexts reference*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

GitHub, Inc. (2026). *gh attestation verify*. GitHub CLI Manual. https://cli.github.com/manual/gh_attestation_verify

GitHub, Inc. (2026). *actions/attest*. GitHub. https://github.com/actions/attest

Supply-chain Levels for Software Artifacts. (n.d.). *Provenance (v1.2)*. https://slsa.dev/spec/v1.2/provenance
