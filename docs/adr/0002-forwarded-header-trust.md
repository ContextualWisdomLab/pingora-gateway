# ADR 0002: Distrust Downstream Forwarding Identity

**Status:** Accepted for v1 development

## Context

`Forwarded` and `X-Forwarded-*` are security-sensitive because trusting arbitrary downstream values can spoof client IP, host, port, scheme, or proxy provenance. The first runtime has no trusted-proxy CIDR/hop contract.

## Decision

Delete inbound `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP` before sending upstream. Because the v1 listener is cleartext, emit only gateway-owned `Forwarded: proto=http`. Do not assert a client IP or preserve downstream proxy provenance.

## Consequences

Upstreams cannot obtain end-client IP from generic v1 and must not infer identity or provenance from legacy forwarding headers. A future trusted-proxy feature requires an explicit trust-source contract, RFC 7239 formatting, spoofing/chain tests, and consumer justification.
