#!/bin/bash

# Sentry initialization script for Loka Stratum monitoring
# This script sets up the initial Sentry configuration, creates a superuser, and project

set -e

echo "🚀 Initializing Sentry for Loka Stratum monitoring..."

# Wait for database to be ready
echo "⏳ Waiting for PostgreSQL to be ready..."
until PGPASSWORD=sentry123 psql -h sentry-postgres -U sentry -d sentry -c '\q' 2>/dev/null; do
  echo "   PostgreSQL is unavailable - sleeping"
  sleep 2
done
echo "✅ PostgreSQL is ready!"

# Wait for Redis to be ready
echo "⏳ Waiting for Redis to be ready..."
until redis-cli -h sentry-redis ping 2>/dev/null; do
  echo "   Redis is unavailable - sleeping"
  sleep 2
done
echo "✅ Redis is ready!"

# Initialize database (run migrations)
echo "📦 Running database migrations..."
sentry upgrade --noinput

# Create superuser if it doesn't exist
echo "👤 Creating superuser..."
sentry createuser \
  --email="${SENTRY_ADMIN_EMAIL:-admin@loka-stratum.local}" \
  --password="${SENTRY_ADMIN_PASSWORD:-admin123}" \
  --superuser \
  --no-input || echo "   Superuser already exists, skipping..."

# Create organization and project for Loka Stratum
echo "🏢 Setting up Loka Stratum organization and project..."

# Use Sentry shell to create organization and project
python3 << 'EOF'
import os
import django
from django.conf import settings

# Configure Django
os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'sentry.conf.server')
django.setup()

from sentry.models import Organization, Project, User, Team, OrganizationMember
from sentry.models.organizationmember import InviteStatus

# Get or create organization
try:
    org = Organization.objects.get(slug='loka-stratum')
    print(f"   ✅ Organization 'loka-stratum' already exists")
except Organization.DoesNotExist:
    org = Organization.objects.create(
        name='Loka Stratum Mining Proxy',
        slug='loka-stratum'
    )
    print(f"   ✅ Created organization: {org.name}")

# Get superuser
admin_email = os.environ.get('SENTRY_ADMIN_EMAIL', 'admin@loka-stratum.local')
try:
    user = User.objects.get(email=admin_email)
    print(f"   ✅ Found admin user: {user.email}")
    
    # Add user to organization if not already a member
    try:
        OrganizationMember.objects.get(organization=org, user=user)
        print(f"   ✅ Admin user is already a member of organization")
    except OrganizationMember.DoesNotExist:
        OrganizationMember.objects.create(
            organization=org,
            user=user,
            role='owner',
            has_global_access=True,
            invite_status=InviteStatus.APPROVED.value
        )
        print(f"   ✅ Added admin user to organization")
        
except User.DoesNotExist:
    print(f"   ❌ Admin user not found: {admin_email}")

# Get or create team
try:
    team = Team.objects.get(organization=org, slug='loka-stratum-team')
    print(f"   ✅ Team 'loka-stratum-team' already exists")
except Team.DoesNotExist:
    team = Team.objects.create(
        organization=org,
        name='Loka Stratum Team',
        slug='loka-stratum-team'
    )
    print(f"   ✅ Created team: {team.name}")

# Get or create main project
try:
    project = Project.objects.get(organization=org, slug='loka-stratum-proxy')
    print(f"   ✅ Project 'loka-stratum-proxy' already exists")
    print(f"   📋 Project DSN: {project.get_keys()[0].dsn.public if project.get_keys() else 'No keys found'}")
except Project.DoesNotExist:
    project = Project.objects.create(
        organization=org,
        team=team,
        name='Loka Stratum Proxy',
        slug='loka-stratum-proxy',
        platform='rust'
    )
    print(f"   ✅ Created project: {project.name}")
    
    # Get the DSN
    project_key = project.get_keys()[0] if project.get_keys() else None
    if project_key:
        print(f"   📋 Project DSN: {project_key.dsn.public}")
        
        # Save DSN to file for easy access
        with open('/var/lib/sentry/files/loka-stratum-dsn.txt', 'w') as f:
            f.write(project_key.dsn.public)
        print(f"   💾 DSN saved to /var/lib/sentry/files/loka-stratum-dsn.txt")
    else:
        print(f"   ⚠️  No project key found for DSN generation")

print("🎉 Sentry initialization completed successfully!")
EOF

echo "🎯 Sentry setup complete!"
echo ""
echo "🌐 Access Sentry at: http://localhost:9000"
echo "👤 Admin email: ${SENTRY_ADMIN_EMAIL:-admin@loka-stratum.local}"
echo "🔑 Admin password: ${SENTRY_ADMIN_PASSWORD:-admin123}"
echo ""
echo "📋 To get your DSN for Loka Stratum integration:"
echo "   docker exec -it $(docker ps --filter name=monitoring-sentry-1 --format '{{.ID}}') cat /var/lib/sentry/files/loka-stratum-dsn.txt"
echo ""