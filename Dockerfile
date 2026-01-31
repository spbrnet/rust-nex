FROM rust:alpine AS build-container

RUN apk add --no-cache protobuf-dev git musl-dev lld openssl-dev openssl-libs-static yq bash

FROM build-container AS builder

WORKDIR /app

COPY . .

ARG EDITION

RUN git submodule update --init --recursive

RUN ./test-edition.sh
RUN ./build-edition.sh

FROM scratch AS node-holder
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/edge_node_holder_server /edge_node_holder_server
ENTRYPOINT ["/edge_node_holder_server"]


FROM scratch AS proxy-insecure
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM scratch AS proxy-secure
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]


FROM scratch AS backend-auth
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend_server_insecure /backend_server_insecure
ENTRYPOINT ["/backend_server_insecure"]

FROM scratch AS backend-secure
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend_server_secure /backend_server_secure
ENTRYPOINT ["/backend_server_secure"]

# make sure the final output container is the dev container so that we can use it from the devcontainer.json
FROM build-container AS dev-container
RUN apk add openjdk21-jdk gcompat
