# Context Map

| Context | Type | Owns | Relationship |
| --- | --- | --- | --- |
| Edge Contract | Supporting | Versioned active configuration, explicit upstream authority, request-body budget, timeout/TLS invariants | Upstream of Startup/Activation and Pingora Delivery; v1 remains one-upstream-per-process |
| Edge Routing | Supporting characterization | Deterministic exact/prefix path selection among explicitly named upstream identities, precedence and fail-closed ambiguity | Consumer-derived shared-edge contract; transport-neutral; not active in v1 |
| HTTP Policy | Supporting characterization | Explicit edge-owned HTTP response field identity/value contracts and fail-closed validation | Separate from Edge Routing and product response semantics; transport-neutral; not active in v1 |
| Migration Plan | Supporting composition | Explicit migration upstream-authority set plus validated Edge Routing and HTTP Policy | Transport-neutral prerequisite for migration delivery; does not grant network authority |
| Migration Delivery | Generic adapter | One-to-one binding from admitted migration upstream identities to explicit validated Pingora peers | Downstream of Migration Plan and `UpstreamConfig`; rejects missing/duplicate/undeclared authority; not yet wired into `GatewayProxy` |
| Startup/Activation | Application | `--config` parsing, file loading, validation-before-listen, process composition | Orchestrates active Edge Contract and Delivery |
| Pingora Delivery | Generic adapter | `HttpPeer`, proxy callbacks, health response, forwarded-header distrust, streamed-body enforcement | Conformist to Pingora, downstream of admitted edge contracts |
| Consumer Product | External bounded context | Product routes and semantics, static behavior, authentication/authorization, business policy, domain state | Supplies legacy-edge behavior for characterization where responsibility is truly shared; never imported here |
| Wardnet / EgressWeave | External security contexts | Security verdicts and canonical network/security policy owned there | Referenced boundary only; not reimplemented in HTTP Policy |
| Keyverse | External identity context | Canonical identity/credential authority | Referenced boundary only; not reimplemented here |
| Certificate Management | External operational context | Issuance, renewal, storage, rotation | Supplies trust material/platform TLS as appropriate; not owned by this runtime |

The active v1 shared kernel is the public versioned configuration vocabulary. `edge_routing`, `http_policy`, and `migration_plan` are executable migration characterizations; `migration_delivery` may prebind those characterized stable identities to explicit Pingora peers but does not activate them in the v1 proxy request path. A later versioned runtime/config transition must explicitly admit multi-route serving. There is no shared Rust domain model with consumer products, Context Graph, Enterprise Architecture, Wardnet/EgressWeave, or Keyverse.
