# ---- build stage ----
FROM rust:1-slim AS builder
WORKDIR /app
# system deps for building (openssl for TLS to Supabase)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# ---- runtime stage (small image) ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tao-catalog-service /app/tao-catalog-service
ENV RUST_LOG=info
EXPOSE 8080
CMD ["/app/tao-catalog-service"]
