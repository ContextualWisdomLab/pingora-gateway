# Ubiquitous Language

**Edge contract** — the versioned, fail-closed operator configuration admitted before network authority is granted.

**Network authority** — permission for a running gateway process to listen and to contact a specifically configured upstream. Request-controlled destinations are not network authority.

**Listener** — the downstream socket owned by one gateway process.

**Upstream** — one explicitly configured destination the gateway may contact. V1 admits exactly one upstream per process and therefore has no hidden routing or load-balancing rule.

**TLS identity** — the SNI/hostname used to authenticate an HTTPS upstream together with normal certificate-chain verification.

**Request-body budget** — the maximum number of downstream body bytes admitted for one request.

**Delivery adapter** — Pingora-specific code that realizes an admitted edge contract. It may not define product domain rules.

**Liveness** — evidence that the serving process can answer through Pingora.

**Readiness** — in v1, evidence that configuration was admitted and the serving path is active. It does not mean the upstream is healthy.

**Forwarding identity** — `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, or similar claims about client/proxy origin. V1 trusts none from downstream clients.

**Migration evidence** — characterization/RED and replacement/GREEN evidence through the production path, plus deployment, security, rollback, and exact release artifact proof. String replacement is not migration evidence.
