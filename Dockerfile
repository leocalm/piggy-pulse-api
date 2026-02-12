# Build stage
FROM rust:1.93.0 AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
# Needed for sqlx::migrate! compile-time embedding.
COPY migrations ./migrations
COPY sqlx.toml ./sqlx.toml

# Install any required tools
RUN rustup component add rustfmt clippy

# Run format check
# RUN cargo fmt -- --check

# Run linter
# RUN cargo clippy --all-targets -- -D warnings

# Run tests
RUN cargo test --all --release

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/budget /app/budget

# Copy configuration examples (will be overridden by env vars)
COPY Budget.toml.example ./Budget.toml
COPY Rocket.toml ./Rocket.toml

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

# Create a non-root user and set permissions
RUN useradd -m -u 1000 appuser \
    && sed -i 's/\r$//' /usr/local/bin/docker-entrypoint.sh \
    && chmod +x /usr/local/bin/docker-entrypoint.sh \
    && chown -R appuser:appuser /app

# Expose the default port
EXPOSE 8000

# Switch to non-root user
USER appuser

# Use entrypoint script through /bin/sh for portability
ENTRYPOINT ["/bin/sh", "/usr/local/bin/docker-entrypoint.sh"]
