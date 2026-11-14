# Build stage - statically linked binary via musl-cross
# Supports linux/amd64 and linux/arm64 via TARGETARCH (set automatically by Docker BuildX)
ARG TARGETARCH
FROM ghcr.io/rust-cross/rust-musl-cross:${TARGETARCH}-musl AS builder
WORKDIR /home/rust/src

# Copy manifest files first for dependency caching
COPY Cargo.toml Cargo.lock ./
# main.rs has to exist so we just touch it, gets overridden when we add src
RUN mkdir -p src && touch src/main.rs
RUN cargo fetch --locked

# Copy source code last (changes frequently, invalidates cache less often)
COPY src/ ./src/
COPY ui/ ./ui/

# Build static binary (target is pre-configured in the image)
RUN cargo build --release --locked

# Strip binary to reduce size
RUN musl-strip target/${TARGETARCH}-unknown-linux-musl/release/done-mock

# Production stage - static distroless (no C standard library needed)
FROM gcr.io/distroless/static-debian13:nonroot
WORKDIR /app
COPY --chown=nonroot:nonroot --from=builder /home/rust/src/target/${TARGETARCH}-unknown-linux-musl/release/done-mock ./done-mock
COPY --chown=nonroot:nonroot --from=builder /home/rust/src/ui ./ui
EXPOSE 8000
ENTRYPOINT ["./done-mock"]
