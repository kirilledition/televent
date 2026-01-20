#!/bin/bash
# Automated dev container setup
set -e

echo "🚀 Setting up Televent development environment..."

# Install and start PostgreSQL
echo "📦 Installing PostgreSQL..."
sudo apt-get update -qq
sudo apt-get install -y -qq postgresql postgresql-contrib

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

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file..."
    cat > .env << 'ENVFILE'
DATABASE_URL=postgresql://televent:dev@localhost:5432/televent
TELEGRAM_BOT_TOKEN=your_bot_token_here
TELEGRAM_BOT_USERNAME=your_bot_username
API_HOST=0.0.0.0
API_PORT=3000
API_BASE_URL=http://localhost:3000
JWT_SECRET=dev_secret_change_for_production
JWT_EXPIRY_HOURS=24
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_FROM_EMAIL=noreply@televent.app
SMTP_FROM_NAME=Televent
RUST_LOG=info,televent=debug,sqlx=warn
JAEGER_ENDPOINT=http://localhost:4317
SENTRY_DSN=
ENVIRONMENT=development
ENVFILE
    chmod 600 .env
    echo "⚠️  Don't forget to add your Telegram bot token to .env!"
fi

# Run migrations
echo "🔄 Running database migrations..."
sqlx migrate run

# Build project
echo "🔨 Building project..."
cargo build --workspace

echo "✅ Setup complete!"
echo ""
echo "Quick start:"
echo "  1. Add your Telegram bot token to .env"
echo "  2. Run: just bot"
echo "  3. Message your bot on Telegram!"
