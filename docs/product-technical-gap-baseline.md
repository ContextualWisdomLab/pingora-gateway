# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-07 KST. Mutable PR heads and protected-branch tips are evidence snapshots, not release authority. Later live evidence supersedes exact identities below.

## Canonical responsibility boundary

`pingora-gateway` owns reusable Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation behavior. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure primitives, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy or business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/private-key custody, or application-specific business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs. Source copies, cross-service application SQL, mutable sibling-PR dependencies, and hidden Shared Kernels are rejected.

## Current dependency root — #64 normally integrated into #62

Ready #64 exact `1b6c5307e6ff7837dd7504ea2cc23936ab44a86c` was normally merged into the non-protected #62 integration branch. The resulting #62 exact head is `32e0aeedac7b0fe6234d476245f37994b1b9168f`; #62 remains based on compiler prerequisite #56 exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`.

This is ordinary non-force child-to-parent integration, not a protected-`main` merge, release, canary, cutover, or legacy-removal event. The composed #62 effective range relative to #56 is exactly six paths:

- `.github/workflows/ci.yml`;
- `TEST_STRATEGY.md`;
- `tests/pingora_supplier_semantics_contract.rs`;
- added `tests/load/load_origin.rs`;
- removed `tests/load/upstream_fixture.py`;
- added `tests/load_evidence_workflow_contract.rs`.

No Pingora pin, `Cargo.lock`, production gateway Rust, routing/TLS/auth/business logic, consumer state, or release metadata changed in this integration.

Fresh PR-triggered revalidation belongs to the new exact merge head rather than either parent. CI `34051494660` and Supply Chain `34051494627` were queued at the first post-merge read. Predecessor #62/#64 GREEN and review evidence remain valid component history but do not transfer exact-head GREEN or review credit to `32e0aeed...`. A fresh exact-head CodeRabbit review was requested on the six-path composed range; bot/static review remains distinct from any independently required `APPROVED` review.

## Preserved supplier-semantics contract

Prior #62 exact `389801461e28f422c166fb0918a2805d7085d05a` is terminal CI/Supply Chain GREEN and had exact-head no-issue technical review before the merge. Its executable supplier contract remains unchanged in current #62:

- all 26 unconditional non-hook fields in the pinned OpenSSL `PeerOptions` Debug surface remain represented;
- `upstream_tcp_sock_tweak_hook`, `proxy_digest_user_data_hook`, and `upstream_tls_handshake_complete_hook` remain omitted;
- overlapping names, nested/quoted field-like text, incidental Debug layout, and stale/fabricated scalar values cannot satisfy the oracle;
- representative configured values such as `verify_cert`, `verify_hostname`, `max_h2_streams`, `allow_h1_response_invalid_content_length`, `second_keyshare`, and `tcp_fast_open` must be emitted truthfully;
- `Backend` equality/hash/order authority remains `addr` then `weight`, excluding opaque `http::Extensions`; canonical `PartialOrd` is `Some(self.cmp(other))`.

Historical #63 exact `190cd6aebc0b82b01f04b01ff095a741f831a0c8` remains valid causal evidence for the direct-origin readiness repair and was closed only after verified complete succession into prior #62. Its 400-request / 4-VU GREEN artifact remains historical evidence, not current merged-head execution credit.

## Rust-only performance attribution

Merged #64 replaced Python `ThreadingHTTPServer` inside the measured k6 round trip with a bounded std-only Rust HTTP/1.1 origin. The fixture validates startup controls before bind, caps workers at 256, uses a bounded accepted-socket queue and bounded request-header buffer, enables TCP_NODELAY, and emits deterministic Content-Length framing. It stays outside Cargo production/example targets.

The readiness probe remains direct to `127.0.0.1:18081/fixture-ready` and does not traverse or warm the measured gateway route. The acceptance contract remains 400 requests / 4 VUs, exact status/body checks, zero failed HTTP requests, and `http_req_duration p(95) < 20 ms`; no sample reduction, cache warm-up, or measurement exclusion is accepted.

#64 exact CI `34045381577` and Supply Chain `34045381591` were terminal GREEN before integration. Exact load artifact `9993754723`, digest `sha256:0c545787f21d264fd012d9b34f16d4272c5c8afeeccd3795ac256c70487d321e`, recorded 400/400 HTTP-200 checks, 400/400 body-identity checks, `http_req_failed=0`, and p95 `0.9037063 ms`. This is Rust-only loopback regression evidence, not an Internet/TLS/H2/H3/multi-hop production SLO.

The predecessor #64 hosted RED on `bad9e0ed158d633c259861106f50e223247370dd` remains RCA evidence: gateway release compilation succeeded, then `rustfmt --check` exposed three layout-only fixture diffs before direct fixture tests or k6. The minimal formatter-only repair produced `1b6c5307...`. The later always-run evidence upload did not obscure the causal formatter failure; successful traffic still requires a non-empty summary.

## Compiler prerequisite — #56

Ready #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`. Rust 1.98.1 is the release compiler after the Rust 1.98.0 vtable-generation miscompilation finding. CI `33992794787` and Supply Chain `33992794799` are terminal GREEN on that exact head.

Fresh formal review enumeration still contains no independent `APPROVED` review; submitted reviews are `COMMENTED`. Technical CodeRabbit evidence does not replace the organization-required approving review. Do not self-approve or use administrator bypass merely to advance the stack.

## Supplier-intake RED — #54

Ready #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, based on #56. Its intended hosted RED remains authoritative: CI `33998449940` has GREEN load and OCI lanes but `test 101392951060` fails after reaching compile/test because committed `Cargo.lock` still contains exact package `derivative 2.2.0`. Supply Chain `33998449901` is GREEN on the same SHA.

`RUSTSEC-2024-0388` is an unmaintained advisory, not a memory-safety CVE claim. Do not manufacture GREEN with an audit ignore, deleted lock evidence, muted regression, scanner suppression, or mutable supplier pin.

## Supplier owner path

Protected `cloudflare/pingora/main` is freshly verified at `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open, and fresh open-PR search finds no maintainer-integrated `derivative` removal candidate.

The downstream evidence comment on #889 keeps the accepted repair surface explicit:

- remove `derivative` from workspace, `pingora-core`, and `pingora-load-balancing` manifests;
- replace `PeerOptions` macro Debug with standard/manual Debug preserving feature-gated non-hook fields, actual configured values, and hook omissions;
- replace `Backend` macro Clone/Debug/Eq/Hash/Ord/PartialOrd semantics with std/manual traits over address+weight while excluding `Extensions`;
- use canonical `PartialOrd = Some(self.cmp(other))` and remove the obsolete non-canonical-PartialOrd Clippy exception;
- regenerate `Cargo.lock` and prove the exact `derivative` package is absent;
- run supplier formatting, workspace/all-feature tests, strict Clippy, rustdoc, advisory/audit checks, `PeerOptions` regressions, and `Backend` identity/order/hash regressions.

A direct upstream branch-creation attempt from the integration identity returned `403 Resource not accessible by integration`. Supplier source mutation therefore remains maintainer-owned. A mutable downstream fork or contributor PR pin is not release authority.

## Historical pg-erd succession boundary

Historical Draft #35 remains open. Its generic Rust-origin delta is now genuinely carried by current #62 through the normal #64 merge, but #35 also carried a pg-erd routed-load path that built `cwl-pingora-pg-erd-migration`, launched distinct backend/frontend Rust origins, and ran `tests/load/pg_erd_gateway_smoke.js`.

Current #62 does not contain that pg-erd routed-load test or binary build in its six-path integration range. Therefore #35 is not fully succeeded and must not be closed merely because the generic Rust-origin work is integrated. The remaining pg-erd stack must be reconciled against current bounded-edge authority rather than copied wholesale from the old diverged stack. Product auth/business logic remains outside the gateway.

## Protocol path

Draft #52/#53 remain behind the supplier root. Their H2→H1 Cookie/body-framing contracts require ordinary non-force ancestry repair after the current dependency root is satisfied. Real-wire acceptance must negotiate the intended protocol path and prove supplier behavior rather than substitute h2c, H1-only input, client-side pre-coalescing, a CWL Cookie shim, or mutable supplier PR pinning.

Protocol work does not receive execution credit until an unchanged current-stack head produces the required RED→GREEN traffic evidence against immutable supplier authority.

## Organization Actions owner plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source, refs, and PR reconciliation. This Pingora lane sends evidence through owner issues and does not modify central source.

Fresh protected `.github/main` is `5c60b5d9e8fb8461480d6a30b0b5f149753afb12`, which integrates #1980's cancel-in-progress value contract. Overlapping `.github#1946` remains at `790ef33ea60ada7d21503ca57ccf955a1a36d44b`; exact comparison with current protected main is diverged at ahead 3 / behind 10 with merge base `43024633eba9d96b0456970391360da5a171fbda`. Its four central Pingora-policy files must be reconciled normally by the dedicated owner before `.github#1952` performs the static-only-vs-edge-runtime responsibility-classification RED→GREEN.

Static-only serving must not be classified as shared Pingora responsibility merely because Nginx vocabulary is present. Shared reverse proxy/upstream routing, ingress, public TLS/HTTP edge policy, and explicit edge-runtime contracts remain the relevant classification signals.

## Buyer-visible release gaps

`pingora-gateway` is an implemented candidate, not a released edge product. Protected `main` remains the governance scaffold until the dependency-ordered stack is normally promoted. No GitHub Release exists yet.

Release/cutover credit still requires one unchanged protected candidate with version/CHANGELOG/tag/package, immutable artifact, SBOM, provenance, reproducibility and rollback evidence; representative TLS/H1.1/H2/H3/WebSocket/streaming and failure traffic; timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain and rootless runtime evidence; realistic buyer-path p95 ≤20 ms where applicable; and parity → shadow/canary → observed rollback → cutover → verified legacy removal.

Legacy Nginx/OpenResty presence alone is not a migration trigger. Static-file serving, certificate issuance/key custody, FastCGI, product authentication, and business logic stay with the correct owner. Only actual shared edge/runtime responsibility is migrated.

## Current causal order

The current work-conserving order is:

`#62@32e0aeed exact revalidation + exact composed review → #54 derivative RED + preserved supplier semantics/load contracts → maintainer-integrated immutable Pingora derivative repair → gateway supplier bump + committed Cargo.lock regeneration → unchanged #54 absence regression GREEN + #62 current-stack GREEN → #56 independent APPROVED governance → protected integration → current pg-erd/protocol non-force restacks → real-wire RED/GREEN → immutable release/SBOM/provenance/reproducibility/rollback → shadow/canary/cutover → verified Nginx/OpenResty removal`.

Queued checks are incomplete evidence, not GREEN. Predecessor execution/review does not transfer across a new exact head. No force push, destructive rebase, self-approval, routine bypass, mutable supplier dependency, threshold weakening, sample reduction, or release/cutover claim is accepted.