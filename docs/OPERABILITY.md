# Operability and Recovery

Startup is fail closed: the listener is not added until versioned configuration validates and upstream DNS resolves. Exit code 78 denotes rejected configuration. `/livez` reports process liveness after listener startup; `/readyz` currently means configuration was validated and the process is serving, not that every upstream is healthy. `/metrics` exposes bounded counters.

Pingora owns normal process signal handling and graceful shutdown/drain. Deployments must send SIGTERM, stop routing new traffic, allow the configured platform termination window, then SIGKILL only after that window. A consumer must test its platform's drain and rolling rollback behavior before protected migration.

Rollback is image/config based: retain the prior immutable consumer image and config; revert traffic to that exact digest if readiness, error rate or latency regress. This repository does not own ACME/certificate issuance. Downstream TLS termination remains with the platform until a separate ADR moves it.

Read-only-root deployments mount the config read-only and provide no application write path. The current Dockerfile still needs immutable base-image digests and a published image digest before release closure.
