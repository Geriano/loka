# Migration-specific Dockerfile
FROM rust:1.88-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

# Create app directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy migration and model sources
COPY migration/ ./migration/
COPY model/ ./model/

# Build migration binary
RUN cargo build --release -p migration

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata postgresql-client && \
    addgroup -g 1001 -S migration && \
    adduser -u 1001 -S migration -G migration

# Copy migration binary
COPY --from=builder /app/target/release/migration /usr/local/bin/migration

# Switch to non-root user
USER migration

# Default command
CMD ["migration"]

# Labels
LABEL maintainer="Geriano <geriano@nusantara.network>"
LABEL description="Database migration tool for Loka Stratum"
LABEL version="0.1.0"