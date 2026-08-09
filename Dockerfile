# syntax=docker/dockerfile:1
FROM debian:bookworm-slim AS base
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

FROM base AS proxy-insecure
COPY dist/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM base AS proxy-secure
COPY dist/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]

FROM base AS backend-auth
COPY dist/rnex-server-backend-auth /rnex-server-backend-auth
ENTRYPOINT ["/rnex-server-backend-auth"]

FROM base AS backend-secure
COPY dist/rnex-server-backend-secure /rnex-server-backend-secure
ENTRYPOINT ["/rnex-server-backend-secure"]