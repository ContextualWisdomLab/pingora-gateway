# ADR 0002: Forwarded-header trust starts closed

Status: Accepted, 2026-09-01.

The initial runtime does not infer a trusted-proxy chain. It removes inbound `Forwarded`, `X-Forwarded-*` and `X-Real-IP` before sending upstream and retains Pingora's default hop-by-hop normalization. A future consumer that requires preserved client identity must define trusted proxy CIDRs, append/replace semantics and tests as a separate contract. Blind passthrough is prohibited.
