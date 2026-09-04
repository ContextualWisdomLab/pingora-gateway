# Ubiquitous Language

**Edge contract** — the versioned, fail-closed operator configuration admitted before network authority is granted.

**Network authority** — permission for a running gateway process to listen and to contact a specifically configured upstream. Request-controlled destinations are not network authority.

**Listener** — the downstream socket owned by one gateway process.

**Upstream** — one explicitly configured destination the gateway may contact. V1 admits exactly one upstream per process and therefore has no hidden routing or load-balancing rule.

**TLS identity** — the SNI/hostname used to authenticate an HTTPS upstream together with normal certificate-chain verification.

**Request-body budget** — the maximum number of downstream body bytes admitted for one request.

**Request-header parser admission budget** — an operator-controlled bound enforced while downstream request headers are being parsed/buffered, before the application callback receives a constructed request header. A callback-only rejection after parsing is not this budget. Pinned Pingora has finite supplier HTTP/1 ceilings but no supported configurable HTTP/1 parser-phase hook for CWL yet; #43/#993 track that gap. HTTP/2 decoded-header-list accounting is a separate protocol-specific limit.

**Response-body lifetime budget** — the optional pg-erd version-2 monotonic lifetime that begins at the first non-informational upstream response header and is checked on subsequent body-progress callbacks without resetting on progress. It complements per-read inactivity and is not an exact interrupt for a quiescent pending read.

**Protocol transition** — a connection-wide change from HTTP/1.1 to another protocol, signaled by HTTP `Upgrade` semantics. Generic v1 and the bounded pg-erd candidate do not admit protocol transitions; WebSocket or another upgraded protocol requires a separate versioned consumer-derived contract. HTTP/2 Extended CONNECT and HTTP/3 mechanisms are distinct contracts rather than aliases for HTTP/1 Upgrade.

**Delivery adapter** — Pingora-specific code that realizes an admitted edge contract. It may not define product domain rules.

**Liveness** — evidence that the serving process can answer through Pingora.

**Readiness** — in v1, evidence that configuration was admitted and the serving path is active. It does not mean the upstream is healthy.

**Forwarding identity** — `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, or similar claims about client/proxy origin. V1 trusts none from downstream clients.

**Migration evidence** — characterization/RED and replacement/GREEN evidence through the production path, plus deployment, security, rollback, and exact release artifact proof. String replacement is not migration evidence.
