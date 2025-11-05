FROM rust:alpine AS dev-container

RUN apk add --no-cache protobuf-dev git musl-dev lld openssl-dev openssl-libs-static

FROM dev-container as builder

WORKDIR /app

COPY . .

RUN git submodule update --init --recursive

RUN OPENSSL_LIB_DIR=/usr/lib OPENSSL_INCLUDE_DIR=/usr/include/openssl OPENSSL_STATIC=1 RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo test --target x86_64-unknown-linux-musl
RUN OPENSSL_LIB_DIR=/usr/lib OPENSSL_INCLUDE_DIR=/usr/include/openssl OPENSSL_STATIC=1 RUSTFLAGS="-C relocation-model=static -C linker=ld.lld" cargo build --release --target x86_64-unknown-linux-musl # 

FROM scratch AS node-holder
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/edge_node_holder_server /edge_node_holder_server
ENTRYPOINT ["/edge_node_holder_server"]


FROM scratch AS proxy-insecure-v1
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxy_insecure /proxy_insecure
ENTRYPOINT ["/proxy_insecure"]

FROM scratch AS proxy-secure-v1
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxy_secure /proxy_secure
ENTRYPOINT ["/proxy_secure"]


FROM scratch AS backend-auth
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend_server_insecure /backend_server_insecure
ENTRYPOINT ["/backend_server_secure"]

FROM scratch AS backend-secure
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend_server_secure /backend_server_secure
ENTRYPOINT ["/backend_server_secure"]

# make sure the final output container is the dev container so that we can use it from the devcontainer.json
FROM final as dev-container
