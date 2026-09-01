# Product Requirements Document

## Problem

CWL repositories contain CWL-managed Nginx/OpenResty/ingress-nginx runtimes whose edge behavior is duplicated and difficult to govern consistently. The organization needs one reusable Pingora transport runtime while preserving product ownership of product-specific routing and policy.

## Initial release requirement

The gateway must reject invalid versioned configuration before bind; listen on a configurable non-privileged address; expose liveness, readiness and metrics; proxy only explicitly configured HTTP/HTTPS origins; verify HTTPS certificates and hostnames; sanitize untrusted forwarded headers; bound header/body/time resources; emit credential-free structured logs and low-cardinality metrics; drain on Pingora's normal process shutdown; and run non-root in an OCI image compatible with a read-only root filesystem.

## Non-goals for this increment

Static serving, WebSocket policy, advanced load balancing, dynamic reload, TLS termination/ACME and Kubernetes Gateway API are separate increments. Product authentication, tenancy and domain routing rules remain with consumers.

## Acceptance

A local upstream fixture must pass through the built production gateway path. Invalid config and oversized requests must fail closed. Architecture fitness must prove domain routing code has no Pingora dependency. Security/dependency and OCI build gates must be visible on the exact PR head.
