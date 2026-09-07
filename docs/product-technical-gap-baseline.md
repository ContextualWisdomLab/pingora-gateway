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

Fresh PR-triggered execution is independently terminal GREEN on unchanged exact `32e0aeedac7b0fe6234d476245f37994b1b9168f`. CI `34051494660` completed success for `test 101535818434`, `oci-runtime 101535818466`, and `load-contract 101535818420`; Supply Chain `34051494627` / `candidate-evidence 101535818363` also completed success. Exact load artifact `9995582772`, digest `sha256:d7066934079b979cf16c9cc024d2b3ecf7a5fca3aaaf89bb291fdcc6caf777cf`, records 400 requests, 800/800 status/body checks, `http_req_failed` rate `0`, and `http_req_duration` p95 `0.71593235 ms` against the unchanged 4-VU / p95 `<20 ms` contract. The earlier runnerless queue state cleared without source churn, a no-op retrigger, or a runner-selector change; predecessor #62/#64 GREEN is no longer needed as current-head execution authority.

CodeRabbit review explicitly covers exact `32e0aeedac7b0fe6234d476245f37994b1b9168f`, reports no actionable comments, and marks merge risk Minimal up to that exact head. Its walkthrough revalidates the `PeerOptions` value contract, bounded Rust origin, direct readiness gate, non-empty k6 summary requirement, and causal evidence handling. This is exact-head technical bot/static evidence, not an independently required `APPROVED` review.

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

`#64` exact CI `34045381577` and Supply Chain `34045381591` were terminal GREEN before integration. Exact load artifact `9993754723`, digest `sha256:0c545787f21d264fd012d9b34f16d4272c5c8afeeccd3795ac256c70487d321e`, recorded 400/400 HTTP-200 checks, 400/400 body-identity checks, `http_req_failed=0`, and p95 `0.9037063 ms`. This is Rust-only loopback regression evidence, not an Internet/TLS/H2/H3/multi-hop production SLO.

The predecessor #64 hosted RED on `bad9e0ed158d633c259861106f50e223247370dd` remains RCA evidence: gateway release compilation succeeded, then `rustfmt --check` exposed three layout-only fixture diffs before direct fixture tests or k6. The minimal formatter-only repair produced `1b6c5307...`. The later always-run evidence upload did not obscure the causal formatter failure; successful traffic still requires a non-empty summary.

## Compiler prerequisite — #56

Ready #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`. Rust 1.98.1 is the separately gated release-path compiler prerequisite that repairs the Rust 1.98.0 vtable-generation miscompilation; the foundation/docs branch itself remains on manifest MSRV `1.98.0` until #56 is normally integrated. CI `33992794787` and Supply Chain `33992794799` are terminal GREEN on #56's exact head.

Fresh formal review enumeration still contains no independent `APPROVED` review; submitted reviews are `COMMENTED`. Technical CodeRabbit evidence does not replace the organization-required approving review. Do not self-approve or use administrator bypass merely to advance the stack.

## Supplier-intake RED — #54

Ready #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`, based on #56. Its intended hosted RED remains authoritative: CI `33998449940` has GREEN load and OCI lanes but `test 101392951060` fails after reaching compile/test because committed `Cargo.lock` still contains exact package `derivative 2.2.0`. Supply Chain `33998449901` is GREEN on the same SHA.

`RUSTSEC-2024-0388` is an unmaintained advisory, not a memory-safety CVE claim. Do not manufacture GREEN with an audit ignore, deleted lock evidence, muted regression, scanner suppression, or mutable supplier pin.

## Supplier owner path

Protected `cloudflare/pingora/main` is freshly verified at `09696b51bc59315353d96686355861604d0bb48c`. Issue `cloudflare/pingora#889` remains open as of 2026-09-07, and fresh open-PR search returns zero `derivative` removal candidates. The latest published GitHub release, Pingora `0.8.1` from 2026-06-04, still declares workspace `derivative = "2.2.0"`; protected current `main` declares the same dependency.

Historical release manifests `0.3.0`, `0.1.1`, and `0.1.0` do not declare the workspace dependency, but source comparison proves they are not drop-in supplier candidates for the current gateway request-policy boundary. Current #62 imports `HttpUpstreamRequestPolicy` and assigns `peer.options.http_upstream_request_policy = HttpUpstreamRequestPolicy::standard()`; the corresponding `pingora-core/src/upstreams/peer.rs` snapshots at those three tags expose `PeerOptions` without that field/current peer-policy API. A direct dependency downgrade therefore cannot preserve the current hop-by-hop / HTTP/1 upgrade-policy contract without source redesign or separately governed compatibility/backport work. GitHub also reports the `0.3.0` release object as `immutable: false`, independently failing the current immutable-supplier gate. The supported conclusion remains that no admissible already-released supplier candidate has been demonstrated for #54; this does not claim every historical Pingora release is generally incompatible or contains `derivative`.

The downstream evidence comment on #889 keeps the accepted repair surface explicit:

- remove `derivative` from workspace, `pingora-core`, and `pingora-load-balancing` manifests;
- replace `PeerOptions` macro Debug with standard/manual Debug preserving feature-gated non-hook fields, actual configured values, and hook omissions;
- replace `Backend` macro Clone/Debug/Eq/Hash/Ord/PartialOrd semantics with std/manual traits over address+weight while excluding `Extensions`;
- use canonical `PartialOrd = Some(self.cmp(other))` and remove the obsolete non-canonical-PartialOrd Clippy exception;
- regenerate `Cargo.lock` and prove the exact `derivative` package is absent;
- run supplier formatting, workspace/all-feature tests, strict Clippy, rustdoc, advisory/audit checks, `PeerOptions` regressions, and `Backend` identity/order/hash regressions.

A direct upstream branch-creation attempt from the integration identity returned `403 Resource not accessible by integration`. Supplier source mutation therefore remains maintainer-owned. A mutable downstream fork or contributor PR pin is not release authority. The consumer bump must target a maintainer-integrated, release-qualified immutable supplier revision rather than an unreleased mutable head.

## Historical pg-erd succession boundary

Historical Draft #35 remains open. Its generic Rust-origin delta is now genuinely carried by current #62 through the normal #64 merge, but #35 also carried a pg-erd routed-load path that built `cwl-pingora-pg-erd-migration`, launched distinct backend/frontend Rust origins, and ran `tests/load/pg_erd_gateway_smoke.js`. Current #62 does not yet contain that pg-erd routed-load test or binary build, so complete succession is not established.

The surviving pg-erd stack is no longer blocked by undifferentiated inherited formatting debt. Exact Rust 1.98.0 CI logs localized that debt and it was repaired at the lowest demonstrated responsible layers without force pushes, destructive rebases, formatter exclusions, gate weakening, or semantic bulk edits: #6 `013ac250fb3904e2be431ea8e6e9bc2972c3d4d8`; repaired #7 `4a750cbd60dffd42669dfb41ab3487313ec67e24`; #10 formatter repair `ce96a600fced88b7504a244e446ef784d04ae2c4`; #11 formatter repair `e33ae30c981dff0907fe42e21b2d3184f7ccc066`; and explicit two-parent non-force conflict repair `ec1175070e979b1647e0f8a24c28c7a67a235b22` before the #12 layer.

Fresh review then exposed real #12 Admin Config/runtime gaps rather than more formatting noise. Same-port same-family wildcard/concrete listener overlap was rejected, the compiled production path now proves hostile `X-Forwarded-Port` is replaced with the actual gateway listener port, and an accidental child reversal of inherited `MigrationDeliveryPlan::response_header_rules()` was restored so #12 no longer carries a `src/migration_delivery.rs` effective delta.

A further dependency-root inspection found a platform-dependent socket-authority gap on exact `7aae2384215fb3918195a8a003ecb9c37f202eab`: same-port `[::]` plus an IPv4 listener/metrics address was still admitted even though the IPv6 wildcard may consume IPv4 authority when `IPV6_V6ONLY` is disabled. The live #12 branch carries explicit TDD repair: RED `d1c67d4562f6979489ab85e096690b6fb376711f` adds symmetric `[::]`/IPv4 collision expectations while preserving distinct concrete IPv4/IPv6 acceptance; GREEN `7a1f2703e42d1a597955ebc03f35009bda363f8d` minimally extends the migration-local overlap predicate; and `d3c3dc6184651e31f829f97e9b6ddf8f246c1476` makes `API_CONFIG_CONTRACT.md` code-current with the same conservative admission rule.

The CodeRabbit pre-merge summary also carried a valid production-docstring gap. The repair deliberately documents production lifecycle/boundary functions rather than padding self-explanatory tests: `2c9b084671d6356526bd0265e1e8f1d2c11a2b7a` covers migration callback/private lifecycle functions, `da722e1c54a419ece8c934eb279882acc42c7ebc` covers the process-local health response, and `67f07fb88d24818756932c73d9b6bf2716125524` covers the production composition root. These commits are documentation-only at the function level and do not change routing, admission, forwarding, TLS, retry, or product behavior.

The next exact sweep found an additional socket alias not covered by the prior collision matrix: `[::ffff:127.0.0.1]:8080` is an IPv4-mapped IPv6 representation of the same concrete IPv4 authority as `127.0.0.1:8080`. RED `997469fa0ace109756297b28130c20ceb3cd7317` adds symmetric mapped/unmapped collision expectations; GREEN `774d62f8cf58fdb2f107773008c31ca42474d290` minimally extends `listener_authorities_overlap` with `Ipv6Addr::to_ipv4_mapped()` while preserving distinct concrete IPv4/IPv6 admission. Source rustdoc already described this invariant; `f10ddaede49df1bdca65a82478eabc80e591282f` is the documentation-only follow-up that adds the exact IPv4-mapped alias to `API_CONFIG_CONTRACT.md`, keeping the public Admin Config contract aligned with executable behavior.

Current Draft #12 is exact `f10ddaede49df1bdca65a82478eabc80e591282f`, exact base repaired #11 `e33ae30c981dff0907fe42e21b2d3184f7ccc066`, mergeable, with 17 effective files. Exact CI `34080901878` and Supply Chain `34080901932` are queued on this current head; the three CI jobs remain pre-checkout with `steps=[]` and `runner_id=0`. Fresh exact-head technical review covers `e33ae30c...f10ddaed` and reports no still-valid defect in the 17-file effective range; it specifically confirms the mapped-alias predicate, preserved wildcard rejection, and distinct concrete non-aliased admission. This review is technical bot/static evidence only. Until current-head formatting, compile/test, strict Clippy, rustdoc, complete owned coverage, resolved lock, load/OCI and supply-chain execution finishes, the pg-erd repair remains candidate work rather than GREEN succession.

Once exact #12 is GREEN, the repaired ancestry must continue ordinary/non-force through the remaining pg-erd stack and reacquire current-head routed-load/review/OCI/supply-chain evidence before any #35 closure or current-stack succession claim. Product authentication/business logic remains outside the gateway throughout this repair. Historical #42 routed-loopback capacity evidence remains component evidence only and is not transferred to new heads or treated as representative TLS/network/deployment SLO.

## Protocol path

Draft #52/#53 remain behind the supplier root. Their H2→H1 Cookie/body-framing contracts require ordinary non-force ancestry repair after the current dependency root is satisfied. Real-wire acceptance must negotiate the intended protocol path and prove supplier behavior rather than substitute h2c, H1-only input, client-side pre-coalescing, a CWL Cookie shim, or mutable supplier PR pinning.

Protocol work does not receive execution credit until an unchanged current-stack head produces the required RED→GREEN traffic evidence against immutable supplier authority.

## Organization Actions owner plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`; its dedicated writer owns source, refs, and PR reconciliation. This Pingora lane sends evidence through owner issues and does not modify central source.

The last verified protected `.github/main` snapshot in this lane is `c9052e607e5f3cc76e73207e7786b21500721b79`. Any later central movement supersedes it and must be handled by the dedicated owner. Static-only serving must not be classified as shared Pingora responsibility merely because Nginx vocabulary is present. Shared reverse proxy/upstream routing, ingress, public TLS/HTTP edge policy, and explicit edge-runtime contracts remain the relevant classification signals.

## Buyer-visible release gaps

`pingora-gateway` is an implemented candidate, not a released edge product. Protected `main` is freshly verified at `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; the GitHub Releases collection remains empty. Protected `main` therefore remains the governance scaffold until the dependency-ordered stack is normally promoted.

Release/cutover credit still requires one unchanged protected candidate with version/CHANGELOG/tag/package, immutable artifact, SBOM, provenance, reproducibility and rollback evidence; representative TLS/H1.1/H2/H3/WebSocket/streaming and failure traffic; timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain and rootless runtime evidence; realistic buyer-path p95 ≤20 ms where applicable; and parity → shadow/canary → observed rollback → cutover → verified legacy removal.

Legacy Nginx/OpenResty presence alone is not a migration trigger. Static-file serving, certificate issuance/key custody, FastCGI, product authentication, and business logic stay with the correct owner. Only actual shared edge/runtime responsibility is migrated.

## Current causal order

The current work-conserving order is:

`#54 derivative RED + #62 exact hosted/technical GREEN → maintainer-integrated release-qualified immutable Pingora derivative repair → gateway supplier bump + committed Cargo.lock regeneration → unchanged #54 absence regression GREEN + preserved #62 current-stack GREEN → #56 independent APPROVED governance → protected integration → #12 exact mapped-alias/Admin Config + production-docstring GREEN → pg-erd ordinary/non-force succession + current protocol restacks → real-wire RED/GREEN → immutable release/SBOM/provenance/reproducibility/rollback → shadow/canary/cutover → verified Nginx/OpenResty removal`.

Queued checks are incomplete evidence, not GREEN. Predecessor execution/review does not transfer across a new exact head. No force push, destructive rebase, self-approval, routine bypass, mutable supplier dependency, threshold weakening, sample reduction, or release/cutover claim is accepted.
