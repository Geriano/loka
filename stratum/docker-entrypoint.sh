#!/bin/sh

# Docker entrypoint script for Loka Stratum

set -e

# Default configuration file
CONFIG_FILE="${CONFIG_FILE:-/app/config/loka-stratum.toml}"

# Ensure config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "ERROR: Configuration file not found at $CONFIG_FILE"
    echo "Available files:"
    ls -la /app/config/
    exit 1
fi

echo "Starting Loka Stratum with config: $CONFIG_FILE"

# Start the stratum server
exec loka-stratum -c "$CONFIG_FILE" start "$@"
