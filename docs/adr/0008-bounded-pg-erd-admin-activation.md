# ADR 0008: Activate the pg-erd migration through a bounded admin profile

- Status: Candidate
- Date: 2026-09-02
- Bounded contexts: Admin Config, Edge Routing, HTTP Policy, Migration Delivery, Runtime Isolation, Observability, Pingora Delivery

## Context

PR #11 makes the characterized `pg-erd-cloud` route, HTTP-policy, forwarding-trust, runtime-isolation and observability contracts executable as `MigrationGatewayProxy`, but the only production composition root still consumes `GatewayConfig` v1. That public v1 contract intentionally admits one upstream per process, while the captured pg-erd edge requires two stable transport authorities: `backend` and `frontend`.

The next migration step needs a real listener-capable composition root without turning the generic gateway configuration into a programmable routing language. Operator input must not become authority to invent routes, service discovery, product authentication/business policy, Keyverse identity, Wardnet/EgressWeave verdicts, or new upstream names.

A second constraint is trust-material handling. `pingora_delivery::build_peer` loads an operator-selected custom PEM trust bundle. Parsing configuration must not preload that file and then load it again later, because a validate-then-reload sequence creates an avoidable time-of-check/time-of-use window around mutable trust material.

A third constraint emerged once the dedicated listener was tested as a process rather than as a callback object: migration routing has a fallback `/ -> frontend`, so without an explicit process-health boundary it also routes `/livez` and `/readyz` into the consumer. That conflates gateway process health with product health and makes saturation/rollout diagnosis unsafe.

A fourth constraint emerged from effective socket authority rather than field equality. A traffic listener such as `0.0.0.0:8080` and a metrics listener such as `127.0.0.1:8080` are different `SocketAddr` values but can compete for one port authority at bind time. An IPv6 wildcard can also consume the IPv4 port on dual-stack platforms when `IPV6_V6ONLY` is disabled. Because listener bind semantics are part of Admin Config admission, wildcard overlap must fail before Pingora receives network authority instead of surfacing as platform-dependent startup failure. The same review found that generic `GatewayConfig` still admitted port-zero traffic, metrics and upstream sockets even though the migration profile had already made those invalid: listener port zero delegates authority to an OS-selected ephemeral port, metrics port zero makes the declared endpoint indeterminate, and upstream port zero is not a concrete connectable authority.

A fifth constraint emerged when the same network-authority model was reviewed across listener and upstream roles. A concrete upstream can currently equal the gateway's own traffic listener, producing a recursive self-proxy path until admission or I/O budgets stop it, or equal the metrics listener, routing ordinary application requests into the gateway-owned Prometheus surface. Exact-address checks alone are insufficient because wildcard and IPv6 dual-stack aliases can create the same effective authority. This is a gateway-owned socket-separation invariant, not product route or origin identity policy.

## Alternatives

1. **Widen `GatewayConfig` v1 to arbitrary multi-route configuration.** Rejected for this slice. It would combine a public contract version change with migration-specific routing semantics and create a second authority for product routing.
2. **Encode the pg-erd plan directly in the binary without an Admin Config boundary.** Rejected. Listener addresses, runtime budgets and concrete upstream transport/TLS data are operational inputs that require strict validation and a reusable application boundary.
3. **Use a bounded migration profile with fixed shared-edge semantics and operator-controlled transport values.** Selected. It exposes only values that actually vary by deployment while compiling the characterized migration contract into code.

## Decision

Introduce `PgErdMigrationConfig` version 1 and a separate `cwl-pingora-pg-erd-migration` composition root.

The profile accepts downstream and metrics socket addresses, non-zero request-body/in-flight/upstream-keepalive budgets, and exactly one concrete transport binding for each characterized stable upstream identity, `backend` and `frontend`. Each binding reuses the existing `UpstreamConfig` address, TLS/SNI/trust-bundle and timeout contract.

Traffic and metrics addresses must not overlap one effective socket authority. The shared `edge_contract::socket_authorities_overlap` invariant rejects equal addresses, same-port same-family wildcard aliases, and same-port IPv6-wildcard/IPv4 combinations whose dual-stack bind behavior is platform-dependent. Distinct concrete IP addresses remain configurable on the same non-zero port. Generic `GatewayConfig` and the migration-specific Admin Config consume this same overlap invariant, while `UpstreamConfig` rejects transport port zero at the shared value boundary. The migration profile performs its characterized zero-port check before shared upstream validation so its established `ZeroTransportAuthorityPort` contract remains stable.

Every admitted upstream must also remain separate from both gateway-owned listeners under that same effective socket-authority model. `edge_contract::validate_upstream_authority_separation` rejects an upstream that overlaps the traffic listener or metrics listener through an exact address, same-family wildcard alias, or conservative IPv6-wildcard/IPv4 dual-stack case. Generic v1 applies the invariant to its single upstream; `PgErdMigrationConfig` applies it only after the fixed `backend`/`frontend` identity has passed migration-plan validation. This prevents recursive self-proxy and application routing into the internal metrics service without asserting that distinct product-owned upstream identities can never share an endpoint.

The profile does not accept route rules, response headers, product policy, identity/authentication configuration, service-discovery inputs, or arbitrary migration upstream names. Its fixed plan preserves the observed pg-erd Traefik behavior: exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` including `/apiary`, fallback `/ -> frontend`, and the four characterized response-security fields.

Configuration parsing performs pure contract/authority validation. It checks the exact transport-authority bijection and `UpstreamConfig` invariants but does not load custom trust-bundle bytes. `build_proxy` materializes `MigrationDeliveryPlan` once immediately before the composition root creates listeners, so custom trust material is read once through the canonical Pingora delivery adapter. Any read/PEM failure remains fail-closed before listener activation without a duplicate preload.

`/livez` and `/readyz` are shared process-local Pingora endpoints. Both the generic and migration adapters consume one internal health responder, while the legacy consumer `/healthz` remains ordinary characterized traffic to `backend`. This is a runtime-operability boundary, not product health inference.

The fixed migration plan and an already validated `MigrationDeliveryPlan`/`RuntimeIsolationLimits` are internal invariants rather than operator-controlled failure surfaces. `MigrationGatewayProxy::new` therefore models callback construction as infallible after those boundaries have succeeded. `try_new` remains as a compatibility constructor for existing callers. The admin error type contains only failures that can actually be caused by parsed operator input or peer materialization; impossible fixed-profile error variants are not retained merely to satisfy a generic shape.

The generic `cwl-pingora-gateway` binary, `GatewayConfig` v1 public shape, and one-upstream semantics stay unchanged. Its network-authority validation is tightened to require non-zero traffic/metrics/upstream ports, effective traffic/metrics separation, and upstream separation from both gateway-owned listeners because the same admission defects existed there independently of pg-erd routing. Process identity still makes the intended migration deployment contract explicit rather than silently changing generic routing semantics.

## RED -> GREEN evidence

RED commit `251330f5f47cebde186bb2c26f1bd01284f37090` introduced the executable admin-config contract before `migration_admin` existed. Initial GREEN commits `38c0ee803f826bd3e1f61dee1cdf5c4c59553218`, `242a7a6b09c6c45f3da05ffc427ef46054b6f086`, and `52acafeecef807ce8e362ebc15d1e049ae29c613` added the bounded profile, public module and separate composition root.

A fresh source review found that `from_yaml` called `build_delivery`, causing custom trust material to be loaded during parse and again during `build_proxy`. Commit `4af750e272eb7a2c48378f9f7e76c3b346c5356f` removed that duplicate materialization and commit `ce8d7f28b6499b4373dde3ffedca3f721faf90d1` made missing, extra, duplicate and renamed transport authority explicit executable cases.

Compiled-listener RED/traffic contract `c1cc2c8a08b06546020f6ccad2f168a90bf4328c` then exposed the process-health fallthrough defect. Commits `957e1f45e122135be7c68b4e645b66a1a9cfef8b`, `21f55240935e33a35daa0769d16e4a2d35cd1402`, `450a10d2a05eaf520009ab5250ef19fd411f36c9`, and `57a70ee5b9be247e954d8fb7180d35ce35a22377` introduced one shared process-health responder and applied it to both runtime adapters without changing consumer `/healthz` routing. Commit `674a5aa1f2b4eaabc564fb20a9bebf87c31c7a2a` made the dedicated binary target explicit in Cargo metadata. Commits `0619e2de168ebb5e9600660c81fbed64b327b5ab` and `50f5874c178c8494766b752905facb3be9a99ef5` add fail-closed compiled-startup and Admin Config negative coverage. Commits `39b353ffbcedf23b5d0b62916b506e9b84484e83` and `7cc38c27f5cb5ebb605707299a6e134c9dc508c3` remove structurally impossible callback/fixed-profile error branches rather than weakening the repository's 100% owned-production coverage gate.

The listener-authority repair was also test-first. RED `937c73c7ba2146048bba6873123595573b428ff5` proved that pg-erd wildcard/specific aliases were accepted by the predecessor. GREEN `4430e34e1fbf1ff6fb5cb7f216bd8627cf9e12f5` initially closed the migration path. A second review found the same equality-only defect in generic `GatewayConfig`; RED `dfe897915f563659488d22a2284439df96e33534` exposed it, `31175105e44cd775f57e4960c3926622915a9d31` moved the effective-authority invariant into the shared edge contract, and `26c6eb0cc92d75376e3b7690e4821db4c789d203` removed the migration-local duplicate. Commit `777f6d0f960480fdc8c1b06d9cf651524fa167be` covers equal, wildcard-first, wildcard-second, IPv4, IPv6, mixed-family, distinct-address and distinct-port decision paths without weakening the 100% region gate.

A third source sweep found generic port-zero authority remained inconsistent with the already bounded migration profile. RED `50637f6419059382c57634747b527614ea7a6f23` requires generic traffic/metrics/upstream port zero to fail closed. GREEN `0cd848abf17f1f1a1a2194bf67a404934656b201` adds non-zero listener/metrics checks to `GatewayConfig` and a shared non-zero transport invariant to `UpstreamConfig`; `cbdacdf93ac71594652de15b17231b4b1e465605` preserves the migration profile's more specific `ZeroTransportAuthorityPort` error contract by evaluating that characterized condition before shared upstream validation.

A fourth review on the post-commit-reset stack found that admitted upstream authority could still point at a gateway-owned listener. RED commits `dca69bf586fe6dd5f309678f941e3924f608a382` and `94301ef3872d4926424594aaf8caada0ee172149` add generic and pg-erd expectations for traffic-listener self-loop, metrics-listener exposure, wildcard/dual-stack aliases, and distinct-concrete acceptance before the new production error variants existed. GREEN `f256ec3ecc1b27e876c8e734a8a57e9e6396cd7b` places the reusable separation rule in `edge_contract`; `e778bbc7a200e520e6c4722503fcd8cbff2584f1` applies it to the fixed migration profile after stable identity validation. Generic and pg-erd compiled startup contracts `d209f0f87766f392858cb3f0565a4c69da08533e` and `db7d3f470f151b60bf434302c7882e564358d493` require the invalid authority to fail before listener activation. These commits are source acceptance only until one unchanged exact PR #27 head executes all required gates.

Hosted exact-head evidence must be reacquired after every later source/documentation movement; predecessor runs do not transfer. Source presence of the compiled traffic test is not a parity claim.

## Risks and consequences

This slice can start a clear-text multi-route listener in source, so mistakes now have a larger blast radius than characterization-only code. It remains Draft and pre-traffic until exact-head formatting, compile/test, clippy, rustdoc, 100% owned-production line/region coverage, supply-chain/security, and the dedicated compiled-listener traffic contract are terminal GREEN.

The cross-family IPv6 wildcard rule is intentionally conservative. A platform configured with IPv6-only wildcard sockets could technically bind an IPv4 listener on the same port, but admitting that configuration would make the contract depend on an OS socket option the current Admin Config does not control. If a future listener adapter explicitly owns `IPV6_V6ONLY`, that option needs its own versioned behavior and acceptance tests before this fail-closed rule can be narrowed.

The listener/upstream separation intentionally applies only to gateway-owned listener authority. It does not reject distinct concrete upstream identities that happen to share a product endpoint, because such co-location can be legitimate and the gateway does not own the product deployment topology. If future service discovery or dynamic binding is introduced, the same no-self-loop/no-metrics-origin invariant must be enforced on the resolved authority before it is admitted.

Rejecting port zero deliberately trades OS-assigned ephemeral listener convenience for a stable operator-declared production authority. Tests that need ephemeral ports should obtain them in test harnesses and then pass the concrete bound address to test components; production Admin Config does not delegate public edge identity to the kernel implicitly.

The fixed migration profile is consumer-specific. It must not become a pattern of accumulating unrelated product semantics in the shared gateway. A future reusable multi-route contract requires separately proven common semantics and a versioned public design, not copy/paste growth of this profile.

The current pg-erd source entryPoint is HTTP. HTTPS/TLS listener activation, HTTP/2 or HTTP/3, WebSocket/streaming policy, load balancing, retries beyond the generic one-attempt invariant, larger origin-capacity stress, shadow/canary, cutover and legacy removal remain separate evidence-backed decisions.

## Context Fabric and authority boundary

This source candidate is producer evidence only. `context-graph-contracts` and `enterprise-architecture-core` remain read-only to this writer. It does not become authoritative EA `validated execution` until an immutable released Context Assertion/CloudEvent contract, an immutable protected Pingora release and real parity -> shadow/canary -> cutover/rollback evidence exist and are admitted through the Context Fabric owner path.

No request bodies, forwarding headers, cookies, credentials, customer data, runtime logs or product-domain facts are copied into Context Graph or EA stores, and no cross-service SQL is introduced.

## Next acceptance

Run one unchanged exact PR #27 head through formatting, compile/test, clippy, warning-denied rustdoc, 100% owned-production line/region coverage, routed k6, OCI, SAST and supply-chain/security gates; repair any deterministic repository defect causally and do not transfer predecessor evidence. The stacked pg-erd source already contains dedicated saturation/recovery, refused-origin, connected-silent read timeout, orderly partial response, pre-header and post-commit TCP reset, routed graceful drain, payload-free observability, hardened OCI invocation and local routed k6 contracts. After those and the listener/upstream authority separation are terminal GREEN, the next technical gaps are broader streaming/Upgrade failure recovery, slow-drip/whole-response-lifetime control, larger origin-capacity stress, tracing, and representative TLS/network/deployment measurement. Release/canary additionally requires the unresolved organization dependency/security and authoritative Dependency Review gates, immutable artifact provenance/SBOM, rehearsed rollback, and then-live protected review governance.
