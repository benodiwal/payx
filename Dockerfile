FROM rust:1.85-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/payx-server/Cargo.toml crates/payx-server/Cargo.toml
COPY crates/payx-cli/Cargo.toml crates/payx-cli/Cargo.toml

RUN mkdir -p crates/payx-server/src crates/payx-cli/src && \
    echo "fn main() {}" > crates/payx-server/src/main.rs && \
    echo "pub fn lib() {}" > crates/payx-server/src/lib.rs && \
    echo "fn main() {}" > crates/payx-cli/src/main.rs

RUN cargo build --release && rm -rf crates/*/src

COPY crates crates
RUN touch crates/payx-server/src/main.rs crates/payx-server/src/lib.rs crates/payx-cli/src/main.rs && \
    cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/payx-server /usr/local/bin/payx-server
COPY --from=builder /app/target/release/payx /usr/local/bin/payx
COPY --from=builder /app/crates/payx-server/migrations /app/migrations

ENV RUST_LOG=info
EXPOSE 8080

CMD ["payx-server"]
