# Protected Release Evidence Traceability

## Decision status

This document describes an unreleased, post-integration evidence-assembly lane. It does not publish a Git tag, GitHub Release, package, image, deployment, canary, or cutover and does not make a mutable release immutable.

## Problem

The release-quality contract requires the exact protected source revision to carry reproducible binaries, SBOM/security evidence, authenticated provenance, immutable publication, and rollback/cutover evidence. Pull-request evidence is necessary but insufficient: on a pull-request run the explicitly checked-out head and GitHub's synthetic merge signer/source identity are intentionally different. After protected integration, the existing `Release Reproducibility` push lane can re-establish one protected commit as checkout, attested source, and signer identity.

A second risk appears at packaging time. Rebuilding binaries or images while preparing a release would create another compiler/build authority after reproducibility and provenance have already been established. Copying arbitrary artifacts by run number without binding their repository, workflow, event, branch, source SHA, conclusion, receipt, and attestation could likewise assemble a plausible-looking release bundle from unrelated evidence.

## Selected boundary

`.github/workflows/protected-release-evidence.yml` is a manually dispatched, main-only evidence assembler. It accepts the run IDs of the completed `Release Reproducibility` and `Supply Chain` **push** runs for the selected protected-main commit. Before downloading artifacts it verifies through GitHub's API that each run:

- belongs to this repository;
- is completed with `success`;
- has `head_sha` equal to the workflow's own protected `github.sha`;
- has `head_branch=main` and `event=push`; and
- has the expected workflow name and path.

The downloaded reproducibility receipt must identify the same protected commit as `checkout_sha`, `attested_source_sha`, and `signer_sha`, must record the exact protected-main workflow certificate identity, and must report `result=byte-identical`. The supply-chain receipt must identify the same `source_sha`. Both binaries are then independently re-verified with `gh attestation verify` against the exact repository, protected-main workflow certificate identity, source/signer SHA, and hosted-runner requirement.

The assembler also verifies the content digests already written into both upstream receipts before copying anything. The reproducibility artifact retains its `release-binaries/` directory, so the lane consumes that real uploaded layout rather than assuming that `actions/upload-artifact` flattens paths. Downloaded `Cargo.lock` and `deny.toml` must additionally compare byte-for-byte with the protected checkout.

Only those already-built binaries and already-generated supply-chain artifacts are copied into the bundle. The assembler does not run Cargo, Rustup, Docker builds, dependency resolution, or another SBOM generator. It therefore remains downstream of the release compiler and evidence-producing workflows instead of becoming a second build authority.

The bundle contains both release binaries, the reproducibility receipt, committed lock and dependency policy, SPDX dependency SBOM, both image scan reports, the supply-chain receipt, a sorted SHA-256 manifest, and a bundle receipt. The two previously attested binary byte streams are explicitly normalized to executable mode `0755` before packaging rather than relying on artifact-transport permission behavior; mode normalization does not alter their contents or digests. The tar stream uses the protected commit timestamp, stable ordering, and numeric ownership metadata, and `gzip -n` suppresses gzip header name/time variability instead of relying on `tar -z` defaults. The receipt deliberately states `publication_state=unpublished`.

## Rejected alternatives

Rebuilding during packaging was rejected because a later toolchain, dependency resolver, environment, or source movement could produce artifacts different from the proven candidate. Accepting arbitrary successful run IDs was rejected because success alone does not establish source or workflow identity. Automatically running the assembler on every push was rejected because evidence-producing workflows can complete in either order and a release bundle is a deliberate promotion action rather than a general CI side effect. Publishing a normal GitHub Release from this lane was rejected because repository release immutability is an administrative policy outside this workflow's current authority.

The first archive implementation also used `tar -czf` directly and copied downloaded binaries without normalizing executable mode. Review rejected both choices before promotion: a release bundle should not depend on transport-specific permission behavior, and deterministic tar metadata alone does not explicitly suppress gzip-header variability. The initial download-path assumption was also checked against an actual #91 reproducibility artifact and corrected: the binary files are stored under `release-binaries/`, while supply-chain evidence is stored at artifact root. The current lane verifies those real layouts and the receipt-recorded SHA-256 values before assembly.

## Immutable-release boundary

GitHub immutable releases are the publication authority for the planned first release. Publication must fail closed unless repository or organization release immutability is actually enabled. The safe publication sequence is draft release, attach every exact asset, publish, then verify the release and each asset. The current connector/runtime cannot read or change the repository Administration endpoint that controls immutable releases, so this lane does not infer that policy from ordinary repository write access and does not create a release.

After that administrative prerequisite is verified, the publication implementation must consume this protected-source evidence bundle rather than rebuild it, bind the intended version and changelog to the same protected source, publish all assets before making the release immutable, and verify the resulting release/asset attestations. A published release remains downstream of independent governance, supplier release qualification, and the remaining representative NUMA evidence.

## Evidence and follow-up

PR evidence for this lane consists of the normal exact-head repository gates plus the executable structural contract in `tests/protected_release_evidence_workflow_contract.rs`. The actual bundle is post-integration evidence because `workflow_dispatch` is intentionally required to run from protected `main` and must consume successful same-SHA push artifacts.

The next release sequence is: independent governance and dependency-ordered protected integration; exact protected-main reproducibility/provenance and supply-chain push runs; this protected-source bundle; verified immutable-release administration; version/changelog/tag/package publication from the same evidence; release and asset verification; representative deployment evidence; shadow/canary; observed rollback; cutover; then verified legacy proxy removal.
