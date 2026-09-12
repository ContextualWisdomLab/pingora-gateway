# pg-erd-cloud Forwarding Parity

## Evidence boundary

The migration candidate is characterized from `ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535` and Traefik's documented HTTP proxy behavior. The consumer permits `X-Forwarded-For` to influence rate-limit and observability client identity only when `API_RATE_LIMIT_TRUST_X_FORWARDED_FOR=true`, with explicit operator guidance that the ingress must sanitize the field first. This makes forwarding metadata part of the edge migration contract rather than an application-owned authentication fact.

Traefik documents that it normally adds `X-Forwarded-For`, `X-Real-Ip`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, and `X-Forwarded-Server` when proxying HTTP. Its current EntryPoint documentation separately requires explicit `trustedIPs` or insecure mode before incoming `X-Forwarded-*` values are trusted. The Pingora migration therefore cannot pass request-controlled forwarding identity through unchanged.

## Chosen contract

`forwarding_policy::ForwardingContext` receives only values observed at the accepted edge boundary: client IP, original Host authority, listener port, and characterized downstream scheme. Before the upstream request is sent, the policy removes request-controlled `Forwarded`, `X-Forwarded-*`, and `X-Real-IP`, then reconstructs:

- `X-Forwarded-For` from the accepted client IP;
- `X-Real-IP` from the same client IP;
- `X-Forwarded-Host` from the original Host authority;
- `X-Forwarded-Port` from the accepted listener address; and
- `X-Forwarded-Proto` from the characterized downstream scheme.

`Forwarded` is removed rather than synthesized because the observed consumer contract is the Traefik compatibility field family. `X-Forwarded-Server` is also removed: it describes proxy-host identity, and no pg-erd consumer behavior currently requires a replacement value. Fabricating either field would widen the migration contract without executable legacy evidence.

The characterized pg-erd Traefik routers use only the `web` EntryPoint, so the current migration adapter explicitly emits `X-Forwarded-Proto: http`. HTTPS is not inferred from an upstream TLS setting or arbitrary header. A future TLS listener migration must derive and test its own downstream TLS/scheme contract before cutover.

## RED / GREEN acceptance

Executable RED `tests/pg_erd_forwarding_contract.rs` requires hostile incoming proxy headers to be discarded and transport-derived compatibility fields to replace them. `tests/pg_erd_runtime_proxy_contract.rs` applies the same invariant through `MigrationGatewayProxy`'s transport-neutral helper.

The callback slice is not GREEN for traffic activation until an exact-head compiled-binary test proves the same behavior through a real Pingora listener and upstream observer. That test must demonstrate that a hostile client cannot preserve a forged forwarding identity and that direct client IP, original Host, listener port, and characterized scheme are what the upstream receives. A separate TLS listener case is required before any HTTPS claim.

## Sources

- `ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535`: `deploy/traefik/dynamic.yaml`, `.env.example`, `backend/app/rate_limit.py`, `backend/app/observability.py`, and `docs/api-security-checklist.md`.
- Cloudflare Pingora pinned source `09696b51bc59315353d96686355861604d0bb48c`: downstream `Session::client_addr` / `server_addr` and Pingora socket-address conversion used by the callback adapter.
- Traefik Labs. (n.d.). *Traefik getting started FAQ: Forwarded headers when proxying HTTP requests*. https://doc.traefik.io/traefik/getting-started/faq/
- Traefik Labs. (n.d.). *Traefik EntryPoints: Forwarded headers*. https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/
