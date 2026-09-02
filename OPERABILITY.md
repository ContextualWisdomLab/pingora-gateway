# Operability and Recovery

## Start

Run the generic process as `cwl-pingora-gateway --config /path/to/gateway.yaml`. Configuration is read and validated before the listener is registered. Invalid or missing configuration exits non-zero. `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size` are mandatory positive deployment inputs; the process will not start with a zero value or silently inherit a Pingora keepalive default. If a TLS upstream declares `trust_bundle_file`, that absolute PEM bundle is read and parsed during peer activation before listeners open; unreadable, empty, or malformed trust material prevents activation.

The pg-erd migration candidate is a separate executable: `cwl-pingora-pg-erd-migration --config /path/to/pg-erd-migration.yaml`. It consumes the bounded `PgErdMigrationConfig` profile rather than widening generic `GatewayConfig` v1. The profile admits exactly the compiled `backend` and `frontend` transport identities plus deployment-variable sockets, runtime budgets and upstream transport/TLS values. Route tables, response-policy fields, product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, service discovery and arbitrary destinations are not operator-configurable.

Admin parsing is side-effect free with respect to custom trust bytes. It validates the exact transport-authority set and `UpstreamConfig` invariants first. `build_proxy` then materializes Pingora peers and any custom PEM trust bundle once, still before listener registration. This avoids reading mutable trust material in a validation pass and reading it again for activation.

Trust bundles are deployment input, not certificate-authority ownership. Mount them read-only from the platform or canonical secret/certificate owner and rotate them by replacing the deployment revision. The gateway does not issue certificates, manage ACME, or write trust material.

## Health and backpressure

Both process identities reserve `/livez` and `/readyz` and return 200 with `Cache-Control: no-store` through the Pingora serving path. Readiness is process/configuration readiness, not upstream reachability. Do not use it as proof that a dependent application is healthy. For the pg-erd migration, consumer `/healthz` remains routed application traffic to `backend`; it is intentionally distinct from the gateway-local probes.

`max_in_flight_requests` limits concurrently admitted non-health requests for one gateway process. At capacity the gateway fails new application traffic fast with HTTP 503 and increments `cwl_pingora_gateway_backpressure_rejections_total`; it does not queue unbounded work. Process health probes bypass that admission budget so operators can distinguish process health from traffic saturation. The request lease is released when the Pingora request context ends, including error paths, and a subsequent request is admissible again.

`upstream_keepalive_pool_size` is copied into Pingora `ServerConf` before bootstrap. Choose it with expected upstream concurrency, origin capacity, instance count and connection reuse in mind. It limits retained reusable upstream connections; it is not a substitute for the downstream in-flight admission limit and does not create product-domain load-balancing semantics.

## Forwarding and protocol boundary

The generic v1 adapter and the pg-erd migration adapter have different compatibility forwarding contracts. Generic v1 strips inbound proxy-identity fields and emits only `Forwarded: proto=http`. The pg-erd migration adapter removes request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and legacy `X-Forwarded-Server` authority, then rebuilds only the characterized `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host`, `X-Forwarded-Port`, and `X-Forwarded-Proto` fields from accepted downstream transport/request authority. Product identity, tenant identity and authorization are never derived from these transport fields by the shared gateway.

The captured pg-erd Traefik entryPoint is clear-text `web`, so this migration profile currently uses downstream scheme `http`. Do not deploy it behind a TLS listener and assume `X-Forwarded-Proto: https` parity. Downstream TLS, HTTP/2, HTTP/3, WebSocket/upgrade and streaming behavior require separate executable contracts before they become admitted migration behavior.

## Retry and shutdown

The runtime does not inherit Pingora's retry, keepalive-pool, or drain defaults. `src/runtime_policy.rs` sets Pingora's `max_retries` field to `1`; in the pinned proxy loop that value means one total upstream attempt, so the shared generic edge performs no automatic retry. The pg-erd composition root uses the same server policy. Retry policy requires request-idempotency knowledge and stays with a later explicit contract rather than being inferred by the gateway.

SIGTERM uses Pingora graceful termination with an explicit 5-second request-drain grace period and a 10-second runtime-shutdown timeout. The pinned Pingora server calls Tokio `Runtime::shutdown_timeout` with that timeout and then sleeps for the same timeout while service runtimes are shut down in parallel. The policy therefore requires a 30-second supervisor hard-kill budget: its modeled worst-case Pingora process budget is 25 seconds plus scheduler/process-exit overhead. A Kubernetes-style deployment must set `terminationGracePeriodSeconds` to at least 30 or provide an equivalent supervisor budget; a shorter external kill deadline is not an admitted deployment contract.

`tests/graceful_shutdown.rs` exercises the generic compiled binary with a held upstream response. `tests/production_path.rs` covers generic saturation, health and failure recovery. `tests/pg_erd_production_path.rs` exercises the dedicated pg-erd process with real loopback backend/frontend origins, including process-local health, characterized route/response-header behavior, transport-derived forwarding replacement and declared body rejection. `tests/pg_erd_runtime_isolation_traffic.rs` covers dedicated streamed-body rejection plus saturation/recovery, while `tests/pg_erd_upstream_failure_traffic.rs` requires refused-backend failure to remain bounded and leave readiness plus the independent frontend route usable. `tests/pg_erd_binary_startup.rs` requires missing/invalid configuration and unreadable trust material to fail before listener activation. These source contracts become evidence only after terminal success on the exact current head; predecessor success never transfers.

## Container

Run as a non-root user and prefer a read-only root filesystem. Mount only the versioned config and any required upstream trust bundle read-only. The runtime does not intentionally write logs or state files; stdout/stderr should be collected by the platform. Do not bake secrets or private keys into the image or config.

The Dockerfile has one shared `runtime-common` hardening stage and two explicit final targets. Default `docker build .` still resolves to `gateway` and packages only `cwl-pingora-gateway`; the pg-erd candidate requires explicit `--target pg-erd-migration` and packages only `cwl-pingora-pg-erd-migration`. This prevents a migration image from silently changing generic v1 deployment identity while keeping both images on the same digest-pinned distroless base and uid/gid `65532` boundary.

The OCI gate builds both exact-source targets, verifies their distinct entrypoints and declared non-root user, starts both with read-only root filesystems, all Linux capabilities dropped and `no-new-privileges`, mounts only their versioned config read-only, and requires `/livez` to become available. The supply-chain gate separately vulnerability-scans both final images and binds both local image IDs plus the shared dependency SBOM/policy evidence to the exact source SHA. These are unreleased candidate checks: an immutable registry digest, signature/attestation policy, reproducibility and rollback rehearsal remain required before deployment.

`examples/pg-erd-migration.yaml` exists only as a deterministic container-start fixture. Its loopback upstream ports do not represent consumer production authority and are never evidence of a pg-erd deployment. Real deployment configuration must bind the characterized `backend` and `frontend` identities to reviewed concrete transport authority.

## Cutover and rollback

A consumer migration must keep the last known-good deployment manifest/image digest and its executable legacy characterization. Route and HTTP-policy parity must first be terminal GREEN through the compiled dedicated candidate, then exercised through explicit shadow/canary evidence before production cutover. A source PR or listener-capable binary is not traffic-state evidence.

Roll back by restoring the exact protected prior deployment revision, not by editing a live container. Certificate management, identity, product authorization/business policy, and security-verdict ownership must remain with their existing bounded owners during edge-runtime rollback.

No consumer may pin `pingora-gateway` until a protected release publishes an immutable image digest and rollback has been rehearsed. Dedicated migration OCI source acceptance is now defined, but it remains uncredited until the exact candidate head executes terminal-success together with the rest of its quality/security gates. Routed load/failure/drain evidence and release/canary eligibility remain separate prerequisites.
