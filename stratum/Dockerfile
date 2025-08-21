# Multi-stage build for Loka Stratum
# Stage 1: Build stage
FROM rust:1.88-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    curl \
    vim

# Create app directory
WORKDIR /app

# Copy entire workspace
COPY . /workspace/

# Set working directory to stratum
WORKDIR /workspace/stratum

# Build the application in release mode using workspace
RUN cd /workspace && cargo build --release -p loka-stratum

# Stage 2: Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    ca-certificates \
    tzdata \
    tini \
    curl \
    vim

# Create a non-root user
RUN addgroup -g 1001 -S loka && \
    adduser -u 1001 -S loka -G loka

# Create directories for configuration and data
RUN mkdir -p /app/config /app/data /app/logs && \
    chown -R loka:loka /app

# Copy the binary from builder stage
COPY --from=builder /workspace/target/release/loka-stratum /usr/local/bin/loka-stratum

# Copy entrypoint script
COPY --chmod=755 stratum/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

# Copy default configuration
COPY --chown=loka:loka stratum/config/ /app/config/

# Switch to non-root user
USER loka

# Set working directory
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:9090/health || exit 1

# Expose ports
EXPOSE 3333 9090

# Use tini as entrypoint for proper signal handling
ENTRYPOINT ["/sbin/tini", "--"]

# Default command
CMD ["docker-entrypoint.sh"]

# Labels for metadata
LABEL maintainer="Geriano <geriano@nusantara.network>"
LABEL description="High-performance Bitcoin Stratum V1 mining proxy"
LABEL version="0.1.0"
LABEL org.opencontainers.image.source="https://github.com/geriano/loka"
LABEL org.opencontainers.image.description="Production-ready Bitcoin mining proxy with enterprise monitoring"
LABEL org.opencontainers.image.licenses="MIT"