# Build stage - download dependencies first for better caching
FROM rust:1.99-slim AS dependency
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked
RUN mkdir -p target && cargo build --release --target-dir target

# Build stage - compile the application
FROM rust:1.99-slim AS builder
WORKDIR /app

# Install build dependencies for SQLx (native-tls)
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy dependency layer from previous stage
COPY --from=dependency /app/target ./target
COPY Cargo.toml Cargo.lock ./

# Copy source code and build
COPY src/ ./src/
COPY ui/ ./ui/
RUN cargo build --release --locked

# Production stage - minimal distroless image
FROM gcr.io/distroless/static-debian12

WORKDIR /app
COPY --from=builder /app/target/release/mock-server ./mock-server
COPY --from=builder /app/ui ./ui

EXPOSE 8000
ENTRYPOINT ["./mock-server"]
