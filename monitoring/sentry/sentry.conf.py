"""
Sentry configuration for Loka Stratum monitoring
"""
import os

# Get environment variables first
redis_host = os.environ.get('SENTRY_REDIS_HOST', 'sentry-redis')
redis_port = int(os.environ.get('SENTRY_REDIS_PORT', '6379'))

# CRITICAL: Set cache configuration BEFORE importing server config
# This must be done early to prevent the "You must configure cache.backend" error
os.environ.setdefault('SENTRY_CACHE_BACKEND', 'sentry.cache.redis.RedisCache')
os.environ.setdefault('SENTRY_CACHE_OPTIONS', f'{redis_host}:{redis_port}:1')

# Import the server configuration AFTER setting cache environment
from sentry.conf.server import *  # noqa

# Set cache configuration explicitly in SENTRY_OPTIONS as well
SENTRY_OPTIONS['cache.backend'] = 'sentry.cache.redis.RedisCache'
SENTRY_OPTIONS['cache.options'] = {
    'hosts': {
        0: {
            'host': redis_host,
            'port': redis_port,
            'db': 1,  # Use DB 1 for cache
        }
    }
}

# Database configuration
DATABASES = {
    'default': {
        'ENGINE': 'sentry.db.postgres',
        'NAME': os.environ.get('SENTRY_DB_NAME', 'sentry'),
        'USER': os.environ.get('SENTRY_DB_USER', 'sentry'),
        'PASSWORD': os.environ.get('SENTRY_DB_PASSWORD', 'sentry123'),
        'HOST': os.environ.get('SENTRY_POSTGRES_HOST', 'sentry-postgres'),
        'PORT': os.environ.get('SENTRY_POSTGRES_PORT', '5432'),
        'AUTOCOMMIT': True,
        'ATOMIC_REQUESTS': False,
    }
}

# You should not change this setting after your database has been created
# unless you have altered all schemas first
SENTRY_USE_BIG_INTS = True

# Redis configuration for queues and buffers
SENTRY_OPTIONS['redis.clusters'] = {
    'default': {
        'hosts': {
            0: {
                'host': redis_host,
                'port': redis_port,
                'db': 0,  # Use DB 0 for queues
            }
        }
    }
}

# Broker configuration for Celery
BROKER_URL = 'redis://{}:{}/0'.format(redis_host, redis_port)
CELERY_RESULT_BACKEND = BROKER_URL

# Buffer configuration
SENTRY_BUFFER = 'sentry.buffer.redis.RedisBuffer'
SENTRY_BUFFER_OPTIONS = {
    'cluster': 'default',
}

# Rate limiter
SENTRY_RATELIMITER = 'sentry.ratelimits.redis.RedisRateLimiter'
SENTRY_RATELIMITER_OPTIONS = {
    'cluster': 'default',
}

# Quotas
SENTRY_QUOTAS = 'sentry.quotas.redis.RedisQuota'
SENTRY_QUOTA_OPTIONS = {
    'cluster': 'default',
}

# TSDB
SENTRY_TSDB = 'sentry.tsdb.redis.RedisTSDB'
SENTRY_TSDB_OPTIONS = {
    'cluster': 'default',
}

# File storage
SENTRY_OPTIONS['filestore.backend'] = 'filesystem'
SENTRY_OPTIONS['filestore.options'] = {
    'location': '/var/lib/sentry/files',
}

# Web server configuration
SENTRY_WEB_HOST = '0.0.0.0'
SENTRY_WEB_PORT = 9000
SENTRY_WEB_OPTIONS = {
    'workers': 3,
    'buffer-size': 32768,
}

# Security
SECRET_KEY = os.environ.get('SENTRY_SECRET_KEY', 'change-me-please')
ALLOWED_HOSTS = ['*']
SECURE_PROXY_SSL_HEADER = ('HTTP_X_FORWARDED_PROTO', 'https')
SESSION_COOKIE_SECURE = False
CSRF_COOKIE_SECURE = False
SENTRY_OPTIONS['system.url-prefix'] = 'http://localhost:9000'

# Email backend (console for development)
EMAIL_BACKEND = 'django.core.mail.backends.console.EmailBackend'
EMAIL_HOST = 'localhost'
EMAIL_PORT = 25
EMAIL_USE_TLS = False
SERVER_EMAIL = os.environ.get('SENTRY_SERVER_EMAIL', 'admin@loka-stratum.local')
DEFAULT_FROM_EMAIL = SERVER_EMAIL

# Disable beacon
SENTRY_BEACON = False

# Features
SENTRY_FEATURES = {
    'organizations:crash-rate-alerts': True,
    'organizations:performance-view': True,
    'organizations:profiling-view': True,
}

# Logging
import logging
logging.getLogger('sentry.errors').setLevel(logging.INFO)

print("✅ Sentry configuration loaded from sentry.conf.py")