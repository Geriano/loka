#!/bin/sh

# Docker entrypoint script for Loka Stratum

set -e

# Start the stratum server
exec loka-stratum start "$@"
