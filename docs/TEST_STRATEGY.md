# Test Strategy

Domain unit tests cover route uniqueness and precedence. Configuration tests cover schema closure and private-network opt-in. `tests/architecture.sh` is a DDD fitness function forbidding Pingora dependencies in the Edge Routing domain. `tests/integration.sh` starts a local upstream and the real gateway process, then proves readiness, proxy delivery, metrics and 413 enforcement through the production listener.

Before a consumer migration, add characterization tests for the exact Nginx behavior it uses: route precedence, SPA/static semantics, HTTP caching/ranges, Host/SNI/TLS, forwarded-header trust, WebSocket/upgrade, limits/timeouts, redirects/security headers, health/drain, streaming/uploads and error responses as applicable. Performance claims require representative benchmark evidence; no 20 ms claim exists until measured.

Planned gates: property tests for route selection/config boundaries, fuzzing for config/header normalization, concurrency/drain/recovery tests, exact production statement/branch coverage evidence, OCI non-root/read-only-root execution and published artifact provenance.
