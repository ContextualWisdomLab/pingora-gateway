# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads and protected-branch tips are evidence snapshots, not release authority; later live evidence supersedes exact identities below.

## Current execution update — #63 GREEN and #62 non-force adoption

Focused load-harness repair #63 is exact `190cd6aebc0b82b01f04b01ff095a741f831a0c8`, direct child of #56 exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, and changes only `.github/workflows/ci.yml` (+14/-0). Before gateway admission/k6 it proves the origin process is alive and directly reachable at `127.0.0.1:18081/fixture-ready`, while failing immediately if that process exits. The probe does not traverse or warm the measured gateway `/load-contract` route; the 400-request / 4-VU sample, zero-failure requirements, p95 `<20 ms`, runtime, dependency graph, compiler, Pingora source, and routing semantics remain unchanged.

`#63` exact CI `34027297080` is terminal GREEN on unchanged exact head: `test 101470437546`, `load-contract 101470437571`, and `oci-runtime 101470437479` all succeeded. Supply Chain `34027296921` / `candidate-evidence 101470436916` is also terminal GREEN through committed dependency audit, exact candidate image, SPDX SBOM, image scan, exact-source binding, and evidence upload. Immutable load artifact `9988273511`, digest `sha256:64500c71061e2df4a42693952fca72cdef5eac4df979b26bff3209722cf4276a`, records 400/400 HTTP-200 and body-identity checks, checks rate `1`, `http_req_failed` rate `0`, and `http_req_duration p(95)=0.64378865 ms`. The startup-race repair therefore restores zero-failure correctness without weakening the `<20 ms` threshold or reducing samples.

Ready supplier-semantics #62 adopted the proven #63 workflow delta by an ordinary non-force merge commit, exact `389801461e28f422c166fb0918a2805d7085d05a`, with parents `d4a54d8d1337797c3763c3d915c674cdf483db1c` and `190cd6aebc0b82b01f04b01ff095a741f831a0c8`. This preserves the full supplier-characterization lineage and the validated readiness repair without force-push or destructive rebase. Its effective child scope relative to #56 is now exactly `.github/workflows/ci.yml`, `TEST_STRATEGY.md`, and `tests/pingora_supplier_semantics_contract.rs`; Pingora pin, `Cargo.lock`, production gateway Rust, routing/TLS/auth/business logic, consumer state, and release metadata remain unchanged.

Fresh current-head technical review independently inspected exact `389801461e28f422c166fb0918a2805d7085d05a`, confirmed `190cd6ae...` is the second parent and the adoption delta from prior #62 is only `.github/workflows/ci.yml`, rechecked the supplier characterization and unchanged load acceptance, and found no issue. This is current technical-review evidence, not an organization-required independent `APPROVED` review.

New exact #62 validation is CI `34030105237` and Supply Chain `34030105222`. Latest fresh sweeps still show `test 101477936704`, `oci-runtime 101477936789`, `load-contract 101477936942`, and `candidate-evidence 101477936825` pre-checkout queued with `steps=[]` and `runner_id=0`. Predecessor #63 GREEN and predecessor #62 GREEN/RED evidence are causal evidence only; the merged exact head must prove test/load/OCI/Supply Chain again before any successor or promotion credit is assigned. No no-op retrigger or runner-selector churn is justified by runnerless pre-checkout admission alone.

The prior #62 load RED remains preserved as RCA evidence. Exact `d4a54d8d...` produced 395 successes / 5 failures over 400 requests / 4 VUs while p95 stayed `1.0152792 ms`; all five failures were first-second gateway 502s caused by `Upstream ConnectRefused` to the asynchronously starting fixture at `127.0.0.1:18081`. The #63 GREEN proves that readiness admission, not latency-threshold relaxation, was the causal fix.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. #63 now independently proves the repaired load harness on the same unchanged 400-request / 4-VU contract with zero failures and p95 `0.64378865 ms`. These are exact-head loopback CI results, not buyer-path WAN/TLS/H2/H3 performance.

Production host/SNI/routing parity, realistic TLS/H1.1/H2/H3/WebSocket/streaming failure traffic, timeout/retry/backpressure behavior, client-IP/header/cookie/body-limit parity, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. No immutable gateway release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation

Workflow-policy RED→GREEN is integrated into foundation #1 exact `0da81a93f93e869c15bb7d34c55fc87479d16522`. Foundation owns `main`-only push scope, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts. No force update, destructive rebase, bypass, or self-approval was used to obtain the foundation state.

## Compiler prerequisite — #56

Ready #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`, based on foundation. Rust 1.98.1 is the release compiler; CI, Supply Chain, and OCI paths install/select/verify it before Cargo and reject later compiler/toolchain rebinding through workflow environment, Cargo selectors/config/wrappers, shell indirection, and Docker build instructions.

Exact CI `33992794787` and Supply Chain `33992794799` completed terminal GREEN on this head. The CI lanes cover OCI non-root/read-only runtime, the loopback k6 contract, formatting, compile/test, strict Clippy, public rustdoc, owned-production coverage, and dependency-lock evidence. Supply Chain covers committed dependency audit, exact candidate image, SPDX SBOM, image scan, and exact-source binding.

CodeRabbit reviewed the final narrow compiler-oracle subrange and found no new issue. This does not replace the organization-required independent approving review. Fresh review enumeration still contains no `APPROVED` review for #56. #56 remains unmerged; self-approval and administrator bypass are not used.

## Supplier-intake RED — #54

Ready #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, with #56 as exact base. Its effective child delta remains `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`.

The intended hosted RED is proven. CI `33998449940` has GREEN `load-contract 101392950922`, GREEN `oci-runtime 101392951059`, and failing `test 101392951060`. The failing test job passed exact checkout, native dependencies, Rust 1.98.1 and formatting, then failed at `Compile and test`. The effective regression `rustsec_2024_0388_dependency_is_absent_from_committed_lock` requires no exact `derivative` package in committed `Cargo.lock`; the pinned graph still contains `derivative 2.2.0`.

Supply Chain `33998449901` / `candidate-evidence 101392950711` is GREEN on the same SHA through `cargo deny`, exact image build, SPDX SBOM, image scan and exact-source binding. This is not contradictory: the general audit lane permits transitive unmaintained crates under current policy, while #54 is the stricter CWL supplier-intake release criterion. `RUSTSEC-2024-0388` is an unmaintained advisory, not a memory-safety-CVE claim.

Do not manufacture GREEN with an audit ignore, deleted lock evidence, scanner suppression, muted test, or mutable supplier pin. The causal repair is a maintainer-integrated immutable supplier revision/release that removes `derivative`, followed by an exact gateway supplier bump and lock regeneration.

## Supplier-semantics characterization — #62

Ready #62 is a direct child of #56 and does not carry #54's intentionally failing dependency-absence oracle. Current exact head is `389801461e28f422c166fb0918a2805d7085d05a`, created by ordinary non-force adoption of exact #63 into prior supplier-semantics head `d4a54d8d1337797c3763c3d915c674cdf483db1c`. Effective scope relative to #56 is exactly `.github/workflows/ci.yml`, `TEST_STRATEGY.md`, and `tests/pingora_supplier_semantics_contract.rs`. The Pingora revision, `Cargo.lock`, production gateway source, routing/TLS/auth/business logic, and consumer state are unchanged.

Pinned supplier `cloudflare/pingora@09696b51bc59315353d96686355861604d0bb48c` uses `derivative` for `PeerOptions` Debug and for `Backend` Clone/Hash/Eq/Ord/Debug semantics. The downstream `PeerOptions` contract requires all 26 unconditional non-hook fields in the OpenSSL build (`bind_to` through `custom_l4`) to remain represented in Debug output while `upstream_tcp_sock_tweak_hook`, `proxy_digest_user_data_hook`, and `upstream_tls_handshake_complete_hook` remain absent.

Four acceptance-oracle defects are explicit and test-first:

- RED `f3ff0e94534d07a499ff27862800530f95034d16` → GREEN `79e53796e4a1acaf786fd151ab423ab7f3d9d924`: raw substring matching could let `connection_timeout` pass merely because `total_connection_timeout` was present.
- RED `eb9bb65ebf39b54a238a11f198a4a015121daa42` → GREEN `b3d3abaec2c7c0ff9fc097f085f17eba44ae9640`: field identity must not depend on incidental Debug whitespace or layout.
- Review at `2cbaaeff2fe89416f4698098ebd1c95e461ef4e8` found that the structural matcher still accepted nested or quoted field-like text because it did not track structural depth. RED `8f48cbab2f55a9a5764365cc6df74942e26581c1` adds nested and quoted false-positive controls; GREEN `281a1fd4f55864e235c1e8a63306b940c14dfdf6` tracks outer brace/bracket/parenthesis depth plus quoted/escaped regions.
- Fresh source inspection found that field-name presence still did not prove value semantics: a replacement could retain all expected names while reporting stale or fabricated values. RED `7001f0d712e9ddd7dd3eaf8245042e618b71f07d` adds a deliberately presence-only scalar matcher plus a wrong-value negative control and live configured-value acceptance. GREEN `67e4c6f65665dea3caf0383644c96eee9b7bfa9f` requires the exact top-level scalar value token; `82566e02...` records the invariant in `TEST_STRATEGY.md`. The live contract mutates `verify_cert`, `verify_hostname`, `max_h2_streams`, `allow_h1_response_invalid_content_length`, `second_keyshare`, and `tcp_fast_open` and requires Debug to reflect those configured values.

Prior exact `d4a54d8d1337797c3763c3d915c674cdf483db1c` proved supplier-semantics source GREEN in `test 101463462339`, including formatting, compile/test, strict lint, rustdoc, 100% owned-production line/region coverage, and lock verification; `oci-runtime 101463462432` and Supply Chain `candidate-evidence 101463497940` were also GREEN. Its load-only RED was the fixture-startup race described above. Fresh exact-range CodeRabbit review covered `8027ebfb...d4a54d8d` with no actionable comments; this remains predecessor technical review evidence after adoption.

Current exact `38980146...` has current technical review with no issue but still must execute all lanes again. Exact #63 GREEN proves the adopted workflow change independently, but no predecessor execution credit is transferred to the merged head.

The generic gateway does not enable Pingora load balancing merely to instantiate `Backend` for a supplier test. `Backend` address+weight equality/hash/order with opaque `Extensions` excluded remains an upstream-owner acceptance item until that bounded capability is actually consumed downstream.

## Load-contract reliability repair — #63

Ready #63 exists because the full exact #62 traffic execution exposed a correctness RED while latency remained GREEN. Exact job logs proved five first-second gateway 502s caused by upstream TCP connection refusal to the not-yet-bound local fixture, followed by 395 successful requests on the unchanged process pair. Exact head `190cd6aebc0b82b01f04b01ff095a741f831a0c8` changes only `.github/workflows/ci.yml` from #56. The repair adds explicit test-origin readiness admission before gateway liveness and measured k6 traffic; the origin process must stay alive and answer directly or the job fails before any measured gateway sample is taken.

The repair is now exact-head GREEN. CI `34027297080` completed GREEN for test, load-contract, and OCI runtime, and Supply Chain `34027296921` completed GREEN. Artifact `9988273511` proves 400/400 request checks, zero request failures, and p95 `0.64378865 ms` on the unchanged load contract. Fresh CodeRabbit review covers exact `18fb38b1...190cd6ae`, selects only `.github/workflows/ci.yml`, binds review-risk coverage to exact head, and reports no actionable comments. This technical review does not replace an independent `APPROVED` review.

The valid #63 delta has already been adopted non-destructively into #62 exact `389801461e28f422c166fb0918a2805d7085d05a`. #63 remains open until the successor exact head proves all carried deltas and evidence; closure is allowed only after verified complete succession, not merely because a merge commit was created.

## Supplier owner path

Protected `cloudflare/pingora/main` is freshly verified at `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open and still identifies `derivative 2.2.0` as unmaintained; fresh open-PR search finds no maintainer-integrated removal candidate.

Existing #889 downstream evidence is maintained in place and has been updated to current #62 exact/revalidation. The minimal owner repair remains: replace `PeerOptions` macro Debug with a manual/std Debug implementation preserving every current non-hook top-level field, its actual configured value, and hook omissions under the same feature gates; replace `Backend` macro equality/hash/order with std/manual traits over address+weight only, with canonical `PartialOrd = Some(self.cmp(other))` and opaque `Extensions` excluded; remove `derivative` from workspace/core/load-balancing manifests; regenerate the lockfile; then prove supplier fmt/tests/Clippy/rustdoc/audit GREEN.

A direct attempt from this automation identity to create an upstream repair branch at exact protected supplier head was rejected by GitHub with `403 Resource not accessible by integration`. That makes the remaining supplier source mutation an external maintainer-permission boundary for this lane; it does not authorize a mutable fork/PR pin or advisory suppression. Only a maintainer-integrated immutable supplier revision/release is downstream dependency authority.

## Protocol path

Draft #52/#53 remain behind the supplier root. They must be repaired by ordinary non-force ancestry adoption/restack after the supply-chain dependency root is satisfied. The H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before protocol RED/GREEN, merge, or release credit.

Mutable supplier Cookie/body-framing work is evidence only until current-main maintainer integration produces immutable authority. After supplier repair, protocol order is `#52/#53 non-force ancestry repair → H2→H1 Cookie/body-framing traffic RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora sends exact evidence through the owner path without modifying central source from this lane.

A fresh owner-plane sweep at 2026-09-06T11:37:30Z observed protected `.github/main` at `0b0f10476469d52adc40f98495d50855486cd32f` (#1957, preflight rate-limit candidate postponement). Its immediate parent `5ea1cc47ec040fa4f6417136f059be637666c2a2` is the valid #1958 integration point that added workflow-level coalescing for superseded OpenCode review dispatches. The latter is real queue-health progress, but neither #1957 nor #1958 modifies or satisfies #1952's Nginx/Pingora bounded-context classification finding. This is an evidence snapshot; later protected-main movement supersedes the tip identity without invalidating the two integrated commits.

Owner handoff `.github#1952` remains open. Overlapping #1946 remains open at exact `790ef33ea60ada7d21503ca57ccf955a1a36d44b` and changes the same four central policy files required by #1952 for a distinct oversized-Contents/Git-Blobs repair. Its old base is behind protected main; GitHub currently reports the PR mergeable, while exact comparison against protected `main@0b0f1047...` is diverged at ahead 3 / behind 2 with merge base `43024633eba9d96b0456970391360da5a171fbda`. The owner-safe sequence is therefore still: non-force reconcile/integrate #1946 onto fresh protected main, then adopt that protected-main result and add #1952's static-only-vs-edge-runtime responsibility-profile RED→GREEN. The Pingora lane updated the existing #1952 coordination comment in place and did not modify `.github` source/ref/PR state.

Queue-health evidence and product-source evidence remain distinct. #62's prior runnerless states later acquired runners and exposed real leaf findings; current exact merged #62 may likewise wait for admission without justifying source no-op retriggers. Runner delay, formatting failure, semantic failure, and traffic correctness failure must not be collapsed into one scheduler diagnosis.

## Legacy migration and release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Nginx/OpenResty presence alone is not enough to move a workload into `pingora-gateway`: the responsibility must be shared edge routing/TLS/HTTP/load-balancing/runtime policy rather than static-file serving, certificate issuance, FastCGI, product auth, or business logic.

Migration remains release-first: owner-safe inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified legacy removal.

Protected `pingora-gateway/main` is freshly verified at `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`, and the GitHub Releases collection remains empty. Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. No release, canary, cutover, or legacy-removal credit is assigned until fresh protected-main and release evidence proves those gates.

## Current causal order

Primary execution root: `#63 exact readiness-repair GREEN → #62 ordinary non-force adoption at 38980146... → re-prove #62 exact test/load/OCI/Supply Chain; current technical review is already no-issue`.

Primary supplier root after that local repair: `#54 hosted derivative RED + preserved #62 supplier semantics → maintainer-integrated immutable derivative repair → gateway supplier bump + committed lock regeneration → unchanged #54 absence regression GREEN + preserved #62 semantics/load GREEN → exact CI/Supply Chain/security/runtime GREEN → #56 independent approval/governance → foundation integration as applicable → #52/#53 non-force ancestry repair → protocol traffic RED/GREEN → immutable release → parity/shadow/canary/rollback/cutover → verified Nginx/OpenResty removal`.

Parallel owner path: `.github#1946` must first reconcile onto a fresh protected-main tip; `.github#1952` then adds the bounded static-only-vs-edge-runtime classification contract. Integrated #1958 improves queued OpenCode review coalescing but does not satisfy #1952. Neither path grants the Pingora writer permission to modify `.github` source/refs or relax true edge-runtime enforcement.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps current ownership, exact execution dependencies, buyer-visible gaps, and next actions.