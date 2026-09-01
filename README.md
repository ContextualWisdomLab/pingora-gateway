# Pingora Gateway

`pingora-gateway` is ContextualWisdomLab's shared Rust edge-runtime boundary for CWL-managed reverse-proxy traffic. It is built on Cloudflare Pingora, but consumer products integrate through the repository's versioned edge configuration rather than through Pingora types.

The current `0.1.0` development line is intentionally narrow: one traffic listener, one separately declared metrics listener, one explicitly approved HTTP or HTTPS upstream, fail-closed configuration, bounded request bodies and upstream I/O budgets, liveness/readiness endpoints, credential-safe low-cardinality telemetry, distrust of client-supplied forwarding identity, and a production binary that runs through Pingora. Product-specific routing, static-site semantics, WebSocket policy, load balancing, dynamic reload, downstream TLS termination, ACME, and Kubernetes Gateway API are separate increments and must be justified by a real consumer migration.

## Run locally

Create a configuration from `examples/gateway.yaml`, point the upstream address at a service you control, and run:

```bash
cargo run --bin cwl-pingora-gateway -- --config ./gateway.yaml
```

The process refuses to start when `--config` is missing, unreadable, unknown fields are present, the contract version is unsupported, the traffic and metrics listeners collide, the body limit is zero, more than one v1 upstream is configured, a timeout is zero, or TLS identity is incomplete. `/livez` and `/readyz` are served directly through the Pingora request path. Readiness currently means that the configuration was admitted and the serving path is active; it does not probe upstream health.

For proxied requests, inbound `Forwarded`, `X-Forwarded-*`, and `X-Real-IP` identity are discarded. The v1 cleartext downstream listener emits only `Forwarded: proto=http`; it does not assert a client IP until a separately reviewed trusted-proxy contract exists. Pingora's standard upstream request policy strips hop-by-hop and connection-nominated headers.

The dedicated metrics listener serves Prometheus text and should normally be bound to loopback, a pod-only address, or another access-controlled observability network. The initial gateway metrics deliberately have no attacker-controlled labels: total completed requests, request lifecycle errors, and observed request-body bytes. Gateway access logs contain only response status, coarse outcome, and request-body byte count; the application does not log URI/query, Host, client identity, Authorization, Cookie, tokens, or configured credentials.

## Container

`Dockerfile` runs the binary as numeric uid/gid `65532` and does not require a writable application directory. Mount configuration read-only and run the image with a read-only root filesystem. The Dockerfile is a packaging contract, not a published release: no immutable image digest exists yet, and the repository still lacks a committed `Cargo.lock`, SBOM/provenance publication, hosted container integration evidence, and release approval.

## Architecture and operations

Start with `ARCHITECTURE.md`, `CONTEXT_MAP.md`, `UBIQUITOUS_LANGUAGE.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TEST_STRATEGY.md`, `OPERABILITY.md`, and `API_CONFIG_CONTRACT.md`. Current migration gaps and organization inventory are tracked in `docs/product-technical-gap-baseline.md`; primary-source traceability is in `docs/doctoring/TRACEABILITY.md`.
