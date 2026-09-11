# Release Reproducibility Traceability

## Decision status

This document records a writer-safe release-evidence increment after downstream TLS/H2 performance acceptance. It does not declare an immutable release, independent provenance, package publication, deployment, canary, cutover, or legacy-proxy removal.

## Problem

The repository already binds CI and Supply Chain evidence to exact source SHAs and builds release-mode gateway candidates, but candidate evidence alone does not show that the two shipped Rust binaries are byte-identical when rebuilt from the same exact source, lock file, compiler, runner class, and build command. A release candidate whose output changes between isolated builds cannot be promoted as reproducible evidence.

Foundation PR #56 establishes Rust 1.98.1 as the release compiler because Rust 1.98.0 contains a vtable-generation miscompilation. Until #56 is independently approved and protected-integrated, this lane verifies Rust 1.98.1 locally for its own release-evidence boundary; it does not claim that every repository workflow has inherited the foundation.

## Constraints and alternatives

The acceptance must not weaken `Cargo.lock`, select a mutable supplier branch, hide compiler overrides, reuse one target directory, or compare only metadata. Cargo `--locked` requires the committed lock file to remain unchanged. Dependencies are fetched once under the locked graph and both candidate builds run offline with incremental compilation disabled, so network movement cannot alter either build after input acquisition.

A single hosted runner performing two isolated builds proves same-platform byte reproducibility, not independent reproducibility. SLSA distinguishes ordinary reproducibility from stronger verification using independent build systems. Independent builders, signed provenance, OCI-image reproducibility, registry publication and release signing remain later release gates rather than being overstated by this workflow.

## Selected acceptance

`.github/workflows/release-reproducibility.yml` executes on pull requests and protected `main` pushes. It:

1. checks out and verifies the exact source SHA;
2. installs, selects and verifies Rust 1.98.1 before Cargo;
3. fails closed on repository or environment compiler overrides relevant to the release command;
4. fetches the committed locked dependency graph once and verifies that `Cargo.lock` did not move;
5. builds `cwl-pingora-gateway` and `cwl-pingora-pg-erd-migration` twice in distinct target directories with `CARGO_NET_OFFLINE=true` and `CARGO_INCREMENTAL=0`;
6. requires SHA-256 equality and byte-for-byte `cmp` equality for each binary;
7. uploads the exact-source binaries and a receipt identified explicitly as `unreleased-same-platform-release-binary-reproducibility`.

`tests/release_reproducibility_workflow_contract.rs` locks the compiler, isolated-build, offline/locked, binary-selection, digest-comparison, source-binding and non-claim vocabulary against workflow drift.

## Risk and interpretation

A GREEN result means that the exact candidate produced identical binary bytes in two isolated target directories on the same GitHub-hosted Ubuntu 24.04 runner using verified Rust 1.98.1 and the committed dependency lock. It does not prove that a different build platform, container base, linker/toolchain image, or later protected head produces the same bytes. It also does not establish the reproducibility of OCI image layers.

A RED result is a release-evidence defect. The response is to identify the nondeterministic input or build-path dependency and repair it; reducing the comparison to semantic equivalence, excluding changed bytes, or weakening the digest requirement is not acceptable.

## Promotion boundary

This lane may become release evidence only on its unchanged exact head together with the normal CI, Supply Chain and security gates. Protected release promotion still requires independent governance, #56 protected compiler integration, release-qualified supplier roots, representative NUMA evidence where applicable, version/CHANGELOG/tag/package identity, immutable artifacts, authenticated provenance, rollback evidence and deployment parity through shadow/canary/cutover.

`docs/product-technical-gap-baseline.md` remains owned by dedicated documentation lane #61 and is not modified by this increment. Live issue #51/#58 carries the owner-path handoff until that lane projects the current release state.

## Primary-source traceability

- Cargo documents `--locked` as asserting that the exact dependencies and versions from the existing lock file are used, and `--offline` as preventing network access during the build. The workflow fetches the locked graph before using offline release builds.
- The Rust Release Team published Rust 1.98.1 on September 3, 2026 to repair a vtable-generation miscompilation in Rust 1.98.0; this is the release-compiler reason inherited from #56.
- SLSA provenance describes verifiable information connecting an artifact to how it was produced. SLSA guidance treats rebuilds on independent build systems as a stronger verified-reproducibility property than repeated builds on one platform. This lane deliberately claims only the narrower same-platform evidence it executes.

## References

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Rust Project Developers. (n.d.). *cargo build*. The Cargo Book. Retrieved September 12, 2026, from https://doc.rust-lang.org/cargo/commands/cargo-build.html

Supply-chain Levels for Software Artifacts. (n.d.). *Provenance (v1.2)*. Retrieved September 12, 2026, from https://slsa.dev/spec/v1.2/provenance

Supply-chain Levels for Software Artifacts. (n.d.). *Frequently asked questions*. Retrieved September 12, 2026, from https://slsa.dev/spec/draft/faq
