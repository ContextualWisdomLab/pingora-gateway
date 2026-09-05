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

Compiler repair #56 is current exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, Ready/open/mergeable, based on foundation `0da81a93f93e869c15bb7d34c55fc87479d16522`.

Rust 1.98.1 remains the required release compiler. Release-producing CI, Supply Chain, and OCI paths install/select/verify Rust 1.98.1 before Cargo. Compiler-authority contracts reject alternate authority through YAML environment scopes, explicit Cargo toolchain selectors, shell control operators and command indirection, GNU `env` variants, command substitution, persistent Cargo aliases, Cargo compiler-wrapper variables, repository Cargo configuration, and Docker `ENV`/`ARG`/`RUN` paths.

The official Rust Docker source still declares Rust `1.98.0` in `rust-lang/docker-rust/master/versions.toml`; no reviewed official `1.98.1-bookworm` image authority has yet been adopted by this stack. The digest-pinned 1.98.0 builder therefore remains bootstrap-only, explicitly installs/selects 1.98.1 and verifies `release: 1.98.1` before compilation. Remove that bridge only after an official 1.98.1 image is published, its exact digest is reviewed/pinned, and exact-head OCI/Supply Chain evidence is reacquired.

### Hosted formatter RED and repair

Exact predecessor `5e66e334695697cee8469442161f0bfbe8368249` acquired hosted runners in CI `33982856368`: `load-contract 101351075777` succeeded; `test 101351075934` failed at `cargo fmt --all -- --check` before compile/test, strict Clippy, rustdoc, coverage or dependency-lock evidence; `oci-runtime 101351075955` acquired a runner and entered image build.

The formatter output named exactly five Rust test/support files. #56 repaired only those rustfmt layouts with ordinary fast-forward commits, reaching `8ade8894330b896cb4d5bf46ee6b8be22b2cc6ab`. Fresh CodeRabbit review of exact `0da81a93...8ade889` reported no new issue and confirmed the formatter-only subrange.

### Wrapped Cargo-alias authority

Fresh source review after the formatter repair found an acceptance-oracle gap. Persistent Cargo aliases such as `CARGO=cargo` were guarded when invoked directly as `"$CARGO" +nightly ...`, but an explicit selector could be hidden one executable layer deeper as `command "$CARGO" +nightly ...` or `env "$CARGO" +nightly ...`. The hostile wrapper forms are absent from current release workflows, so this is an acceptance-oracle gap rather than an observed production-path compiler bypass.

Initial TDD lineage was scaffold `6d6ea939648d20af60a8f5cb3d0b84b55f3982b8` → RED `052ce9c4c164b770da1063d23c95bf0effd2b9b2` → GREEN `bb07c623763da3417cfe144af508d1036c2a4054`. Fresh CodeRabbit review then found a valid High successor: `${CARGO:?}` and `${CARGO:-cargo}` still resolve the persistent Cargo alias at shell runtime but were not recognized. That was repaired RED `88aa8eb8cc966c416cb9570d4e6b6e1ebbcfc737` → GREEN `9598c65456dc4960047b811afd0356806c85df5e`.

Fresh exact review of `9598c654...` found a valid Medium false positive. `${CARGO:+word}` and `${CARGO+word}` select `word` when CARGO is set rather than preserving the Cargo value. RED `379da86fa283cbca7a5edc38ea9010a325792e47` characterized that boundary and GREEN `42810ff1af77777662e4fda00fd89e4deaafce8a` preserved Cargo identity only for complete parameter forms whose set-value branch retains the parameter: plain expansion plus default/assign/error operators (`-`, `:-`, `=`, `:=`, `?`, `:?`). Alternative-value (`+`, `:+`) and transforming forms are not guessed as Cargo execution.

### Legacy command-substitution companion consistency

The first legacy-substitution repair compared the command-substitution companion with Bash semantics. Bash supports both `$(command)` and the legacy backquote form. The repository-wide release shell contract already fails closed on active legacy backquotes, so this was not a newly demonstrated release-path or production bypass. The narrower inconsistency was that `tests/toolchain_command_substitution_contract.rs` analyzed `$()` but did not apply the same compiler-authority semantics to the legacy form.

Source RED `c3504a67af695980e373f18cd50f5ae74d42cd85` added hostile legacy bodies plus benign/literal controls. GREEN `c9b770b7ce8e24fc77e4d1692dd7da0ee044189f` extracted active legacy bodies outside single quotes and reused the existing `$()` analyzer.

Fresh source verification then found a second, narrower quote-semantics inconsistency. In Bash, backquote command substitution remains active inside double quotes, while a single-quote character occurring inside double quotes is literal. The `c9b770b7...` companion helper tracked only `single_quoted`, so source such as `"prefix '`rustup default 1.98.0`' suffix"` could toggle a false single-quoted state and skip executable backquotes in that companion.

The repair is again acceptance-oracle-only and test-first:

- source RED `6adc4c890a835d445305415ed554c2df3c1eedef` adds hostile double-quoted legacy bodies containing `rustup default 1.98.0` and `cargo +1.98.0 ...`;
- GREEN/current `18fb38b1ba70c4bf222642ef347f3d57a98379a2` adds double-quote state to the legacy scanner, so `'` toggles single-quote state only outside double quotes and active backquotes remain analyzed inside double quotes.

Fresh compare `c9b770b7...18fb38b1` is ahead 2 / behind 0 with exact merge base and changes only `tests/toolchain_command_substitution_contract.rs` (+8/-1). No workflow, production gateway source, Cargo manifest, Dockerfile, selected compiler version, routing, TLS, auth or business logic changed.

### Current exact hosted GREEN

CI `33992794787` and Supply Chain `33992794799` have now completed successfully on unchanged exact `18fb38b1...` after the earlier runnerless queue period.

- `oci-runtime 101377894560` checked out and verified the exact revision, built the candidate image, verified declared non-root identity, and exercised the read-only least-privilege runtime;
- `load-contract 101377894692` installed Rust 1.98.1, built the exact gateway candidate, installed checksum-pinned k6 2.2.0, exercised the concurrent loopback traffic contract, and uploaded latency evidence;
- `test 101377894722` passed formatting, compile/test, strict lint, public rustdoc, pinned coverage tooling, owned-production coverage workload and complete-coverage enforcement, resolved dependency-lock verification, and evidence upload;
- `candidate-evidence 101377894065` passed committed dependency-graph audit, exact candidate image build, SPDX SBOM generation, image scan, exact-source binding, and evidence upload.

CodeRabbit has reviewed exact current subrange `c9b770b7...18fb38b1` without predecessor credit and reported no new issue; returned inline review threads are resolved/outdated. This technical review does not satisfy the organization ruleset's required approving review. #56 therefore remains unmerged despite repository CI/Supply Chain GREEN; no self-approval or administrator bypass is used.

## Supply-chain RED child — #54

#54 is now Ready/open at unchanged exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, based on current #56 `18fb38b1ba70c4bf222642ef347f3d57a98379a2`.

When #56 advanced from `c9b770b7...` to current, #54 was not rebased or force-pushed. Ordinary two-parent adoption preserved predecessor child `12bb14ae174f7abed550b07bd990e765596275b6` as first parent and current #56 as second parent, then advanced with `force=false`. Fresh compare uses current #56 as exact merge base, ahead 75 / behind 0. Effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`; no parent compiler-oracle delta is duplicated or reverted.

Because #56 has independently reached exact-head hosted GREEN, #54 was moved Ready without source churn to expose the intentional supply-chain RED. New exact Ready-state runs are CI `33998449940` and Supply Chain `33998449901`. At the current read, CI jobs `load-contract 101392950922`, `oci-runtime 101392951059`, `test 101392951060` and Supply Chain `candidate-evidence 101392950711` are queued before checkout with `steps=[]` and no assigned runner. The earlier Draft-state runs were skipped and do not count as Ready-state evidence.

`#54` intentionally requires committed `Cargo.lock` to contain no package named `derivative`. Do not add an audit ignore, suppress OSV/RustSec, remove lock evidence, or consume a mutable supplier PR to manufacture GREEN. `RUSTSEC-2024-0388` is an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

Required supplier order is now `#54 exact Ready execution → derivative semantic RED → maintainer-integrated immutable supplier repair/release → gateway dependency bump → committed lock without derivative → unchanged supply-chain GREEN`. If current execution fails before reaching the intended semantic assertion, that earlier deterministic failure is a local repair finding and must be fixed first.

## Protocol / supplier path

Draft #53 remains a protocol-test lineage that must be ancestry-repaired only after the compiler/supply-chain dependency root reaches its required state. Its H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the current sweep. Cookie #901 remains open/unmerged on a stale base and is not immutable dependency authority; body-framing #936 and derivative owner issue #889 remain supplier-owner work until fresh maintainer integration evidence says otherwise. Mutable supplier work is evidence, not dependency authority.

For derivative, #889 remains open and the existing owner-path evidence identifies the minimal semantics-preserving repair surface in `PeerOptions` and `Backend`; there is no maintainer-integrated immutable supplier repair yet. After the supply-chain root is repaired, protocol order is `#52/#53 non-force ancestry repair → protocol-only H2→H1 Cookie RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora only sends exact evidence through the owner path while that writer is active.

Runner delay and semantic failure remain distinct states. #56 proves recovery on the same unchanged exact head: its jobs first sat queued without runners and later all executed successfully. #54 is now the current Ready-state specimen: its newly materialized exact jobs are queued before checkout, so no no-op source churn or runner-selector change is justified solely to retrigger them. Central scheduler/admission source remains `.github` owner authority rather than gateway-local policy.

## Legacy migration / release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Migration stays release-first: owner-safe structural inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Until fresh protected-main and release reads prove otherwise, no immutable gateway release, canary, cutover, or legacy-removal credit is assigned.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.
