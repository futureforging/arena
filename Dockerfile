# Dockerfile for Arena Omnia Echo Runtime

FROM rust:alpine AS build

RUN apk add --no-cache build-base cmake perl
# Add wasm32-wasip2 target (rust-toolchain.toml excluded so we use image's rust)
RUN rustup target add wasm32-wasip2

WORKDIR /app

COPY . .

# Build guest (WASM) and runtime
RUN cargo build -p arena-guest --target wasm32-wasip2 --release && \
    cargo build -p arena-runtime --release

# Runtime stage
FROM alpine:latest

RUN adduser -D -u 10001 appuser

COPY --from=build /app/target/wasm32-wasip2/release/arena_guest.wasm /app/arena_guest.wasm
COPY --from=build /app/target/release/arena-runtime /bin/arena-runtime

USER appuser
EXPOSE 8080

ENTRYPOINT ["/bin/arena-runtime", "run"]
CMD ["/app/arena_guest.wasm"]
