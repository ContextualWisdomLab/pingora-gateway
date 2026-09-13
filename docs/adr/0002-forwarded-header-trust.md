# ADR 0002: Distrust Downstream Forwarding Identity

**Status:** Accepted for v1 development

## Context

`Forwarded` and `X-Forwarded-*` are security-sensitive because trusting arbitrary downstream values can spoof client IP, scheme, or proxy provenance. The first runtime has no trusted-proxy CIDR/hop contract.

## Decision

Delete inbound `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`, and `X-Real-IP` before sending upstream. Because the v1 listener is cleartext, emit only `Forwarded: proto=http`. Do not assert a client IP.

## Consequences

Upstreams cannot obtain end-client IP from v1 and must not infer it from legacy forwarding headers. A future trusted-proxy feature requires an explicit trust-source contract, RFC 7239 formatting, spoofing/chain tests, and consumer justification.
