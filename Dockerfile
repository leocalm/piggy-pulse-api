# Build stage
FROM rust:1.93.0 AS builder

WORKDIR /app

# Install sqlx-cli for migrations
RUN cargo install sqlx-cli --version 0.7.4 --locked --no-default-features --features rustls,postgres

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY sqlx.toml ./sqlx.toml

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/budget /app/budget

# Copy sqlx-cli for migrations
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Copy configuration examples (will be overridden by env vars)
COPY Budget.toml.example ./Budget.toml
COPY Rocket.toml ./Rocket.toml

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

# Create a non-root user and set permissions
RUN useradd -m -u 1000 appuser \
    && chmod +x /usr/local/bin/docker-entrypoint.sh \
    && chown -R appuser:appuser /app

# Expose the default port
EXPOSE 8000

# Switch to non-root user
USER appuser

# Use entrypoint script
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
