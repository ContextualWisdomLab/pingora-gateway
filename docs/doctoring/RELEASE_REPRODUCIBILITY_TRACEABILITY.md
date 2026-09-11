# Release Reproducibility Traceability

## Decision status

This document records a writer-safe release-evidence increment after downstream TLS/H2 performance acceptance. It does not declare an immutable release, independent provenance, package publication, deployment, canary, cutover, or legacy-proxy removal.

## Problem

The repository already binds CI and Supply Chain evidence to exact source SHAs and builds release-mode gateway candidates, but candidate evidence alone does not show that the two shipped Rust binaries are byte-identical when rebuilt from the same exact source, lock file, compiler, runner class, and controlled build environment. A release candidate whose output changes between clean rebuilds cannot be promoted as reproducible evidence.

Foundation PR #56 establishes Rust 1.98.1 as the release compiler because Rust 1.98.0 contains a vtable-generation miscompilation. Until #56 is independently approved and protected-integrated, this lane verifies Rust 1.98.1 locally for its own release-evidence boundary; it does not claim that every repository workflow has inherited the foundation.

## Constraints and alternatives

The acceptance must not weaken `Cargo.lock`, select a mutable supplier branch, hide compiler overrides, retain build outputs between candidates, strip differing sections, or compare only semantic metadata. Cargo `--locked` requires the committed lock file to remain unchanged. Dependencies are fetched once under the locked graph and both candidate builds run offline with incremental compilation disabled, so network movement cannot alter either build after input acquisition.

The verified `rustup default` alone is not sufficient compiler authority. Cargo can select a compiler or wrapper through `RUSTC`, `CARGO_BUILD_RUSTC`, `RUSTC_WRAPPER`, `CARGO_BUILD_RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER`, repository or ancestor `.cargo/config` / `.cargo/config.toml`, and the Cargo home configuration. Build flags can likewise enter through `RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS`. A same-platform reproducibility receipt that leaves those authorities unconstrained could compare two byte-identical binaries produced by an unintended compiler or wrapper and therefore prove the wrong build identity.

Pingora 0.9.0 enables `pingora-openssl`, whose `openssl` dependency enables the `vendored` feature. The resolved graph therefore builds and statically links OpenSSL through `openssl-src`. `openssl-src` configures vendored OpenSSL with `--prefix` pointing inside Cargo `OUT_DIR`; consequently, deliberately using different Cargo target roots changes a supplier-owned build path that OpenSSL can embed in build metadata. OpenSSL also exposes build-date metadata unless a reproducible build environment supplies stable time input. Treating two intentionally different target roots as the same build environment would test path independence rather than clean rebuild reproducibility and would make the gateway lane own a supplier build-system property it does not control.

The selected repair therefore keeps one canonical target path per exact source SHA but deletes the entire target tree before each build. This gives two clean builds with the same path identity rather than cache reuse. The workflow also derives a non-zero `SOURCE_DATE_EPOCH` from the exact source commit timestamp, so vendored OpenSSL receives stable build-time input. Candidate A is copied out before the target tree is deleted; candidate B is built only after the same target tree has been recreated from scratch.

A single hosted runner performing two clean builds proves controlled same-platform byte reproducibility, not path-independent or independent reproducibility. SLSA distinguishes ordinary reproducibility from stronger verification using independent build systems. Independent builders, signed provenance, OCI-image reproducibility, registry publication and release signing remain later release gates rather than being overstated by this workflow.

## RED and causal repair

Exact `e0258f6e1165dd0734b94627229d2a82afdba9cf` completed exact checkout, Rust 1.98.1 selection, compiler-override rejection, locked dependency fetch, and both release builds, then failed only the byte comparison in Release Reproducibility run `34617690030`. Diagnostic artifact `release-reproducibility-diagnostics-e0258f6e1165dd0734b94627229d2a82afdba9cf` has digest `sha256:df29fd9106c1d688292fca6a920d921ffecf94ef7cd55a140c51183a62c624ab`.

The artifact isolated two deterministic input differences in both binaries rather than application-level nondeterminism:

- vendored OpenSSL strings contained candidate-specific `/tmp/cwl-pingora-repro-a-...` versus `/tmp/cwl-pingora-repro-b-...` install/module paths derived from the two Cargo target roots;
- OpenSSL build metadata recorded different wall-clock build times for candidate A and candidate B.

The binary digests therefore differed even though exact source, lock file, Rust compiler, runner and Cargo command were otherwise fixed. This is a valid release-evidence RED and not grounds to lower the equality gate.

The first causal workflow repair uses one exact-SHA canonical `REPRO_TARGET_DIR`, removes it before each build, stages candidate A outside that directory, removes/recreates the target tree, builds candidate B, and compares the separately staged binaries. It exports exact-commit-derived `SOURCE_DATE_EPOCH` to both builds. SHA-256 and byte-for-byte `cmp` remain mandatory; mismatch diagnostics remain fail-closed.

A later independent review of the exact GREEN workflow found that its compiler-authority claim was still broader than its implementation. Exact `6de6f8e163bdfb8879c3b658c700412bad2024e6` added a structural hostile-case contract for all Cargo compiler/wrapper/build-flag authorities and repository Cargo configuration. Hosted CI run `34621910946`, job `103337685745`, passed formatting and the complete pre-existing test corpus, then failed only `release_reproducibility_lane_fails_closed_on_compiler_or_artifact_drift` with `release reproducibility workflow must reject compiler/build authority RUSTC`. This is the intended RED: the workflow did not yet reject a direct compiler override even though the document and PR claimed it did.

The second causal repair extends the existing fail-closed step rather than changing the compiler, equality oracle or build topology. It rejects `RUSTC`, `CARGO_BUILD_RUSTC`, both Cargo and ordinary rustc wrapper authorities, workspace wrappers, `RUSTUP_TOOLCHAIN`, the three supported build-flag authorities, repository `.cargo/config` and `.cargo/config.toml`, plus the existing rust-toolchain files. The hostile-case contract enumerates the same authorities, so future relaxation is a test failure rather than silent release-evidence drift.

A further current-head audit found that repository-root checks alone still did not match Cargo's configuration hierarchy. Cargo searches `.cargo/config.toml` or `.cargo/config` from the invocation directory through its ancestors and also uses Cargo-home configuration. Exact `da504f42ac4f8f77ea943609e10f1c26102c49cd` encoded that missing boundary in the structural contract, but its CI `34623637928` stopped at Rust 1.98.0 formatting before the semantic assertion executed. The finding is nevertheless valid from Cargo's documented lookup rules. The causal workflow repair now rejects externally supplied `CARGO_HOME`, walks from `$PWD` to `/` and rejects either Cargo configuration filename at every level, then creates a clean exact-SHA-scoped Cargo home under `$RUNNER_TEMP` and exports it before dependency fetch and both offline release builds. The equality oracle, compiler version, target-path strategy, and binary contents are not normalized or weakened by this repair.

## Selected acceptance

`.github/workflows/release-reproducibility.yml` executes on pull requests and protected `main` pushes. It:

1. checks out and verifies the exact source SHA;
2. installs, selects and verifies Rust 1.98.1 before Cargo;
3. fails closed on environment, ancestor/repository Cargo configuration, Cargo-home, build-flag and toolchain-file authorities that can alter the release compiler, wrapper or flags, then establishes one clean exact-SHA-scoped Cargo home;
4. fetches the committed locked dependency graph once and verifies that `Cargo.lock` did not move;
5. derives a non-zero `SOURCE_DATE_EPOCH` from the exact source commit and binds one exact-SHA canonical target path;
6. removes the canonical target tree, builds both release binaries offline/non-incrementally, and stages candidate A outside the target tree;
7. removes the target tree again and repeats the same build in the same canonical path to stage candidate B;
8. requires SHA-256 equality and byte-for-byte `cmp` equality for each binary;
9. uploads the exact-source binaries and a receipt identified explicitly as `unreleased-same-platform-release-binary-reproducibility` only on equality, while preserving bounded diagnostics on mismatch.

`tests/release_reproducibility_workflow_contract.rs` locks the compiler, compiler/wrapper/config authorities, stable source-time input, clean target teardown, canonical-path reuse, offline/locked inputs, binary selection, digest comparison, source binding and non-claim vocabulary against workflow drift.

## Risk and interpretation

A GREEN result means that the exact candidate produced identical binary bytes in two clean builds on the same GitHub-hosted Ubuntu 24.04 runner using verified Rust 1.98.1, the committed dependency lock, the exact source timestamp, the same canonical target path, an isolated Cargo home, and no admitted alternate Cargo compiler/wrapper/build-flag or ancestor configuration authority. It does not prove that different build roots, build platforms, container bases, linkers/toolchain images, or later protected heads produce the same bytes. It also does not establish OCI-layer reproducibility.

A RED result is a release-evidence defect. The response is to identify the nondeterministic or uncontrolled input and repair it; reducing the comparison to semantic equivalence, excluding changed bytes, normalizing binaries after the build, or weakening the digest requirement is not acceptable.

## Promotion boundary

This lane may become release evidence only on its unchanged exact head together with the normal CI, Supply Chain and security gates. Protected release promotion still requires independent governance, #56 protected compiler integration, release-qualified supplier roots, representative NUMA evidence where applicable, version/CHANGELOG/tag/package identity, immutable artifacts, authenticated provenance, rollback evidence and deployment parity through shadow/canary/cutover.

`docs/product-technical-gap-baseline.md` remains owned by dedicated documentation lane #61 and is not modified by this increment. Live issue #51/#58 carries the owner-path handoff until that lane projects the current release state.

## Primary-source traceability

- Cargo documents `--locked` as asserting that the exact dependencies and versions from the existing lock file are used, and `--offline` as preventing network access during the build. The workflow fetches the locked graph before using offline release builds.
- Cargo configuration documents `build.rustc`, `build.rustc-wrapper`, `build.rustc-workspace-wrapper` and `build.rustflags`, together with their environment-variable authorities. Cargo also searches `.cargo/config.toml` or `.cargo/config` from the current directory through ancestor directories and consults Cargo-home configuration. The release lane rejects inherited ancestor configuration and supplies its own empty exact-SHA-scoped Cargo home instead of relying on runner state.
- Pingora 0.9.0's OpenSSL feature reaches `pingora-openssl`, which selects rust-openssl's vendored OpenSSL build. The resolved gateway lock contains `openssl-src 300.6.1+3.6.3` and `openssl-sys 0.9.117`.
- `openssl-src` constructs its install prefix inside Cargo `OUT_DIR` and passes that path as OpenSSL `--prefix`; changing Cargo target roots therefore changes supplier build-path input.
- OpenSSL documents `OPENSSL_BUILT_ON` as build-date metadata and notes that the date may be unavailable in a reproducible build. The OpenSSL build system recognizes `SOURCE_DATE_EPOCH` as reproducible-build time input; this lane supplies the exact commit timestamp rather than wall-clock time.
- The Rust Release Team published Rust 1.98.1 on September 3, 2026 to repair a vtable-generation miscompilation in Rust 1.98.0; this is the release-compiler reason inherited from #56.
- SLSA provenance describes verifiable information connecting an artifact to how it was produced. SLSA guidance treats rebuilds on independent build systems as a stronger verified-reproducibility property than repeated builds on one controlled platform. This lane deliberately claims only the narrower evidence it executes.

## References

OpenSSL Project Authors. (2026). *OpenSSL_version*. OpenSSL Documentation. https://docs.openssl.org/master/man3/OpenSSL_version/

OpenSSL Project Authors. (2026). *openssl-version*. OpenSSL Documentation. https://docs.openssl.org/master/man1/openssl-version/

Rust OpenSSL Developers. (n.d.). *rust-openssl: OpenSSL bindings for Rust*. GitHub. https://github.com/sfackler/rust-openssl

Rust OpenSSL Developers. (n.d.). *openssl-src-rs*. GitHub. https://github.com/alexcrichton/openssl-src-rs

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Rust Project Developers. (n.d.). *cargo build*. The Cargo Book. Retrieved September 12, 2026, from https://doc.rust-lang.org/cargo/commands/cargo-build.html

Rust Project Developers. (n.d.). *Configuration*. The Cargo Book. Retrieved September 12, 2026, from https://doc.rust-lang.org/cargo/reference/config.html

Supply-chain Levels for Software Artifacts. (n.d.). *Provenance (v1.2)*. Retrieved September 12, 2026, from https://slsa.dev/spec/v1.2/provenance

Supply-chain Levels for Software Artifacts. (n.d.). *Frequently asked questions*. Retrieved September 12, 2026, from https://slsa.dev/spec/draft/faq
