# Test Dockerfile to verify config copy works
FROM alpine:latest

# Create directories
RUN mkdir -p /app/config

# Test config copy
COPY stratum/config/ /app/config/

# List contents
RUN ls -la /app/config/