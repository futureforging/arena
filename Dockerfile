FROM rust:slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/arena /app/
COPY --from=builder /app/ref /app/ref
WORKDIR /app
CMD ["/app/arena"]
