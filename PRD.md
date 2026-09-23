# Product Requirements — CWL Pingora Gateway

## Problem

CWL repositories contain several CWL-managed Nginx deployments with duplicated proxy/static/ingress behavior and uneven security evidence. Replacing those runtimes safely requires one reusable edge-runtime owner that is narrow enough not to absorb consumer business policy.

## Users

Primary users are CWL product maintainers and platform operators migrating an owned reverse-proxy boundary. Security and governance reviewers consume the same configuration, test, release, and operational evidence.

## v1 outcome

The first release-quality vertical SHALL:

- parse a strict versioned configuration before opening network authority;
- run a Rust Pingora HTTP proxy as a non-root process on an explicit listener;
- admit exactly one explicit upstream per process, with HTTP or verified HTTPS transport;
- require positive connect/total-connect/read/write/idle budgets;
- bound request bodies, including declared length before upstream selection and streamed bytes during transfer;
- serve liveness and readiness through the production Pingora path;
- use Pingora's standards-oriented hop-by-hop policy and distrust inbound forwarding identity;
- shut down through Pingora's graceful server lifecycle rather than a custom signal loop;
- support a read-only-root container layout; and
- ship tests and documentation sufficient to evaluate a consumer migration without reading implementation internals first.

## Non-goals for v1

No product-specific routes, CDN logic, static-site fallback, downstream TLS termination, ACME/certificate issuance, WebSocket policy, load balancing, dynamic reload, Kubernetes Gateway API controller, or user-controlled forward-proxy destination is included. Those require separate behavior-preserving consumer evidence.

## Release gates

A release is blocked until exact-head tests/checks pass, the branch is integrated under live policy, independent review requirements are satisfied, a committed dependency lock exists, the OCI image is built and exercised non-root/read-only-root, SBOM/provenance are published, an immutable digest is recorded, graceful-drain behavior is tested, security advisories are revalidated, and the gap baseline contains no release-blocking item.
