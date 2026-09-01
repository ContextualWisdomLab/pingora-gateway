# syntax=docker/dockerfile:1.7
FROM rust:1.97.1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --bin cwl-pingora-gateway

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/cwl-pingora-gateway /usr/local/bin/cwl-pingora-gateway

# The process requires no writable application directory. Operators can run the image with a
# read-only root filesystem and mount only the versioned configuration as a read-only file.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/cwl-pingora-gateway"]
