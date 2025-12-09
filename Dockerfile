# syntax=docker/dockerfile:1
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:trixie-slim
COPY --from=builder ./target/release/npm-download-stats-otel-exporter ./target/release/npm-download-stats-otel-exporter
USER 1000
ENTRYPOINT ["./target/release/npm-download-stats-otel-exporter"]
