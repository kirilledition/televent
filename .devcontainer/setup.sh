#!/bin/bash
# Automated dev container setup
set -e

echo "🚀 Setting up Televent development environment..."

echo "🔧 Starting PostgreSQL..."
sudo service postgresql start

# Create database and user
echo "🗄️  Creating database..."
sudo -u postgres psql -tc "SELECT 1 FROM pg_database WHERE datname = 'televent'" | grep -q 1 || \
sudo -u postgres psql <<EOF
CREATE DATABASE televent;
CREATE USER televent WITH PASSWORD 'dev';
GRANT ALL PRIVILEGES ON DATABASE televent TO televent;
ALTER DATABASE televent OWNER TO televent;
EOF

# Run migrations
echo "🔄 Running database migrations..."
sqlx migrate run
