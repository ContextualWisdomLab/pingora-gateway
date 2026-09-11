# Protected Release Evidence Traceability

## Decision status

This document describes an unreleased, post-integration evidence-assembly lane. It does not publish a Git tag, GitHub Release, package, image, deployment, canary, or cutover and does not make a mutable release immutable.

## Problem

The release-quality contract requires the exact protected source revision to carry reproducible binaries, SBOM/security evidence, authenticated provenance, immutable publication, and rollback/cutover evidence. Pull-request evidence is necessary but insufficient: on a pull-request run the explicitly checked-out head and GitHub's synthetic merge signer/source identity are intentionally different. After protected integration, the existing `Release Reproducibility` push lane can re-establish one protected commit as checkout, attested source, and signer identity.

A second risk appears at packaging time. Rebuilding binaries or images while preparing a release would create another compiler/build authority after reproducibility and provenance have already been established. Copying arbitrary artifacts by run number without binding their repository, workflow, event, branch, source SHA, conclusion, receipt, and attestation could likewise assemble a plausible-looking release bundle from unrelated evidence. GitHub's artifact-attestation guidance treats provenance as useful only when the attestation and signer identity are actually verified, rather than merely generated (GitHub, n.d.-a).

## Selected boundary

`.github/workflows/protected-release-evidence.yml` is a manually dispatched, main-only evidence assembler. It accepts the run IDs of the completed `Release Reproducibility` and `Supply Chain` **push** runs for the selected protected-main commit. Before downloading artifacts it verifies through GitHub's API that each run:

- belongs to this repository;
- is completed with `success`;
- has `head_sha` equal to the workflow's own protected `github.sha`;
- has `head_branch=main` and `event=push`; and
- has the expected workflow name and path.

The downloaded reproducibility receipt must contain exactly one record for each authoritative identity field: `checkout_sha`, `attested_source_sha`, `signer_sha`, `certificate_identity`, and `result`. Those values must identify the same protected commit as checkout, attested source, and signer identity, the exact protected-main workflow certificate identity, and `result=byte-identical`. The supply-chain receipt likewise must contain exactly one `source_sha` record equal to the protected commit. Duplicate or contradictory identity fields fail closed rather than letting one expected line mask another conflicting line. Both binaries are then independently re-verified with `gh attestation verify` against the exact repository, protected-main workflow certificate identity, source/signer SHA, and hosted-runner requirement. This follows GitHub's primary guidance to generate provenance for release binaries and verify the resulting attestations with GitHub CLI (GitHub, n.d.-b).

The assembler also verifies the content digests already written into both upstream receipts before copying anything. Every packaged upstream file must have exactly one matching SHA-256 record in its owning receipt; a receipt that omits an expected binary/SBOM/scan/lock/policy digest or repeats the same path fails closed. The verifier then runs `sha256sum --check --strict` on that single record. This exact-coverage rule matters because filtering a receipt to any matching subset and passing the resulting lines to `sha256sum --check` can succeed even when another expected artifact has no receipt record at all. The reproducibility artifact retains its `release-binaries/` directory, so the lane consumes that real uploaded layout rather than assuming that `actions/upload-artifact` flattens paths. Downloaded `Cargo.lock` and `deny.toml` must additionally compare byte-for-byte with the protected checkout.

Digest authenticity is not the same as scanned-object identity. The Supply Chain receipt records the local Docker image IDs for both candidate images, while each Trivy JSON records the scanned `ArtifactName`, `ArtifactType`, `Metadata.Reference`, `Metadata.ImageID`, and repository tags. A hostile or stale scan JSON can be re-hashed into an otherwise self-consistent receipt while still describing a different image. The assembler therefore requires exactly one `generic_local_image_id` and `pg_erd_local_image_id`, validates each as a lowercase `sha256:<64-hex>` identifier, and requires the corresponding Trivy report to identify `container_image`, the exact source-SHA image reference, the same receipt image ID, and a `RepoTags` entry for that exact reference. The report digest is still verified first; the additional binding closes the evidence chain from source SHA to built image identity to the vulnerability report that is packaged for release.

Only those already-built binaries and already-generated supply-chain artifacts are copied into the bundle. The assembler does not run Cargo, Rustup, Docker builds, dependency resolution, or another SBOM generator. It therefore remains downstream of the release compiler and evidence-producing workflows instead of becoming a second build authority.

The bundle contains both release binaries, the reproducibility receipt, committed lock and dependency policy, SPDX dependency SBOM, both image scan reports, the supply-chain receipt, a sorted SHA-256 manifest, and a bundle receipt. All archive permissions are normalized before packaging: directory and binaries are `0755`; evidence, receipts, lock/policy files and checksum manifest are `0644`. This avoids inheriting transport or runner `umask` state without altering evidence bytes. The tar stream uses GNU format, the protected commit timestamp, stable ordering, and numeric ownership metadata, and `gzip -n` suppresses gzip header name/time variability instead of relying on `tar -z` defaults. The receipt deliberately states `publication_state=unpublished`.

## Rejected alternatives

Rebuilding during packaging was rejected because a later toolchain, dependency resolver, environment, or source movement could produce artifacts different from the proven candidate. Accepting arbitrary successful run IDs was rejected because success alone does not establish source or workflow identity. Automatically running the assembler on every push was rejected because evidence-producing workflows can complete in either order and a release bundle is a deliberate promotion action rather than a general CI side effect. Publishing a normal GitHub Release from this lane was rejected because repository release immutability is an administrative policy outside this workflow's current authority.

The first archive implementation also used `tar -czf` directly and copied downloaded files without normalizing complete archive metadata. Review rejected those choices before promotion: a release bundle should not depend on transport-specific permission behavior or runner `umask`, and deterministic tar metadata alone does not explicitly suppress gzip-header variability. The initial download-path assumption was also checked against an actual #91 reproducibility artifact and corrected: the binary files are stored under `release-binaries/`, while supply-chain evidence is stored at artifact root. A later exact-head review found two separate completeness defects in receipt verification. First, the broad filtered `sha256sum --check` pipeline authenticated whichever matching digest lines were present but did not require every packaged path to appear exactly once. Second, exact `grep` checks proved the expected identity line existed but did not reject a second contradictory line for the same field. The current lane rejects missing, duplicate, or contradictory receipt records before assembly.

A subsequent hostile characterization exposed a third evidence-integrity defect: a Trivy report for a different image could be substituted if the attacker or stale producer also updated the receipt's checksum line for that JSON. The old verifier would correctly authenticate the substituted file bytes while never comparing the report's own image identity with the candidate image ID already recorded by Supply Chain. The selected repair does not regenerate scans and does not move image-build authority into this lane; it verifies the existing producer-side image identity and report metadata against each other.

## Immutable-release boundary

GitHub immutable releases are the publication authority for the planned first release. Once enabled and published, they lock the associated tag and release assets and automatically generate a release attestation. GitHub recommends creating the release as a draft, attaching every asset, and publishing only after the asset set is complete (GitHub, n.d.-c). Publication therefore fails closed unless repository or organization release immutability is actually enabled. The current connector/runtime cannot read or change the repository Administration endpoint that controls immutable releases; GitHub's repository API requires `Administration` read permission to check that setting and `Administration` write permission to change it (GitHub, n.d.-d). This lane does not infer the setting from ordinary repository write access and does not create a release.

After that administrative prerequisite is verified, the publication implementation must consume this protected-source evidence bundle rather than rebuild it, bind the intended version and changelog to the same protected source, publish all assets before making the release immutable, and verify the resulting release and each local asset with `gh release verify` and `gh release verify-asset` (GitHub, n.d.-e). A published release remains downstream of independent governance, supplier release qualification, and the remaining representative NUMA evidence.

## Evidence and follow-up

PR evidence for this lane consists of the normal exact-head repository gates plus the executable structural contract in `tests/protected_release_evidence_workflow_contract.rs`. The actual bundle is post-integration evidence because `workflow_dispatch` is intentionally required to run from protected `main` and must consume successful same-SHA push artifacts.

The next release sequence is: independent governance and dependency-ordered protected integration; exact protected-main reproducibility/provenance and supply-chain push runs; this protected-source bundle; verified immutable-release administration; version/changelog/tag/package publication from the same evidence; release and asset verification; representative deployment evidence; shadow/canary; observed rollback; cutover; then verified legacy proxy removal.

## References

GitHub. (n.d.-a). *Artifact attestations*. GitHub Docs. Retrieved September 12, 2026, from https://docs.github.com/en/actions/concepts/security/artifact-attestations

GitHub. (n.d.-b). *Using artifact attestations to establish provenance for builds*. GitHub Docs. Retrieved September 12, 2026, from https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations

GitHub. (n.d.-c). *Immutable releases*. GitHub Docs. Retrieved September 12, 2026, from https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases

GitHub. (n.d.-d). *REST API endpoints for repositories*. GitHub Docs. Retrieved September 12, 2026, from https://docs.github.com/en/rest/repos/repos

GitHub. (n.d.-e). *Verifying the integrity of a release*. GitHub Docs. Retrieved September 12, 2026, from https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/secure-your-dependencies/verify-release-integrity
