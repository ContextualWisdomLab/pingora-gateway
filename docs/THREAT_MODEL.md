# Threat Model

Assets are upstream confidentiality/integrity, gateway availability, routing correctness and audit evidence. Adversaries include unauthenticated clients, malicious/mistaken operators supplying configuration, compromised upstream DNS, and dependency/supply-chain attackers.

Primary threats: request smuggling through hop-by-hop ambiguity; SSRF through request-controlled or rebinding upstream selection; certificate/SNI bypass; memory/connection exhaustion via headers/bodies/timeouts; credential leakage in logs; route shadowing/ambiguity; privilege escalation in the container; and stale vulnerable Pingora/dependencies.

Current controls are Pingora's normalized hop-by-hop handling, removal of untrusted forwarding headers, startup DNS resolution to a fixed validated socket, explicit private-network opt-in, TLS verification, deterministic duplicate-free route precedence, resource limits, non-root execution and dependency CI. Residual gaps include no downstream TLS termination, no trusted-proxy client-IP model, no fuzz corpus, no committed lockfile yet, no immutable published image digest yet and no recovery drill evidence yet.
