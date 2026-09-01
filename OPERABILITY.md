# Operability and Recovery

## Start

Run `cwl-pingora-gateway --config /path/to/gateway.yaml`. Configuration is read and validated before the listener is registered. Invalid or missing configuration exits non-zero.

## Health

`/livez` and `/readyz` return 200 with `Cache-Control: no-store` through the Pingora serving path. In v1 readiness is process/configuration readiness, not upstream reachability. Do not use it as proof that a dependent application is healthy.

## Retry and shutdown

Version 1 does not inherit Pingora's retry or drain defaults. `src/runtime_policy.rs` sets Pingora's `max_retries` field to `1`; in the pinned proxy loop that value means one total upstream attempt, so the generic edge performs no automatic retry. Retry policy requires request-idempotency knowledge and stays with a later explicit contract rather than being inferred by the gateway.

SIGTERM uses Pingora graceful termination with an explicit 5-second request-drain grace period and a 10-second runtime-shutdown timeout. The pinned Pingora server calls Tokio `Runtime::shutdown_timeout` with that timeout and then sleeps for the same timeout while service runtimes are shut down in parallel. The v1 policy therefore requires a 30-second supervisor hard-kill budget: its modeled worst-case Pingora process budget is 25 seconds plus scheduler/process-exit overhead. A Kubernetes-style deployment must set `terminationGracePeriodSeconds` to at least 30 or provide an equivalent supervisor budget; a shorter external kill deadline is not an admitted deployment contract.

`tests/graceful_shutdown.rs` exercises the compiled binary with a held upstream response, sends SIGTERM only after the request is in flight, requires the response to finish during the 5-second grace period, and requires clean process exit before the 30-second supervisor budget. This test must be terminal-success on the exact release candidate; predecessor-head success never transfers.

## Container

Run as a non-root user and prefer a read-only root filesystem. Mount only the versioned config read-only. The runtime does not intentionally write logs or state files; stdout/stderr should be collected by the platform. Do not bake secrets into the image or config.

## Rollback

A consumer migration must keep the last known-good deployment manifest/image digest and its behavior characterization. Roll back by restoring that exact protected deployment revision, not by editing a live container. Certificate management must remain with its existing bounded owner during edge-runtime rollback.

No consumer may pin `pingora-gateway` until a protected release publishes an immutable image digest and rollback has been rehearsed. The current Dockerfile alone is not a releasable artifact.
