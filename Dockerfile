FROM node:22-slim AS ts-builder

RUN corepack enable

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY ts ./ts
RUN pnpm run build

FROM rust:1.93 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY src ./src
COPY --from=ts-builder /app/ts/dist ./ts/dist
COPY migrations ./migrations

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin bookworm

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates iptables && rm -rf /var/lib/apt/lists/*

COPY --from=docker.io/tailscale/tailscale:stable /usr/local/bin/tailscaled /app/tailscaled
COPY --from=docker.io/tailscale/tailscale:stable /usr/local/bin/tailscale /app/tailscale

COPY --from=builder /app/target/release/bookworm /app/bookworm
COPY start.sh /app/start.sh
RUN chmod +x /app/start.sh

CMD ["/app/start.sh"]
