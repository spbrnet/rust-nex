# syntax=docker/dockerfile:1
FROM rust:alpine AS chef
RUN apk add --no-cache musl-dev lld g++ make
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apk add --no-cache protobuf-dev git openssl-dev openssl-libs-static bash yq ca-certificates

COPY --from=planner /app/recipe.json recipe.json
ARG EDITION
ARG DATABASE_URL

RUN --mount=type=cache,id=${EDITION}-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${EDITION}-target,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl && \
    cargo chef cook --tests --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,id=${EDITION}-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${EDITION}-target,target=/app/target \
    RNEX_STATIC=1 ./test-edition.sh && RNEX_STATIC=1 ./build-edition.sh && \
    mkdir -p /app/dist && \
    cp /app/target/x86_64-unknown-linux-musl/release/rnex-server-backend-node-holder /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/proxy_insecure /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/proxy_secure /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/rnex-server-backend-auth /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/rnex-server-backend-secure /app/dist/


FROM scratch AS node-holder
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/dist/rnex-server-backend-node-holder /rnex-server-backend-node-holder
ENTRYPOINT ["/rnex-server-backend-node-holder"]

FROM scratch AS proxy-insecure
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/dist/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM scratch AS proxy-secure
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/dist/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]

FROM scratch AS backend-auth
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/dist/rnex-server-backend-auth /rnex-server-backend-auth
ENTRYPOINT ["/rnex-server-backend-auth"]

FROM scratch AS backend-secure
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/dist/rnex-server-backend-secure /rnex-server-backend-secure
ENTRYPOINT ["/rnex-server-backend-secure"]
