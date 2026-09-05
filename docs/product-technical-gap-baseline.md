# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance.

Production host/SNI/routing parity, realistic TLS/H1.1/H2/H3/WebSocket/streaming failure traffic, timeout/retry/backpressure behavior, client-IP/header/cookie/body-limit parity, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. No immutable gateway release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation — promoted

Workflow-policy RED→GREEN is integrated into foundation #1 exact `0da81a93f93e869c15bb7d34c55fc87479d16522`.

PR #59 predecessor `cb458621b5cfdcec35820083bb82e19e9dc627cf` produced hosted semantic RED. GREEN child #60 exact `e2309106eba4f41f45c770676885b27bf73aa69b` retained only the two workflow files and reached terminal CI and Supply Chain success. Combined #59 exact `cf60f0bce57a8ac530e8fff52fa9ae00be232f07` then reached terminal exact-head GREEN and was normally merged into foundation. No force update, destructive rebase, bypass, self-approval, or predecessor-success synthesis was used.

Foundation now owns `main`-only push scope, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts.

## Compiler root — #56

Compiler repair #56 is current exact `9598c65456dc4960047b811afd0356806c85df5e`, Ready/open, based on foundation `0da81a93f93e869c15bb7d34c55fc87479d16522`.

Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair the vtable-generation miscompilation introduced in 1.98.0. Release-producing CI, Supply Chain, and OCI paths install/select/verify Rust 1.98.1 before Cargo. The compiler-authority contracts reject alternate authority through YAML environment scopes, explicit Cargo toolchain selectors, shell control operators and command indirection, GNU `env` variants, command substitution, persistent Cargo aliases, Cargo compiler-wrapper variables, repository Cargo configuration, and Docker `ENV`/`ARG`/`RUN` paths.

The official Rust Docker source still declares Rust `1.98.0` in `rust-lang/docker-rust/master/versions.toml`; no reviewed official `1.98.1-bookworm` image authority is available. The digest-pinned 1.98.0 builder therefore remains bootstrap-only, explicitly installs/selects 1.98.1 and verifies `release: 1.98.1` before compilation. Remove that bridge only after an official 1.98.1 image is published, its exact digest is reviewed/pinned, and exact-head OCI/Supply Chain evidence is reacquired.

### Hosted formatter RED and repair

Exact predecessor `5e66e334695697cee8469442161f0bfbe8368249` acquired hosted runners in CI `33982856368`: `load-contract 101351075777` succeeded; `test 101351075934` failed at `cargo fmt --all -- --check` before compile/test, strict Clippy, rustdoc, coverage or dependency-lock evidence; `oci-runtime 101351075955` acquired a runner and entered image build.

The formatter output named exactly five Rust test/support files. #56 repaired only those rustfmt layouts with ordinary fast-forward commits, reaching `8ade8894330b896cb4d5bf46ee6b8be22b2cc6ab`. Fresh CodeRabbit review of exact `0da81a93...8ade889` reported no new issue and confirmed the formatter-only subrange.

### Wrapped Cargo-alias authority RED → review RED → GREEN

Fresh source review after the formatter repair found a separate acceptance-oracle gap. Persistent Cargo aliases such as `CARGO=cargo` were guarded when invoked directly as `"$CARGO" +nightly ...`, but an explicit selector could be hidden one executable layer deeper as `command "$CARGO" +nightly ...` or `env "$CARGO" +nightly ...`. Existing contracts understood direct `command cargo`, GNU `env` compiler-variable prefixes, and direct aliases independently but did not compose those paths. This is a source acceptance gap, not an observed release-workflow bypass: the hostile wrapper form is absent from current release workflows.

Initial TDD lineage:

- `6d6ea939648d20af60a8f5cb3d0b84b55f3982b8` scaffolds persistent Cargo-alias tracking without wrapper resolution;
- RED `052ce9c4c164b770da1063d23c95bf0effd2b9b2` requires `command`/`command -p --` and GNU `env`/`env --`/`env -i` wrapper paths to reject alias-based explicit `+toolchain` selectors while admitting ordinary alias execution without a selector;
- GREEN `bb07c623763da3417cfe144af508d1036c2a4054` resolves the executable command position through bounded POSIX `command` and GNU `env` option/assignment prefixes before applying the persistent-alias selector rule.

Fresh CodeRabbit exact-range review of `bb07c623...` found a valid High successor: shell command words `${CARGO:?}` and `${CARGO:-cargo}` still resolve the persistent Cargo alias at runtime, but the parser recognized only `$CARGO` and plain `${CARGO}`. Thus `command "${CARGO:?}" +...` or `env -- "${CARGO:-cargo}" +...` could still hide explicit toolchain selection.

The review finding is repaired test-first:

- RED `88aa8eb8cc966c416cb9570d4e6b6e1ebbcfc737` adds rejection regressions for `${CARGO:?}`, `${CARGO:?message}`, and `${CARGO:-cargo}` through both `command` and GNU `env`, plus no-selector controls;
- GREEN/current `9598c65456dc4960047b811afd0356806c85df5e` parses the parameter identity before shell parameter operators while requiring the closing `}` to terminate the command word. The operator word is not evaluated; the contract only preserves the persistent alias identity for the following explicit `+toolchain` decision.

Query-only or grammar-changing/unknown wrapper options are not guessed. `env -S/--split-string` and compiler-variable prefixes remain owned by existing dedicated contracts. These wrapped-alias repairs change only Rust tests/support; no workflow, production gateway source, Cargo manifest, Dockerfile, selected compiler version, routing, TLS, auth or business logic changed.

Current exact validation runs are CI `33988645358` and Supply Chain `33988645352`. At the latest read `oci-runtime 101366752043`, `load-contract 101366752218`, `test 101366752229`, and `candidate-evidence 101366752096` are materialized but pre-checkout queued with `steps=[]` and no runner identity; current exact-head GREEN is not credited. Fresh CodeRabbit exact-range re-review is requested for `9598c654...`; predecessor review does not transfer.

Do not merge #56 until the current unchanged head proves formatting, compile/test, strict Clippy, rustdoc, owned-production coverage, dependency-lock evidence, load, OCI, Supply Chain, current technical review, and then-live governance.

## Supply-chain RED child — #54

Draft #54 is current exact `96c2984f5a75194df4f9c09c439081d4e8c00044`, based on current #56 `9598c65456dc4960047b811afd0356806c85df5e`.

When #56 advanced through the review repair, #54 was not rebased or force-pushed. Ordinary two-parent adoption preserved predecessor #54 `073f59e24bae3a21fae8ba90924d185c00808fc7` and current #56, then the branch ref advanced with `force=false`. Fresh compare uses current #56 as exact merge base, ahead 70 / behind 0. Effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. Parent compiler-authority repairs are inherited and do not appear in the child range.

`#54` intentionally requires committed `Cargo.lock` to contain no package named `derivative`. That assertion becomes semantic RED only after #56 independently reaches exact-head compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, remove lock evidence, or consume a mutable supplier PR to manufacture GREEN. `RUSTSEC-2024-0388` is an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

Required supplier order remains `#56 exact GREEN → #54 Ready without source churn → derivative semantic RED → immutable supplier repair/release → gateway dependency bump → unchanged supply-chain GREEN`.

## Protocol / supplier path

Draft #53 remains a protocol-test lineage that must be ancestry-repaired only after the compiler/supply-chain dependency root reaches its required state. Its H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before supplier RED/GREEN, merge, or release credit. Foundation workflow files must not remain as accidental protocol ownership.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the current sweep. Cookie #901 remains open/unmerged and non-mergeable on stale base; body-framing #936 remains open/unmerged but mergeable on current protected main; derivative owner issue #889 remains open. Mutable supplier work is evidence, not dependency authority.

After the supply-chain root is repaired, protocol order is `#52/#53 non-force ancestry repair → protocol-only H2→H1 Cookie RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora only sends exact evidence through the owner path while that writer is active.

Protected `.github/main` is current `f2f91b806122ed233e3a0e2a325246077c2e15e4`. Queue-health #1150 remains `e6622a428060194b558929ad651d5b4ae3a9840f`; fresh compare against protected main is diverged, ahead 69 / behind 19, merge base `6d7fbebec8aec31d88a30a36e71ca5b3925d241d`, while its PR body still describes older reconciliation authority. The existing `.github#712` Pingora specimen has been updated in place; this lane does not mutate `.github` source/refs/PR state.

Runner delay and semantic failure are distinct states. The #56 predecessor proved this directly: jobs first remained materialized with no runner/steps, then later acquired hosted runners, after which the leaf `cargo fmt --check` failure became observable. Current exact #56 runs are again pre-checkout queued, so no no-op source churn or runner-selector change is justified solely to retrigger them.

Evidence classification distinguishes startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level acceptance RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Migration stays release-first: owner-safe structural inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody, product identity/authorization, or unbounded application-specific FastCGI/business semantics merely because a legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`, and GitHub Releases remains empty at the current sweep. Current immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.
