# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-07 KST. Mutable PR heads and protected-branch tips are evidence snapshots, not release authority; later live evidence supersedes exact identities below.

## Current execution update — #62 exact GREEN, #63 verified succession

Focused load-harness repair #63 exact `190cd6aebc0b82b01f04b01ff095a741f831a0c8` changed only `.github/workflows/ci.yml` (+14/-0) from #56. Before gateway admission and k6 it proves the origin process is alive and directly reachable at `127.0.0.1:18081/fixture-ready`, while failing immediately if that process exits. The probe does not traverse or warm the measured gateway `/load-contract` route; the 400-request / 4-VU sample, zero-failure requirements, p95 `<20 ms`, runtime, dependency graph, compiler, Pingora source, and routing semantics remain unchanged.

`#63` independently proved the repair GREEN in CI `34027297080` and Supply Chain `34027296921`. Immutable load artifact `9988273511`, digest `sha256:64500c71061e2df4a42693952fca72cdef5eac4df979b26bff3209722cf4276a`, records 400/400 HTTP-200 and body-identity checks, zero request failures, and p95 `0.64378865 ms`.

Ready supplier-semantics #62 adopted the proven #63 workflow delta by ordinary non-force merge commit `389801461e28f422c166fb0918a2805d7085d05a`, with parents `d4a54d8d1337797c3763c3d915c674cdf483db1c` and `190cd6aebc0b82b01f04b01ff095a741f831a0c8`. Effective scope relative to #56 is exactly `.github/workflows/ci.yml`, `TEST_STRATEGY.md`, and `tests/pingora_supplier_semantics_contract.rs`; Pingora pin, `Cargo.lock`, production gateway Rust, routing/TLS/auth/business logic, consumer state, and release metadata remain unchanged.

Exact compare proves the successor tree is the complete non-destructive composition of the separately reviewed component tips: `d4a54d8d...38980146` changes only the #63 workflow delta, while `190cd6ae...38980146` changes only the two supplier-characterization files. There is no later workflow divergence.

Current exact #62 execution is terminal GREEN. CI `34030105237` succeeded for `test 101477936704`, `oci-runtime 101477936789`, and `load-contract 101477936942`. Supply Chain `34030105222` / `candidate-evidence 101477936825` succeeded through committed dependency audit, exact candidate image, SPDX SBOM, image scan, exact-source binding, and evidence upload. Immutable load artifact `9988970232`, digest `sha256:781ee31569f536b3fdaef29c5c2740bbe8bd47d1fb6b0bb451fff790a50e7dc7`, records 400/400 HTTP-200 and 400/400 body-identity checks, checks rate `1`, `http_req_failed` rate `0`, and `http_req_duration p(95)=1.5580363 ms` under the unchanged 400-request / 4-VU contract.

Because current successor execution re-proves all carried workflow, supplier-semantics, OCI, load and Supply Chain evidence, #63 is closed only as **verified complete succession** into #62. It is not counted as merged and no valid delta/test/fixture/contract/evidence is discarded.

Fresh CodeRabbit review now explicitly covers exact `389801461e28f422c166fb0918a2805d7085d05a` and reports no issues. It independently verified #63 as the second parent, the workflow-only adoption delta, direct fixture readiness before gateway/k6 admission, unchanged load thresholds, and the current `PeerOptions` characterization. This is technical bot review evidence only; no independent organization-required `APPROVED` review exists for promotion.

The prior load RED remains RCA evidence. Exact `d4a54d8d...` produced 395 successes / 5 failures over 400 requests / 4 VUs while p95 stayed `1.0152792 ms`; all five failures were first-second gateway 502s caused by `Upstream ConnectRefused` to the asynchronously starting fixture at `127.0.0.1:18081`. The exact successor GREEN proves readiness admission, not threshold relaxation, was the causal fix.

## Rust-only performance-evidence repair — #64

Fresh review found a separate performance-attribution defect in the exact #62 GREEN path: `.github/workflows/ci.yml` still launches `tests/load/upstream_fixture.py`, a Python `ThreadingHTTPServer`, inside the measured k6 round trip. The #62 p95 remains valid evidence for that exact executed topology, but it is not final Rust-first gateway-performance evidence because Python interpreter/thread scheduling contributes to `http_req_duration`.

Ready #64 is an ordinary non-force child of exact #62. Current exact head is `1b6c5307e6ff7837dd7504ea2cc23936ab44a86c`; exact compare from #62 is ahead 9 / behind 0 with #62 as exact merge base and five effective paths: `.github/workflows/ci.yml`, `TEST_STRATEGY.md`, added `tests/load/load_origin.rs`, removed `tests/load/upstream_fixture.py`, and added `tests/load_evidence_workflow_contract.rs`.

The successor adapts the valid intent of historical Draft #35 without merging its old diverged stack and adopts the later bounded-worker origin design from the #41/#42 lineage. The std-only Rust fixture validates startup controls before bind, caps workers at 256, uses a bounded accepted-socket queue, bounds request-header buffering, uses TCP_NODELAY and deterministic Content-Length framing, and defaults to keep-alive/no artificial delay so the existing 4-VU low-contention contract remains distinct from capacity testing.

The load lane uses Rust 1.98.1 to format, compile and run the fixture's direct parser/startup/framing tests with `rustc -D warnings`, compiles the optimized helper outside Cargo production/example targets, then uses that helper for the measured path. The origin-only `/fixture-ready` probe remains outside the gateway and does not warm the measured route. Request count, VUs, exact status/body assertions, zero-failure gate and p95 `<20 ms` threshold are unchanged.

Post-initial review found a separate failure-evidence defect: the loopback artifact upload was `always()` plus `if-no-files-found: error`, so a checkout/build/origin-startup failure before k6 could be followed by a secondary missing-summary failure that obscured the causal error. Source RED `ecb6245e21225dc1f2cfb8ccab01a721821043af` adds a semantic workflow contract; causal GREEN `7ed13abe5e26dc64bfe36b5760b8b5953452f0c3` adds a success-only `test -s k6-summary.json` gate and makes only the later always-run upload ignore an absent file; `0e17dcfae35b5a36e56a1bd25fbee1d4115b872f` keeps the oracle semantic over parsed YAML values; predecessor `bad9e0ed...` makes `TEST_STRATEGY.md` code-current. Successful measured traffic still fails if its summary is absent or empty; only an earlier primary failure is protected from being replaced by artifact-upload noise.

The first hosted execution of predecessor `bad9e0ed158d633c259861106f50e223247370dd` is now terminal and replaces the prior queue snapshot. CI `34041022444` produced GREEN `oci-runtime 101507678563` and GREEN `test 101507678628`, while `load-contract 101507678702` failed in `Build exact gateway candidate and Rust load origin`. Supply Chain `34041022495` is terminal GREEN. The load log shows the exact gateway release build itself completed under Rust 1.98.1; the next `rustfmt --edition 2021 --check tests/load/load_origin.rs` reported exactly three layout-only diffs in the new fixture and stopped execution before its direct tests, optimized-helper compilation, or k6. The later always-run artifact step found no summary and succeeded with `if-no-files-found: ignore`, preserving the formatter error as the causal failure rather than manufacturing a secondary artifact failure.

Commit `1b6c5307e6ff7837dd7504ea2cc23936ab44a86c` is the minimal causal repair and changes only rustfmt-required layout in `tests/load/load_origin.rs`; fixture behavior, queue/worker/header bounds, connection mode, sample/VU count, p95 threshold, production gateway code, workflow logic, dependency graph, and supplier pin are unchanged. Fresh exact-head CI `34045381577` and Supply Chain `34045381591` are queued and uncredited at the latest read. Live CodeRabbit review-risk evidence currently covers only through `ea32917f97c81d1c3053302ec9b2099de8990ea5` and reports no actionable comments/minimal risk for that reviewed scope; current `1b6c5307...` is five commits ahead, so no review credit transfers and fresh exact-head review is required.

Historical #35 remains open because its pg-erd Rust-origin delta has not yet been fully succeeded by #64; closure requires verified complete succession of every valid generic and pg-erd contract/evidence.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Exact #62 proves the functional 400-request / 4-VU loopback path with zero failed requests and `http_req_duration p(95)=1.5580363 ms`, but the measured origin on that head is Python and therefore the latency value is not credited as final Rust-first gateway-performance evidence. #64 must re-prove the unchanged traffic contract with the Rust-only measured path before that evidence is promotable. Neither loopback result is an Internet/TLS/H2/H3 production SLO.

Production host/SNI/routing parity, realistic TLS/H1.1/H2/H3/WebSocket/streaming failure traffic, timeout/retry/backpressure behavior, client-IP/header/cookie/body-limit parity, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. No immutable gateway release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation

Workflow-policy RED→GREEN is integrated into foundation #1 exact `0da81a93f93e869c15bb7d34c55fc87479d16522`. Foundation owns `main`-only push scope, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts. No force update, destructive rebase, bypass, or self-approval was used to obtain the foundation state.

## Compiler prerequisite — #56

Ready #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, based on foundation. Rust 1.98.1 is the release compiler; CI, Supply Chain, and OCI paths install/select/verify it before Cargo and reject later compiler/toolchain rebinding through workflow environment, Cargo selectors/config/wrappers, shell indirection, and Docker build instructions.

Exact CI `33992794787` and Supply Chain `33992794799` are terminal GREEN. CodeRabbit reviewed the final narrow compiler-oracle subrange without a new issue. This does not replace the organization-required independent approving review. Fresh review enumeration still contains no `APPROVED` review for #56, so #56 remains unmerged without self-approval or administrator bypass.

## Supplier-intake RED — #54

Ready #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, with #56 as exact base. Its effective child delta remains `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`.

The intended hosted RED remains authoritative. CI `33998449940` has GREEN `load-contract 101392950922`, GREEN `oci-runtime 101392951059`, and failing `test 101392951060`. The failing test reaches compile/test and fails because `rustsec_2024_0388_dependency_is_absent_from_committed_lock` finds exact package `derivative 2.2.0` in committed `Cargo.lock`. Supply Chain `33998449901` / `candidate-evidence 101392950711` is GREEN on the same SHA.

`RUSTSEC-2024-0388` is an unmaintained advisory, not a memory-safety-CVE claim. Do not manufacture GREEN with an audit ignore, deleted lock evidence, scanner suppression, muted test, or mutable supplier pin. The causal repair is a maintainer-integrated immutable supplier revision/release that removes `derivative`, followed by an exact gateway supplier bump and lock regeneration.

## Supplier-semantics characterization — #62

Ready #62 does not carry #54's intentionally failing dependency-absence oracle. Current exact head is `389801461e28f422c166fb0918a2805d7085d05a` with exact hosted GREEN as recorded above.

Pinned supplier `cloudflare/pingora@09696b51bc59315353d96686355861604d0bb48c` uses `derivative` for `PeerOptions` Debug and for `Backend` Clone/Hash/Eq/Ord/Debug semantics. The downstream `PeerOptions` contract requires all 26 unconditional non-hook fields in the OpenSSL build (`bind_to` through `custom_l4`) to remain represented in Debug output while `upstream_tcp_sock_tweak_hook`, `proxy_digest_user_data_hook`, and `upstream_tls_handshake_complete_hook` remain absent.

Four acceptance-oracle defects were made explicit test-first:

- RED `f3ff0e94534d07a499ff27862800530f95034d16` → GREEN `79e53796e4a1acaf786fd151ab423ab7f3d9d924`: overlapping field names cannot satisfy exact field identity.
- RED `eb9bb65ebf39b54a238a11f198a4a015121daa42` → GREEN `b3d3abaec2c7c0ff9fc097f085f17eba44ae9640`: field identity cannot depend on incidental Debug whitespace/layout.
- RED `8f48cbab2f55a9a5764365cc6df74942e26581c1` → GREEN `281a1fd4f55864e235c1e8a63306b940c14dfdf6`: nested and quoted field-like text cannot satisfy a top-level field contract.
- RED `7001f0d712e9ddd7dd3eaf8245042e618b71f07d` → GREEN `67e4c6f65665dea3caf0383644c96eee9b7bfa9f`: field-name presence is insufficient; representative configured scalar values must be emitted truthfully.

The generic gateway does not enable Pingora load balancing merely to instantiate `Backend` for a supplier test. `Backend` address+weight equality/hash/order with opaque `Extensions` excluded remains an upstream-owner acceptance item until that bounded capability is actually consumed downstream.

## Load-contract reliability repair — #63

`#63` exact `190cd6aebc0b82b01f04b01ff095a741f831a0c8` is closed, not merged, after verified complete succession into #62. Its exact original CI/Supply Chain GREEN and immutable artifact remain causal evidence, while current #62 exact GREEN is the successor execution authority. The closure is valid only because exact ancestry and compare prove the complete +14/-0 workflow delta is present unchanged and the successor re-proves every carried acceptance lane.

## Supplier owner path

Protected `cloudflare/pingora/main` is freshly verified at `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open and identifies `derivative 2.2.0` as unmaintained; fresh open-PR search finds no maintainer-integrated removal candidate.

Existing #889 downstream evidence is maintained in place and updated to current #62 exact GREEN. The minimal owner repair remains: replace `PeerOptions` macro Debug with manual/std Debug preserving every current non-hook top-level field, actual configured values and hook omissions under the same feature gates; replace `Backend` macro equality/hash/order with std/manual traits over address+weight only, canonical `PartialOrd = Some(self.cmp(other))`, opaque `Extensions` excluded; remove `derivative` from workspace/core/load-balancing manifests; regenerate `Cargo.lock`; then prove supplier fmt/tests/Clippy/rustdoc/audit GREEN.

A direct attempt from this automation identity to create an upstream repair branch at exact protected supplier head was rejected with `403 Resource not accessible by integration`. That leaves supplier source mutation with upstream maintainers; it does not authorize a mutable fork/PR pin or advisory suppression.

## Protocol path

Draft #52/#53 remain behind the supplier root. They must be repaired by ordinary non-force ancestry adoption/restack after the supply-chain dependency root is satisfied. The H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before protocol RED/GREEN, merge, or release credit.

Mutable supplier Cookie/body-framing work is evidence only until current-main maintainer integration produces immutable authority. After supplier repair, protocol order is `#52/#53 non-force ancestry repair → H2→H1 Cookie/body-framing traffic RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora sends exact evidence through the owner path without modifying central source from this lane.

Fresh protected owner state is `.github/main@9aad23c09da468716a788cfed65cd44f7d84a284`, which integrates #1975's tests-only concurrency-group parser repair on top of prior #1964 queue coalescing work. Neither infrastructure change satisfies #1952's Nginx/Pingora responsibility-classification finding.

Owner handoff `.github#1952` remains open. Overlapping #1946 remains open at exact `790ef33ea60ada7d21503ca57ccf955a1a36d44b` and changes the same four central policy files required by #1952 for a distinct oversized-Contents/Git-Blobs repair. Exact comparison against protected `main@9aad23c0...` is diverged at ahead 3 / behind 7 with merge base `43024633eba9d96b0456970391360da5a171fbda`. The owner-safe sequence remains ordinary non-force reconciliation/integration of #1946, followed by #1952's static-only-vs-edge-runtime responsibility-profile RED→GREEN. This Pingora lane does not modify `.github` source/ref/PR state.

Static-only product serving must not be promoted into shared Pingora responsibility merely because Nginx vocabulary is present; actual reverse-proxy/upstream routing, ingress-controller, public TLS/HTTP edge policy and explicit edge-runtime contracts remain fail-closed.

## Legacy migration and release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Nginx/OpenResty presence alone is not enough to move a workload into `pingora-gateway`: the responsibility must be shared edge routing/TLS/HTTP/load-balancing/runtime policy rather than static-file serving, certificate issuance, FastCGI, product auth, or business logic.

Migration remains release-first: owner-safe inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified legacy removal.