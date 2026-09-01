# syntax=docker/dockerfile:1.7
FROM rust:1.98.0-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release --bin cwl-pingora-gateway

# Debian 12 distroless reaches the end of its supported image line in September 2026. The
# Debian 13 `cc` image is the narrow runtime intended for dynamically linked Rust/C-family
# binaries: it adds libgcc and its dependencies to distroless/base without restoring a shell,
# package manager, or unrelated userland. CI exercises this exact image read-only with all Linux
# capabilities dropped; that caught the narrower `base` image's missing libgcc_s.so.1 before
# release. The digest is immutable so dependency updates remain explicit reviewable changes.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:d97bc0a941b8d4be647dc0ee75b264ddbb772f1ac5ba690a4309c00723b23775 AS runtime

COPY --from=builder /src/target/release/cwl-pingora-gateway /usr/local/bin/cwl-pingora-gateway

# The process requires no writable application directory. Operators can run the image with a
# read-only root filesystem and mount only the versioned configuration as a read-only file.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/cwl-pingora-gateway"]
