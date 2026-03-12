FROM rust:slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rust_base /app/
WORKDIR /app
CMD ["./rust_base"]
