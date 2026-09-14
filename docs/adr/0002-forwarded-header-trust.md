# ADR 0002: Distrust Downstream Forwarding Identity

**Status:** Proposed

## Context

`Forwarded` and `X-Forwarded-*` are security-sensitive because trusting arbitrary downstream values can spoof client IP, host, port, scheme, proxy provenance, or client-certificate identity. Generic v1 has no trusted-proxy CIDR/hop contract and does not terminate downstream mTLS, so any inbound `X-Forwarded-Client-Cert` value is untrusted caller input rather than gateway-owned certificate evidence.

## Decision

Delete inbound `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, `X-Forwarded-Client-Cert`, and `X-Real-IP` before sending upstream. Because the v1 listener is cleartext, emit only gateway-owned `Forwarded: proto=http`. Do not assert a client IP, client-certificate identity, or downstream proxy provenance.

`X-Forwarded-Client-Cert` is treated as an identity-bearing proxy header rather than generic application metadata. Envoy documents XFCC as carrying client/proxy certificate identity and sanitizes it by default unless an explicit forwarding policy is configured. Generic v1 has no such trust policy, so preserving a caller-supplied XFCC value would cross the identity boundary owned outside this gateway.

## Consequences

Upstreams cannot obtain end-client IP or client-certificate identity from generic v1 and must not infer identity or provenance from legacy forwarding headers. A future trusted-proxy or downstream-mTLS feature requires an explicit trust-source contract, certificate-verification ownership, RFC 7239 semantics where applicable, spoofing/chain tests, and consumer justification before this ADR can advance beyond Proposed.
