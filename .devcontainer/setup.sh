#!/bin/bash
# Automated dev container setup
set -e

echo "🚀 Setting up Televent development environment..."

echo "🔧 Starting Supabase..."
npx -y supabase start

# Run migrations
echo "🔄 Running database migrations..."
sqlx migrate run
