# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance.

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

Ready #62 is a direct child of #56 and does not carry #54's intentionally failing dependency-absence oracle. Current exact head is `6f6b6fea8acbcc58e85a474102ebcefe744d4157`; effective scope remains exactly `tests/pingora_supplier_semantics_contract.rs` and `TEST_STRATEGY.md`. The Pingora revision, `Cargo.lock`, production gateway source, workflows, routing/TLS/auth/business logic, and consumer state are unchanged.

Pinned supplier `cloudflare/pingora@09696b51bc59315353d96686355861604d0bb48c` uses `derivative` for `PeerOptions` Debug and for `Backend` Clone/Hash/Eq/Ord/Debug semantics. The downstream `PeerOptions` contract requires all 26 unconditional non-hook fields in the OpenSSL build (`bind_to` through `custom_l4`) to remain represented in Debug output while `upstream_tcp_sock_tweak_hook`, `proxy_digest_user_data_hook`, and `upstream_tls_handshake_complete_hook` remain absent.

Three acceptance-oracle defects are now explicit and test-first:

- RED `f3ff0e94534d07a499ff27862800530f95034d16` → GREEN `79e53796e4a1acaf786fd151ab423ab7f3d9d924`: raw substring matching could let `connection_timeout` pass merely because `total_connection_timeout` was present.
- RED `eb9bb65ebf39b54a238a11f198a4a015121daa42` → GREEN `b3d3abaec2c7c0ff9fc097f085f17eba44ae9640`: field identity must not depend on incidental `{ field:` / `, field:` Debug whitespace or layout.
- Fresh exact-head review at `2cbaaeff2fe89416f4698098ebd1c95e461ef4e8` found that the structural matcher still accepted nested or quoted field-like text because it did not track structural depth. Source RED `8f48cbab2f55a9a5764365cc6df74942e26581c1` adds nested and quoted false-positive controls. GREEN `281a1fd4f55864e235c1e8a63306b940c14dfdf6` tracks outer brace/bracket/parenthesis depth plus quoted/escaped regions before accepting a candidate as a top-level field. `6f6b6fea...` records the resulting invariant in `TEST_STRATEGY.md`.

The current review credit boundary is exact: the review at `2cbaaeff...` produced the nested-field finding and is not transferred to `6f6b6fea...`. A fresh current-head CodeRabbit review has been requested after the repair.

Current exact Actions are CI `34011310005` and Supply Chain `34011310029`. Fresh reads show `oci-runtime 101427530622`, `load-contract 101427530748`, `test 101427530782`, and `candidate-evidence 101427530846` queued before checkout on `ubuntu-24.04`, with empty executed-step lists and `runner_id=0`. No predecessor GREEN is transferred and no no-op retrigger is used.

The generic gateway does not enable Pingora load balancing merely to instantiate `Backend` for a supplier test. `Backend` address+weight equality/hash/order with opaque `Extensions` excluded remains an upstream-owner acceptance item until that bounded capability is actually consumed downstream.

## Supplier owner path

Protected `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open, and the latest open-PR search finds no maintainer-integrated `derivative`-removal candidate.

Existing #889 downstream evidence is maintained in place. The minimal owner repair remains: replace `PeerOptions` macro Debug with a manual/std Debug implementation preserving every current non-hook top-level field and omitting hooks without treating incidental formatting whitespace as semantic authority; replace `Backend` macro equality/hash/order with std/manual traits over address+weight only, with canonical `PartialOrd = Some(self.cmp(other))` and opaque `Extensions` excluded; remove `derivative` from workspace/core/load-balancing manifests; regenerate the lockfile; then prove supplier fmt/tests/Clippy/rustdoc/audit GREEN. Field-name regression must reject overlapping names, nested values, and quoted field-like text.

Only a maintainer-integrated immutable supplier revision/release is downstream dependency authority.

## Protocol path

Draft #52/#53 remain behind the supplier root. They must be repaired by ordinary non-force ancestry adoption/restack after the supply-chain dependency root is satisfied. The H2→H1 Cookie real-wire fixture must become the effective protocol-only child delta before protocol RED/GREEN, merge, or release credit.

Mutable supplier Cookie/body-framing work is evidence only until current-main maintainer integration produces immutable authority. After supplier repair, protocol order is `#52/#53 non-force ancestry repair → H2→H1 Cookie/body-framing traffic RED → current-main supplier repair/integration → immutable supplier identity → gateway bump → exact traffic GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source/refs/PR state. Pingora sends exact evidence through the owner path without modifying central source from this lane.

Runner delay and semantic failure are kept distinct. #56 waited runnerless and later passed unchanged; #54 waited and later produced the intended semantic RED on the same head. Current #62 `6f6b6fea...` queueing is therefore a lane-local admission sample, not sufficient evidence for gateway-local runner-selector churn or a new central scheduler defect.

Protected `.github/main` remains `efb8926923de45245338159a489a1b227e81945f` after its owner-side contextual-orchestrator retry-stacking repair. That is useful owner-plane progress but is not #62 execution credit.

## Legacy migration and release gate

Legacy consumer repositories with dedicated writers remain read-only from this lane. Nginx/OpenResty presence alone is not enough to move a workload into `pingora-gateway`: the responsibility must be shared edge routing/TLS/HTTP/load-balancing/runtime policy rather than static-file serving, certificate issuance, FastCGI, product auth, or business logic.

Migration remains release-first: owner-safe inventory → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified legacy removal.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. No release, canary, cutover, or legacy-removal credit is assigned until fresh protected-main and release evidence proves those gates.

## Current causal order

`#54 hosted derivative RED + #62 nested/quoted source RED→GREEN → #62 exact hosted execution + fresh current review → maintainer-integrated immutable derivative repair → gateway supplier bump + committed lock regeneration → unchanged #54 absence regression GREEN + preserved #62 semantics GREEN → exact CI/Supply Chain/security/runtime GREEN → #56 independent approval/governance → foundation integration as applicable → #52/#53 non-force ancestry repair → protocol traffic RED/GREEN → immutable release → parity/shadow/canary/rollback/cutover → verified Nginx/OpenResty removal`.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps current ownership, exact execution dependencies, buyer-visible gaps, and next actions.
