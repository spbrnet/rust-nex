# syntax=docker/dockerfile:1
FROM rust:alpine AS chef
RUN apk add --no-cache musl-dev lld g++ make
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apk add --no-cache protobuf-dev git openssl-dev openssl-libs-static bash yq

COPY --from=planner /app/recipe.json recipe.json
ARG EDITION
ARG DATABASE_URL

RUN --mount=type=cache,id=${EDITION}-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${EDITION}-target,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl && \
    cargo chef cook --tests --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,id=${EDITION}-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,,id=${EDITION}-target,target=/app/target \
    RNEX_STATIC=1 ./test-edition.sh && RNEX_STATIC=1 ./build-edition.sh && \
    mkdir -p /app/dist && \
    cp /app/target/x86_64-unknown-linux-musl/release/edge_node_holder_server /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/proxy_insecure /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/proxy_secure /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/backend_server_insecure /app/dist/ && \
    cp /app/target/x86_64-unknown-linux-musl/release/backend_server_secure /app/dist/


FROM scratch AS node-holder
COPY --from=builder /app/dist/edge_node_holder_server /edge_node_holder_server
ENTRYPOINT ["/edge_node_holder_server"]

FROM scratch AS proxy-insecure
COPY --from=builder /app/dist/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM scratch AS proxy-secure
COPY --from=builder /app/dist/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]

FROM scratch AS backend-auth
COPY --from=builder /app/dist/backend_server_insecure /backend_server_insecure
ENTRYPOINT ["/backend_server_insecure"]

FROM scratch AS backend-secure
COPY --from=builder /app/dist/backend_server_secure /backend_server_secure
ENTRYPOINT ["/backend_server_secure"]

FROM chef AS dev-container
RUN apk add --no-cache openjdk21-jdk gcompat git bash protobuf-dev
COPY --from=builder /app/dist/* /usr/local/bin/
