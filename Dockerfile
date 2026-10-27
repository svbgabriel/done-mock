# Build stage - optimized multi-layer caching
FROM rust:1.99-slim AS builder
WORKDIR /app

# Install build dependencies for SQLx (native-tls)
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifest files first for dependency caching
COPY Cargo.toml Cargo.lock ./
# main.rs has to exist so we just touch it, gets overriden when we add src
RUN mkdir src; touch src/main.rs
RUN cargo fetch --locked

# Copy source code last (changes frequently, invalidates cache less often)
COPY src/ ./src/
COPY ui/ ./ui/

# Rebuild with source code (only recompiles changed crates)
RUN cargo build --release --locked

# Production stage - minimal distroless image
FROM gcr.io/distroless/cc-debian13:nonroot

WORKDIR /app
COPY --chown=nonroot:nonroot --from=builder /app/target/release/mock-server ./mock-server
COPY --chown=nonroot:nonroot --from=builder /app/ui ./ui

EXPOSE 8000
ENTRYPOINT ["./mock-server"]
