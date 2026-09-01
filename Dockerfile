FROM rust:1.97.1-bookworm AS build
WORKDIR /src
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 65532 gateway \
    && useradd --system --uid 65532 --gid 65532 --home-dir /nonexistent --shell /usr/sbin/nologin gateway
COPY --from=build /src/target/release/pingora-gateway /usr/local/bin/pingora-gateway
USER 65532:65532
ENV PINGORA_GATEWAY_CONFIG=/etc/pingora-gateway/gateway.yaml
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/pingora-gateway"]
