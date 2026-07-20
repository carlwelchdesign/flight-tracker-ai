FROM rust:1.97.1-bookworm AS development

WORKDIR /workspace

CMD ["cargo", "run", "-p", "flight-tracker-api"]

FROM rust:1.97.1-bookworm AS builder

WORKDIR /workspace
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps/api ./apps/api
COPY migrations ./migrations
RUN cargo build --locked --release -p flight-tracker-api

FROM debian:bookworm-slim AS production

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/flight-tracker-api /usr/local/bin/flight-tracker-api

USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["flight-tracker-api"]
