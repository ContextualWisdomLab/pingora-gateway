# Agent Engineering Contract

This repository owns the reusable CWL Pingora edge runtime. Preserve normal pull-request governance and treat every remembered SHA, check, review, release, image digest, and dependency status as stale until refetched.

## DDD boundary

Active edge configuration is transport-neutral. `src/edge_contract.rs` defines admitted network-authority invariants and must not import Pingora types. `src/edge_routing.rs` and `src/http_policy.rs` may characterize observed shared-edge routing/HTTP behavior before runtime activation, but they must remain transport-neutral and must not become product-domain policy stores. `src/gateway_proxy.rs`, `src/pingora_delivery.rs`, and the binary are delivery/application composition. Product-specific authorization, business routing, static-site semantics, authentication policy, and domain state remain in consumer bounded contexts. Wardnet/EgressWeave security verdicts and Keyverse identity remain with their canonical owners. Do not create generic `utils`, `helpers`, `common`, or product-policy `core` dumping grounds.

The active v1 `GatewayConfig` aggregate admits one listener, one upstream authority, a positive request-body limit, an explicit TLS identity when TLS is enabled, and positive upstream I/O budgets. Characterized route tables or response-header policies do not become active merely because their transport-neutral modules exist. Any activation requires an explicit versioned configuration/runtime transition and executable consumer parity evidence.

## Change method

Characterize behavior before replacing it. Prefer the smallest causal change, realistic RED evidence, focused GREEN, then full exact-head CI. A migration is not complete because Nginx/Traefik strings disappeared. Consumer migrations must preserve the behavior actually used and must pin a real immutable gateway artifact or prove a more appropriate managed hosting boundary.

Do not force-push, destructively rebase, self-approve, weaken required checks, invent secrets or reviewers, or create self-modifying writer workflows. Security/scanner failures are fixed at their true owner; do not patch product code to appease a broken central scanner.

## Security invariants

Treat configuration and requests as untrusted. Arbitrary request-controlled upstream selection is out of scope. TLS peers verify certificates and hostnames. Client-supplied forwarding identity is untrusted. Edge-owned response-header configuration fails closed on ambiguous field authority and CR/LF values; product authorization/business response semantics are not absorbed into HTTP Policy. Logs and metrics must never include authorization headers, cookies, tokens, configuration credentials, or unbounded route labels. Request bodies and I/O budgets remain bounded. Container execution stays non-root and compatible with a read-only root filesystem.
