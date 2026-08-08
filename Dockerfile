# syntax=docker/dockerfile:1
FROM alpine:3.20 AS base
RUN apk add --no-cache ca-certificates libssl3

FROM base AS node-holder
COPY dist/rnex-server-backend-node-holder /rnex-server-backend-node-holder
ENTRYPOINT ["/rnex-server-backend-node-holder"]

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