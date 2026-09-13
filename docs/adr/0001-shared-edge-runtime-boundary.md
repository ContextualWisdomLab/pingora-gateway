# ADR 0001: Shared Edge Runtime Boundary

**Status:** Accepted for v1 development

## Context

CWL-managed Nginx behavior exists in multiple repositories. A shared replacement can reduce duplicate runtime/security work, but centralizing product routing would create a new coupling problem.

## Decision

`pingora-gateway` owns a Supporting/Generic edge-runtime subdomain. Its Edge Contract bounded context admits explicit network authority and remains independent of Pingora. Pingora is a delivery adapter. Consumer Core Domain routing, static semantics, auth and deployment-specific policy stay with the consumer.

V1 deliberately runs one upstream per process. Multiple upstreams imply routing or load-balancing semantics and therefore require a later versioned increment rather than an adapter-side heuristic.

## Consequences

Consumer migrations can reuse one runtime/security baseline while retaining product ownership. More processes may be required for independently owned upstreams. This is preferable to hidden shared routing policy.
