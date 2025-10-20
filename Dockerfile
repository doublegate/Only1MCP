# Only1MCP Dockerfile
# Multi-stage build for minimal production image

# =============================================================================
# Stage 1: Builder - Compile Rust binary
# =============================================================================
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user (will be used in runtime stage)
RUN useradd -m -u 1000 only1mcp

# Set working directory
WORKDIR /build

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY config ./config

# Build release binary
RUN cargo build --release

# =============================================================================
# Stage 2: Runtime - Minimal Debian image
# =============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user matching builder stage
RUN useradd -m -u 1000 only1mcp

# Create directories for config and data
RUN mkdir -p /etc/only1mcp /var/lib/only1mcp /var/log/only1mcp && \
    chown -R only1mcp:only1mcp /etc/only1mcp /var/lib/only1mcp /var/log/only1mcp

# Copy binary from builder
COPY --from=builder /build/target/release/only1mcp /usr/local/bin/only1mcp

# Copy default config template
COPY --from=builder /build/config/templates/solo.yaml /etc/only1mcp/only1mcp.yaml

# Set ownership
RUN chown only1mcp:only1mcp /usr/local/bin/only1mcp

# Switch to non-root user
USER only1mcp

# Set working directory
WORKDIR /home/only1mcp

# Expose proxy port (default 8080)
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV ONLY1MCP_CONFIG=/etc/only1mcp/only1mcp.yaml

# Run server in foreground mode (no daemonize in container)
ENTRYPOINT ["/usr/local/bin/only1mcp"]
CMD ["start", "--foreground", "--host", "0.0.0.0", "--port", "8080"]
