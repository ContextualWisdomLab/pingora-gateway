# pingora-gateway

`pingora-gateway` is the reusable CWL-managed edge runtime for migrations away from CWL-managed Nginx/OpenResty/ingress-nginx. It is intentionally generic: product routing policy remains in the consuming product, while this repository owns safe transport, configuration validation, bounded proxying, observability and packaging.

The first vertical is an explicit HTTP/HTTPS reverse proxy. Configuration is loaded from `PINGORA_GATEWAY_CONFIG` before the listener is bound. Unknown fields, unsupported versions, ambiguous routes and implicit private-network upstreams are rejected. `/livez`, `/readyz` and `/metrics` are local endpoints. Upstream TLS keeps certificate and hostname verification enabled. Incoming `Forwarded`/`X-Forwarded-*` values are not trusted in this first vertical and are removed before proxying.

## Run locally

```bash
cp config/example.yaml /tmp/gateway.yaml
PINGORA_GATEWAY_CONFIG=/tmp/gateway.yaml cargo run
```

The default example listens on `:8080`. The OCI image runs as UID/GID 65532 and does not require writes to the root filesystem. Mount configuration read-only at `/etc/pingora-gateway/gateway.yaml`.

## Scope boundaries

The Edge Routing bounded context (`src/edge_routing`) is transport-independent. `src/delivery/pingora_proxy.rs` is the Pingora Anti-Corruption Layer. Product-specific authentication, tenant rules, rewrites, static-site semantics, certificate issuance/ACME, WebSocket policy, advanced load balancing and Kubernetes topology are not silently absorbed here; each requires a separate evidence-backed increment.

See `ARCHITECTURE.md`, `docs/CONTEXT_MAP.md`, `docs/API_CONFIG_CONTRACTS.md`, `docs/SECURITY.md` and `docs/OPERABILITY.md` before adding behavior.
