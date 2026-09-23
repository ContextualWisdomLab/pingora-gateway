# Context Map

| Context | Type | Owns | Relationship |
| --- | --- | --- | --- |
| Edge Contract | Supporting | Versioned configuration, explicit upstream authority, request-body budget, timeout/TLS invariants | Upstream of Pingora Delivery |
| Startup/Activation | Application | `--config` parsing, file loading, validation-before-listen, process composition | Orchestrates Edge Contract and Delivery |
| Pingora Delivery | Generic adapter | `HttpPeer`, proxy callbacks, health response, forwarded-header distrust, streamed-body enforcement | Conformist to Pingora, downstream of Edge Contract |
| Consumer Product | External bounded context | Routes, static semantics, auth/business policy, deployment-specific behavior | Customer of published config/image contract; never imported here |
| Certificate Management | External operational context | Issuance, renewal, storage, rotation | Supplies trust material/platform TLS as appropriate; not owned by this runtime |

The only shared kernel is the public versioned configuration vocabulary. There is no shared Rust domain model with consumer products.
