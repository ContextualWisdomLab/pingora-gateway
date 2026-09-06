# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Current execution update — #62 load RED and #63 repair

Ready supplier-semantics characterization #62 remains exact `d4a54d8d1337797c3763c3d915c674cdf483db1c`, but its current hosted execution is no longer runnerless. CI `34024689629` executed on that unchanged head. `test 101463462339` is terminal GREEN through exact checkout, Rust 1.98.1, formatting, compile/test, strict Clippy, public rustdoc, pinned coverage tooling, owned-production coverage workload, 100% line/region enforcement, and dependency-lock evidence. `oci-runtime 101463462432` is terminal GREEN under the exact non-root/read-only least-privilege image contract.

`load-contract 101463462445` is a distinct hosted traffic RED. The exact release candidate and checksum-pinned k6 2.2.0 built successfully, but the unchanged 400-request / 4-VU loopback sample produced 395 successful requests and 5 failures. Immutable artifact `9987411966`, digest `sha256:460b1ece2aeeaea5df49d36e37afb081bcd7a4964b987870df62d17d6a2266d5`, records 395/5 pass/fail for both HTTP-200 and body-identity checks, checks rate `0.9875`, `http_req_failed=0.0125`, and `http_req_duration p(95)=1.0152792 ms`. The p95 `<20 ms` threshold passed; zero-failure correctness thresholds did not. This result is therefore not a latency regression and must not be repaired by reducing samples or weakening correctness gates.

The inherited load workflow starts `tests/load/upstream_fixture.py` asynchronously and proves only gateway `/livez` before measured application traffic. It does not prove the test-only origin on `127.0.0.1:18081` has bound. Focused Ready sibling #63 is exact `190cd6aebc0b82b01f04b01ff095a741f831a0c8`, direct child of #56 exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, ahead 1 / behind 0 with that exact merge base and only `.github/workflows/ci.yml` changed (+14/-0). Before gateway admission/k6 it now proves the origin process is alive and directly reachable. That direct probe establishes fixture liveness; it does not pre-exercise the measured gateway application route. The 400-request / 4-VU sample, zero-failure requirements, p95 `<20 ms`, runtime, dependency graph, compiler, Pingora source, and routing semantics remain unchanged.

Exact #63 CI `34027297080` and Supply Chain `34027296921` have materialized. They are the causal discriminator rather than evidence to transfer: a zero-failure GREEN supports the origin-startup-race diagnosis; a repeated RED requires continued runtime/load RCA. Once #63 is exact-head GREEN, #62 must adopt the valid workflow delta by ordinary non-force ancestry and re-prove its supplier characterization on the new exact head. No predecessor GREEN is transferred.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance. Current #62 additionally proves that p95 can remain well below 20 ms while correctness still fails, so zero-failure assertions remain independent release gates rather than being inferred from latency.

Production host/SNI/routing parity, realistic TLS/H1.1/H2/H3/WebSocket/streaming failure traffic, timeout/retry/backpressure behavior, client-IP/header/cookie/body-limit parity, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. No immutable gateway release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation

Workflow-policy RED→GREEN is integrated into foundation #1 exact `0da81a93f93e869c15bb7d34c55fc87479d16522`. Foundation owns `main`-only push scope, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts. No force update, destructive rebase, bypass, or self-approval was used to obtain the foundation state.

## Compiler prerequisite — #56

Ready #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, based on foundation. Rust 1.98.1 is the release compiler; CI, Supply Chain, and OCI paths install/select/verify it before Cargo and reject later compiler/toolchain rebinding through workflow environment, Cargo selectors/config/wrappers, shell indirection, and Docker build instructions.

Exact CI `33992794787` and Supply Chain `33992794799` completed terminal GREEN on this head. The CI lanes cover OCI non-root/read-only runtime, the loopback k6 contract, formatting, compile/test, strict Clippy, public rustdoc, owned-production coverage, and dependency-lock evidence. Supply Chain covers committed dependency audit, exact candidate image, SPDX SBOM, image scan, and exact-source binding.

CodeRabbit reviewed the final narrow compiler-oracle subrange and found no new issue. This does not replace the organization-required independent approving review. #56 remains unmerged; self-approval and administrator bypass are not used.

## Supplier-intake RED — #54

Ready #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, with #56 as exact base. Its effective child delta remains `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`.

The intended hosted RED is proven. CI `33998449940` has GREEN `load-contract 101392950922`, GREEN `oci-runtime 101392951059`, and failing `test 101392951060`. The failing test job passed exact checkout, native dependencies, Rust 1.98.1 and formatting, then failed at `Compile and test`. The effective regression `rustsec_2024_0388_dependency_is_absent_from_committed_lock` requires no exact `derivative` package in committed `Cargo.lock`; the pinned graph still contains `derivative 2.2.0`.

Supply Chain `33998449901` / `candidate-evidence 101392950711` is GREEN on the same SHA through `cargo deny`, exact image build, SPDX SBOM, image scan and exact-source binding. This is not contradictory: the general audit lane permits transitive unmaintained crates under current policy, while #54 is the stricter CWL supplier-intake release criterion. `RUSTSEC-2024-0388` is an unmaintained advisory, not a memory-safety-CVE claim.

Do not manufacture GREEN with an audit ignore, deleted lock evidence, scanner suppression, muted test, or mutable supplier pin. The causal repair is a maintainer-integrated immutable supplier revision/release that removes `derivative`, followed by an exact gateway supplier bump and lock regeneration.

## Supplier-semantics characterization — #62

Ready #62 is a direct child of #56 and does not carry #54's intentionally failing dependency-absence oracle. Current exact head is `d4a54d8d1337797c3763c3d915c674cdf483db1c`; effective scope remains exactly `tests/pingora_supplier_semantics_contract.rs` and `TEST_STRATEGY.md`. The Pingora revision, `Cargo.lock`, production gateway source, workflows, routing/TLS/auth/business logic, and consumer state are unchanged.

Pinned supplier `cloudflare/pingora@09696b51bc59315353d96686355861604d0bb48c` uses `derivative` for `PeerOptions` Debug and for `Backend` Clone/Hash/Eq/Ord/Debug semantics. The downstream `PeerOptions` contract requires all 26 unconditional non-hook fields in the OpenSSL build (`bind_to` through `custom_l4`) to remain represented in Debug output while `upstream_tcp_sock_tweak_hook`, `proxy_digest_user_data_hook`, and `upstream_tls_handshake_complete_hook` remain absent.

Four acceptance-oracle defects are explicit and test-first:

- RED `f3ff0e94534d07a499ff27862800530f95034d16` → GREEN `79e53796e4a1acaf786fd151ab423ab7f3d9d924`: raw substring matching could let `connection_timeout` pass merely because `total_connection_timeout` was present.
- RED `eb9bb65ebf39b54a238a11f198a4a015121daa42` → GREEN `b3d3abaec2c7c0ff9fc097f085f17eba44ae9640`: field identity must not depend on incidental Debug whitespace or layout.
- Review at `2cbaaeff2fe89416f4698098ebd1c95e461ef4e8` found that the structural matcher still accepted nested or quoted field-like text because it did not track structural depth. RED `8f48cbab2f55a9a5764365cc6df74942e26581c1` adds nested and quoted false-positive controls; GREEN `281a1fd4f55864e235c1e8a63306b940c14dfdf6` tracks outer brace/bracket/parenthesis depth plus quoted/escaped regions.
- Fresh source inspection found that field-name presence still did not prove value semantics: a replacement could retain all expected names while reporting stale or fabricated values. RED `7001f0d712e9ddd7dd3eaf8245042e618b71f07d` adds a deliberately presence-only scalar matcher plus a wrong-value negative control and live configured-value acceptance. GREEN `67e4c6f65665dea3caf0383644c96eee9b7bfa9f` requires the exact top-level scalar value token; `82566e02...` records the invariant in `TEST_STRATEGY.md`. The live contract mutates `verify_cert`, `verify_hostname`, `max_h2_streams`, `allow_h1_response_invalid_content_length`, `second_keyshare`, and `tcp_fast_open` and requires Debug to reflect those configured values.

Predecessor exact `8027ebfbd4c9f020783e1bb5d4af5c4d0f6b4ca5` completed terminal CI `34013844984` and Supply Chain `34013844913` GREEN. Exact technical review later inspected the scalar-value delta `8027ebfb...82566e02` against the pinned supplier source and reported no issue; that evidence is historical after the next source commit and is not transferred as current-head review credit.

Hosted execution of exact `82566e02a03eedad2e75b0f7b9468ebd70545165` eventually acquired runner `1001709001`. Exact checkout and Rust 1.98.1 verification passed, but CI `34021912038` / `test 101455926617` then failed at `cargo fmt --all -- --check` before compile/test, lint, rustdoc, coverage, or lock-evidence. The job log's only source diff was Rustfmt collapsing the hostile wrong-value `debug_has_scalar_field_value("PeerOptions { verify_cert: true }", "verify_cert", "false")` call to one line.

Current exact `d4a54d8d1337797c3763c3d915c674cdf483db1c` is the minimal causal formatting repair. Its supplier-semantics source has now proved hosted GREEN in `test 101463462339`, including formatting, compile/test, strict lint, rustdoc, 100% owned-production line/region coverage, and lock verification. `oci-runtime 101463462432` is also GREEN. The sibling load-contract RED described above is a harness/runtime-evidence finding, not a failure of the `PeerOptions` characterization itself. Fresh exact-range CodeRabbit review covers `8027ebfb...d4a54d8d` with no actionable comments; this remains technical review evidence rather than a human `APPROVED` review.

Supply Chain `34024689818` / `candidate-evidence 101463497940` has acquired a runner on exact `d4a54d8d...`; terminal supplier evidence is not claimed until that job completes. #62 also cannot be promoted by transferring #63 evidence; after #63 GREEN it must adopt the workflow repair non-destructively and execute again on the resulting exact head.

The generic gateway does not enable Pingora load balancing merely to instantiate `Backend` for a supplier test. `Backend` address+weight equality/hash/order with opaque `Extensions` excluded remains an upstream-owner acceptance item until that bounded capability is actually consumed downstream.

## Load-contract reliability repair — #63

Ready #63 exists specifically because the full exact #62 traffic execution exposed a correctness RED while latency remained GREEN. Its exact head `190cd6aebc0b82b01f04b01ff095a741f831a0c8` changes only `.github/workflows/ci.yml` from #56. The repair adds explicit test-origin readiness admission before gateway liveness and measured k6 traffic. The origin process must stay alive and answer directly; otherwise the job fails before any measured gateway sample is taken.

This repair does not change the product route, gateway candidate, Python fixture implementation, p95 threshold, request/check failure thresholds, VU count, iteration count, Pingora pin, Rust source, or compiler. Direct fixture liveness is not credited as gateway traffic or performance. Exact CI `34027297080` and Supply Chain `34027296921` are the current evidence wave. If zero failures are not restored, #63 remains RED and the next causal finding must come from the exact logs/artifact rather than from a blind rerun or threshold relaxation.

## Supplier owner path

Protected `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open, and the latest open-PR search finds no maintainer-integrated `derivative`-removal candidate. Latest published Pingora release remains 0.8.1 from 2026-06-04; no newer immutable supplier release resolves this dependency root.

Existing #889 downstream evidence is maintained in place. The minimal owner repair remains: replace `PeerOptions` macro Debug with a manual/std Debug implementation preserving every current non-hook top-level field, its actual configured value, and hook omissions under the same feature gates; replace `Backend` macro equality/hash/order with std/manual traits over address+weight only, with canonical `PartialOrd = Some(self.cmp(other))` and opaque `Extensions` excluded; remove `derivative` from workspace/core/load-balancing manifests; regenerate the lockfile; then prove supplier fmt/tests/Clippy/rustdoc/audit GREEN.

A direct attempt from this automation identity to create an upstream repair branch at exact protected supplier head was rejected by GitHub with `403 Resource not accessible by integration`. That makes the remaining supplier source mutation an external maintainer-permission boundary for this lane; it does not authorize a mutable fork/PR pin or advisory suppression. Only a maintainer-integrated immutable supplier revision/release is downstream dependency authority.

## Protocol path

Draft #52/#53 remain behind the supplier root. They must be repaired by ordinary non-force ancestry adoption/restack after the supply-chain dependency root is satisfied. The H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before protocol RED/GREEN, merge, or release credit.

Mutable supplier Cookie/body-framing work is evidence only until current-main maintainer integration produces immutable authority. After supplier repair, protocol order is `#52/#53 non-force ancestry repair → H2→H1 Cookie/body-framing traffic RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora sends exact evidence through the owner path without modifying central source from this lane.

Protected `.github/main` was last verified at `43024633eba9d96b0456970391360da5a171fbda` before the final exit sweep. Current `scripts/ci/pingora_edge_policy.py` on that authority still makes `_needs_content_scan()` return true for changed Dockerfile/Containerfile/compose and common config/service/script candidates, after which `evaluate_pull_request()` loads the final exact-head file content and applies `scan_content()`. The static-only Nginx ownership contradiction therefore remains live unless a later fresh owner sweep proves otherwise.

Owner handoff `.github#1952` remains the canonical bounded-context repair. Active `.github#1946` currently changes the same four central files required by #1952 (`scripts/ci/pingora_edge_policy.py`, `tests/test_pingora_edge_policy.py`, `docs/policies/PINGORA_EDGE_POLICY.md`, `CHANGELOG.md`) for a distinct oversized-Contents/Git-Blobs repair. The Pingora lane has therefore handed off a single-writer sequence: finish/reconcile #1946, then adopt its protected-main result and add #1952's static-only-vs-edge-runtime RED→GREEN without parallel source branches, destructive rebase, or loss of either repair.

Queue-health owner evidence is also maintained centrally. #62's prior runnerless states later acquired runners and exposed real leaf findings; current #63 may likewise wait for admission without justifying source no-op retriggers. Runner delay, formatting failure, semantic failure, and traffic correctness failure remain distinct and must not be collapsed into one scheduler diagnosis.

## Legacy migration and release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Nginx/OpenResty presence alone is not enough to move a workload into `pingora-gateway`: the responsibility must be shared edge routing/TLS/HTTP/load-balancing/runtime policy rather than static-file serving, certificate issuance, FastCGI, product auth, or business logic.

Migration remains release-first: owner-safe inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified legacy removal.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. No release, canary, cutover, or legacy-removal credit is assigned until fresh protected-main and release evidence proves those gates.

## Current causal order

Primary execution root: `#62 hosted load RED → #63 exact load-harness RED→GREEN → ordinary non-force adoption into #62 → re-prove #62 exact test/load/OCI/Supply Chain`.

Primary supplier root after that local repair: `#54 hosted derivative RED + preserved #62 supplier semantics → maintainer-integrated immutable derivative repair → gateway supplier bump + committed lock regeneration → unchanged #54 absence regression GREEN + preserved #62 semantics/load GREEN → exact CI/Supply Chain/security/runtime GREEN → #56 independent approval/governance → foundation integration as applicable → #52/#53 non-force ancestry repair → protocol traffic RED/GREEN → immutable release → parity/shadow/canary/rollback/cutover → verified Nginx/OpenResty removal`.

Parallel owner path: `.github#1952` must reconcile the organization-required Nginx/Pingora scanner with the same bounded-context migration rule after adopting overlapping #1946 work; `.github#712/#1150` owns runner-health diagnosis/reconciliation. Neither path grants the Pingora writer permission to modify `.github` source/refs or relax true edge-runtime enforcement.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps current ownership, exact execution dependencies, buyer-visible gaps, and next actions.
