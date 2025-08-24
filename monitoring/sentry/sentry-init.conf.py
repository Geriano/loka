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
