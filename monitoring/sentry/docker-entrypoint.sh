#!/bin/bash
set -e

# Create Sentry configuration file
cat > /etc/sentry/sentry.conf.py << 'EOF'
"""
Auto-generated Sentry configuration for Loka Stratum
"""
import os
from sentry.conf.server import *  # noqa

# Database
DATABASES = {
    'default': {
        'ENGINE': 'sentry.db.postgres',
        'NAME': os.environ.get('SENTRY_DB_NAME', 'sentry'),
        'USER': os.environ.get('SENTRY_DB_USER', 'sentry'),
        'PASSWORD': os.environ.get('SENTRY_DB_PASSWORD', 'sentry123'),
        'HOST': os.environ.get('SENTRY_POSTGRES_HOST', 'sentry-postgres'),
        'PORT': os.environ.get('SENTRY_POSTGRES_PORT', '5432'),
    }
}

# Cache configuration - FIXES THE cache.backend ERROR
CACHES = {
    'default': {
        'BACKEND': 'django_redis.cache.RedisCache',
        'LOCATION': 'redis://{}:{}/1'.format(
            os.environ.get('SENTRY_REDIS_HOST', 'sentry-redis'),
            os.environ.get('SENTRY_REDIS_PORT', '6379')
        ),
    }
}

# General Redis configuration
redis_host = os.environ.get('SENTRY_REDIS_HOST', 'sentry-redis')
redis_port = os.environ.get('SENTRY_REDIS_PORT', '6379')

SENTRY_OPTIONS['redis.clusters'] = {
    'default': {
        'hosts': {
            0: {
                'host': redis_host,
                'port': int(redis_port),
                'db': 0,
            }
        }
    }
}

# Celery broker
BROKER_URL = 'redis://{}:{}/0'.format(redis_host, redis_port)
CELERY_RESULT_BACKEND = BROKER_URL

# File storage
SENTRY_OPTIONS['filestore.backend'] = 'filesystem'
SENTRY_OPTIONS['filestore.options'] = {
    'location': '/var/lib/sentry/files',
}

# Web server
SENTRY_WEB_HOST = '0.0.0.0'
SENTRY_WEB_PORT = 9000
FORCE_SCRIPT_NAME = ''
SECURE_PROXY_SSL_HEADER = ('HTTP_X_FORWARDED_PROTO', 'https')
SESSION_COOKIE_SECURE = False
CSRF_COOKIE_SECURE = False
SENTRY_OPTIONS['system.url-prefix'] = 'http://localhost:9000'

# Security
SECRET_KEY = os.environ.get('SENTRY_SECRET_KEY', 'default-insecure-key-change-me')

# Email
EMAIL_BACKEND = 'django.core.mail.backends.console.EmailBackend'
EMAIL_HOST = 'localhost'
EMAIL_PORT = 25
SERVER_EMAIL = 'admin@loka-stratum.local'

# Disable telemetry
SENTRY_BEACON = False

print("✅ Sentry configuration generated successfully")
EOF

echo "Configuration file created at /etc/sentry/sentry.conf.py"

# Execute Sentry with the original command
exec /docker-entrypoint.sh "$@"