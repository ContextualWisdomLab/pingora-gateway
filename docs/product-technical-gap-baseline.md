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

PR #59 predecessor `cb458621b5cfdcec35820083bb82e19e9dc627cf` produced hosted semantic RED: the foundation lacked the required PR admission/cancellation event contract and did not restrict duplicate push evidence to protected `main`. GREEN child #60 exact `e2309106eba4f41f45c770676885b27bf73aa69b` retained only the two workflow files and reached terminal CI and Supply Chain success. Combined #59 exact `cf60f0bce57a8ac530e8fff52fa9ae00be232f07` then reached terminal exact-head GREEN and was normally merged into foundation. No force update, destructive rebase, bypass, self-approval, or predecessor-success synthesis was used.

Foundation now owns `main`-only push scope, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts.

## Compiler root — #56

Compiler repair #56 is current exact `8ade8894330b896cb4d5bf46ee6b8be22b2cc6ab`, Ready/open, based on foundation `0da81a93f93e869c15bb7d34c55fc87479d16522`.

Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair the vtable-generation miscompilation introduced in 1.98.0. Release-producing CI, Supply Chain, and OCI paths install/select/verify Rust 1.98.1 before Cargo. The compiler-authority contracts reject alternate authority through YAML environment scopes, explicit Cargo toolchain selectors, shell control operators and command indirection, GNU `env` variants, command substitution, persistent Cargo aliases, Cargo compiler-wrapper variables, repository Cargo configuration, and Docker `ENV`/`ARG`/`RUN` paths.

The wrapper/Docker findings are source acceptance-gap repairs rather than hosted RED because live release paths did not contain wrapper overrides. Prior exact CodeRabbit review of `0da81a93...5e66e334` reported no new compiler-authority finding and confirmed the Docker repair closed the earlier wrapper-authority gap.

### Hosted formatter RED and repair

Exact predecessor `5e66e334695697cee8469442161f0bfbe8368249` eventually acquired hosted runners in CI `33982856368`. This resolved the earlier classification ambiguity between pre-checkout queue delay and leaf source failure:

- `load-contract 101351075777` completed success after exact checkout, Rust 1.98.1 installation, gateway candidate build, concurrent loopback traffic and evidence upload.
- `test 101351075934` failed at the first gate, `cargo fmt --all -- --check`, before compile/test, Clippy, rustdoc, coverage or dependency-lock evidence.
- `oci-runtime 101351075955` had acquired a runner and entered candidate image build.

The formatter output named exactly five Rust test/support files. #56 repaired only those rustfmt layouts with ordinary fast-forward commits and no workflow, production source, manifest, Dockerfile, policy, or intended semantic change. Fresh compare from hosted RED `5e66e334...` to current `8ade889...` uses the RED head as exact merge base and changes only:

- `tests/compiler_wrapper_authority_contract.rs`
- `tests/support/toolchain_command_substitution.rs`
- `tests/toolchain_command_substitution_contract.rs`
- `tests/toolchain_contract.rs`
- `tests/toolchain_shell_control_contract.rs`

Current exact validation runs are CI `33987097739` and Supply Chain `33987097697`. At the latest read their jobs are materialized but still pre-checkout queued with `steps=[]` and `runner_id=0`; current exact-head GREEN is therefore not credited. A fresh CodeRabbit exact-head review was requested for `0da81a93...8ade889`, explicitly separating the formatter-only range `5e66e334...8ade889` from predecessor review credit.

Do not merge #56 until the current unchanged head proves formatting, compile/test, strict Clippy, rustdoc, owned-production coverage, dependency-lock evidence, load, OCI and Supply Chain together with then-live governance.

## Supply-chain RED child — #54

Draft #54 is current exact `fd3761af34f51ff625bec79cfa9b1604ca3b76da`, based on current #56 `8ade8894330b896cb4d5bf46ee6b8be22b2cc6ab`.

When #56 advanced through the formatter repair, #54 was not rebased or force-pushed. An ordinary two-parent adoption preserved predecessor #54 and current #56, then the branch ref advanced with `force=false`. Fresh compare uses current #56 as exact merge base, ahead 68 / behind 0. Effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. The parent formatter repair is inherited and does not appear in the child range.

`#54` intentionally requires committed `Cargo.lock` to contain no package named `derivative`. That assertion becomes semantic RED only after #56 independently reaches exact-head compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, remove lock evidence, or consume a mutable supplier PR to manufacture GREEN. `RUSTSEC-2024-0388` is an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

Required supplier order remains `#56 exact GREEN → #54 Ready without source churn → derivative semantic RED → immutable supplier repair/release → gateway dependency bump → unchanged supply-chain GREEN`.

## Protocol / supplier path

Draft #53 remains a protocol test lineage that must be ancestry-repaired now that workflow policy is canonical in foundation. Its H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before supplier RED/GREEN, merge, or release credit. Foundation workflow files must not remain as accidental protocol ownership.

Protected public `cloudflare/pingora/main` was `09696b51bc59315353d96686355861604d0bb48c` at the last verified sweep. Cookie repair #901 remained open/unmerged and dirty against a stale base, so it is not dependency authority. Body-framing #936 remained open/unmerged. Supplier issue #889 remained open and continued to track removal of unmaintained `derivative 2.2.0`. Mutable supplier work is not consumed as dependency authority.

After the supply-chain root is repaired, protocol order is `#52/#53 non-force ancestry repair → protocol-only H2→H1 Cookie RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora only sends exact evidence through the owner path while that writer is active.

Runner delay and semantic failure are distinct states. The #56 predecessor proved this boundary directly: jobs first remained materialized with no runner/steps, then later acquired hosted runners, after which the leaf `cargo fmt --check` failure became observable. Current exact #56 reruns are again pre-checkout queued, so no no-op source churn or runner-selector change is justified solely to retrigger them.

Evidence classification therefore distinguishes startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Migration stays release-first: owner-safe structural inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody, product identity/authorization, or unbounded application-specific FastCGI/business semantics merely because a legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.
