# syntax=docker/dockerfile:1

# ── builder stage ─────────────────────────────────────────────────────────────
# Compiles the WASM contracts. This stage carries the full Rust toolchain and
# is discarded from the final image.
FROM rust:1.82-slim AS builder

# Install wasm target and stellar CLI
RUN rustup target add wasm32-unknown-unknown && \
    cargo install --locked stellar-cli --version 22.0.0

WORKDIR /app
COPY . .

# Build all contracts
RUN stellar contract build

# ── test stage ────────────────────────────────────────────────────────────────
FROM builder AS test
CMD ["cargo", "test"]

# ── runtime stage ─────────────────────────────────────────────────────────────
# Minimal image holding only the compiled WASM artifacts (< 50 MB).
FROM scratch AS runtime

WORKDIR /app
COPY --from=builder /app/target/wasm32-unknown-unknown/release/*.wasm /app/
