# Operability and Recovery

## Start

Run `cwl-pingora-gateway --config /path/to/gateway.yaml`. Configuration is read and validated before the listener is registered. Invalid or missing configuration exits non-zero.

## Health

`/livez` and `/readyz` return 200 with `Cache-Control: no-store` through the Pingora serving path. In v1 readiness is process/configuration readiness, not upstream reachability. Do not use it as proof that a dependent application is healthy.

## Shutdown

The binary delegates lifecycle handling to Pingora `Server::run_forever()`, whose documented server lifecycle handles termination/drain. This repository has not yet captured its own exact SIGTERM/in-flight-request GREEN test, so graceful shutdown is a release gap rather than a completed claim.

## Container

Run as a non-root user and prefer a read-only root filesystem. Mount only the versioned config read-only. The runtime does not intentionally write logs or state files; stdout/stderr should be collected by the platform. Do not bake secrets into the image or config.

## Rollback

A consumer migration must keep the last known-good deployment manifest/image digest and its behavior characterization. Roll back by restoring that exact protected deployment revision, not by editing a live container. Certificate management must remain with its existing bounded owner during edge-runtime rollback.

No consumer may pin `pingora-gateway` until a protected release publishes an immutable image digest and rollback has been rehearsed. The current Dockerfile alone is not a releasable artifact.
