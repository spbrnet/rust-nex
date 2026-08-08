# syntax=docker/dockerfile:1
FROM scratch AS node-holder
COPY dist/rnex-server-backend-node-holder /rnex-server-backend-node-holder
ENTRYPOINT ["/rnex-server-backend-node-holder"]

FROM scratch AS proxy-insecure
COPY dist/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM scratch AS proxy-secure
COPY dist/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]

FROM scratch AS backend-auth
COPY dist/rnex-server-backend-auth /rnex-server-backend-auth
ENTRYPOINT ["/rnex-server-backend-auth"]

FROM scratch AS backend-secure
COPY dist/rnex-server-backend-secure /rnex-server-backend-secure
ENTRYPOINT ["/rnex-server-backend-secure"]