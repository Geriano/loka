#!/bin/bash

# Sentry Self-Hosted Deployment Script for Loka Stratum Monitoring
# This script sets up Sentry using a simpler approach to avoid cache.backend issues

set -e

echo "🚀 Deploying Sentry for Loka Stratum monitoring..."

# Stop existing Sentry services if running
echo "🛑 Stopping existing Sentry services..."
docker-compose down sentry sentry-cron sentry-worker 2>/dev/null || true

# Start dependencies
echo "📦 Starting Sentry dependencies..."
docker-compose up -d sentry-postgres sentry-redis

# Wait for services to be ready
echo "⏳ Waiting for services to be ready..."
sleep 10

# Check if database is ready
echo "🔍 Checking PostgreSQL..."
docker-compose exec sentry-postgres pg_isready -U sentry || {
    echo "❌ PostgreSQL not ready"
    exit 1
}

echo "🔍 Checking Redis..."
docker-compose exec sentry-redis redis-cli ping || {
    echo "❌ Redis not ready"
    exit 1
}

# Create a minimal sentry.conf.py for initialization
echo "📝 Creating minimal Sentry configuration..."
cat > sentry/sentry-init.conf.py << 'EOF'
"""
Minimal Sentry configuration for initialization
"""
import os

# Database
DATABASES = {
    'default': {
        'ENGINE': 'sentry.db.postgres',
        'NAME': 'sentry',
        'USER': 'sentry',
        'PASSWORD': 'sentry123',
        'HOST': 'sentry-postgres',
        'PORT': '5432',
        'AUTOCOMMIT': True,
        'ATOMIC_REQUESTS': False,
    }
}

# Cache - use Django's default dummy cache for initialization
CACHES = {
    'default': {
        'BACKEND': 'django.core.cache.backends.dummy.DummyCache',
    }
}

# Secret key
SECRET_KEY = 'e)2b1w$!g(n2=8#!-*4^u=hh13h8(p(ypp!y$_6e^!@@96z2%c'

# Disable features that require full setup
SENTRY_BEACON = False
SENTRY_SINGLE_ORGANIZATION = True
SENTRY_USE_BIG_INTS = True

# Email backend
EMAIL_BACKEND = 'django.core.mail.backends.console.EmailBackend'

print("✅ Minimal Sentry configuration loaded for initialization")
EOF

# Run database migrations with minimal config
echo "🔄 Running database migrations..."
docker-compose run --rm -e SENTRY_CONF=/etc/sentry-init --volume "$(pwd)/sentry/sentry-init.conf.py:/etc/sentry-init/sentry.conf.py:ro" sentry sentry upgrade --noinput

# Create superuser
echo "👤 Creating superuser..."
docker-compose run --rm -e SENTRY_CONF=/etc/sentry-init --volume "$(pwd)/sentry/sentry-init.conf.py:/etc/sentry-init/sentry.conf.py:ro" sentry sentry createuser --email="admin@loka-stratum.local" --password="admin123" --superuser --no-input || echo "   Superuser may already exist"

echo "✅ Database initialization complete!"

# Now start Sentry with full configuration
echo "🚀 Starting Sentry services with full configuration..."
docker-compose up -d sentry sentry-cron sentry-worker

# Wait for Sentry to start
echo "⏳ Waiting for Sentry to start..."
timeout=60
while [ $timeout -gt 0 ]; do
    if curl -f -s http://localhost:9000/_health/ > /dev/null 2>&1; then
        echo "✅ Sentry is running!"
        break
    fi
    echo "   Waiting for Sentry to start... (${timeout}s remaining)"
    sleep 5
    timeout=$((timeout - 5))
done

if [ $timeout -le 0 ]; then
    echo "❌ Sentry failed to start within timeout"
    echo "📋 Checking logs..."
    docker-compose logs --tail=20 sentry
    exit 1
fi

echo "🎉 Sentry deployment completed successfully!"
echo ""
echo "🌐 Access Sentry at: http://localhost:9000"
echo "👤 Admin email: admin@loka-stratum.local"
echo "🔑 Admin password: admin123"
echo ""
echo "📋 Get your DSN by logging into Sentry and creating a project"
echo "   or run: docker-compose exec sentry sentry help"