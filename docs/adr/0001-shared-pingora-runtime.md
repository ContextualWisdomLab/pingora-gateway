# ADR 0001: Shared Pingora runtime boundary

Status: Accepted, 2026-09-01.

## Decision

`ContextualWisdomLab/pingora-gateway` is the canonical reusable CWL-managed Pingora edge runtime. Edge Routing stays transport-independent and Pingora is isolated in a delivery adapter. Product-specific policy remains in consumer bounded contexts.

The initial dependency pins upstream commit `09696b51bc59315353d96686355861604d0bb48c` rather than Pingora 0.8.1 because Cloudflare's 2026-08-24 sync merged the fix moving in-tree Pingora use from advisory-affected `lru` 0.16.x to 0.18.2 after the 0.8.1 release. The pin must be re-evaluated when a newer release contains equivalent fixes.

## Consequences

Consumers gain one hardened transport primitive, but migrations must characterize Nginx behavior instead of substituting image names. An unreleased upstream commit increases provenance/reproducibility burden; therefore exact commit traceability and dependency audit are mandatory until a suitable release exists.
