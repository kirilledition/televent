# Default command shows all available commands
default:
    @just --list

# ==============================================
# Quick Start Commands
# ==============================================

# Initial setup for dev container (PostgreSQL + migrations)
setup-dev:
    @echo "Setting up development environment..."
    @echo "2. Starting PostgreSQL..."
    sudo service postgresql start
    @echo "3. Creating database and user..."
    sudo -u postgres psql -tc "SELECT 1 FROM pg_database WHERE datname = 'televent'" | grep -q 1 || \
    sudo -u postgres psql -c "CREATE DATABASE televent; CREATE USER televent WITH PASSWORD 'dev'; GRANT ALL PRIVILEGES ON DATABASE televent TO televent;"
    @echo "4. Running migrations..."
    sqlx migrate run
    @echo "5. Building project..."
    cargo build --workspace
    @echo "✅ Setup complete! Run 'just bot' to start the bot."

# Start PostgreSQL service (for dev container)
db-start:
    @echo "Starting PostgreSQL..."
    sudo service postgresql start
    @echo "✅ PostgreSQL is running"

# Check PostgreSQL status
db-status:
    @sudo service postgresql status

# Stop PostgreSQL service
db-stop:
    @sudo service postgresql stop

# ==============================================
# Run Services
# ==============================================

# Run Telegram bot
bot:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo run --bin bot

# Run API server
api:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo run --bin api

# Run background worker
worker:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo run --bin worker

# Run all services in parallel (Ctrl+C kills all)
run:
    #!/usr/bin/env bash
    # 1. Define a cleanup function to kill background jobs when this script exits
    cleanup() {
        echo "🛑 Shutting down all services..."
        # 'kill 0' sends the signal to every process in the current process group
        kill 0
    }
    
    # 2. Trap specific signals (SIGINT = Ctrl+C, SIGTERM) to run cleanup
    trap cleanup SIGINT SIGTERM EXIT

    echo "🚀 Starting Televent Services..."
    
    # 3. Start binaries in the background (&)
    # We use parenthesis to group output if needed, but here we just launch them.
    (unset DATABASE_URL && cargo run --bin bot) &
    (unset DATABASE_URL && cargo run --bin api) &
    (unset DATABASE_URL && cargo run --bin worker) &

    # 4. Wait for all background processes to finish (blocks until you hit Ctrl+C)
    wait

# Force kill any lingering Televent binaries (useful if something gets stuck)
kill-all:
    @echo "🔫 Killing all Televent processes..."
    @pkill -f "target/debug/bot" || echo "Bot was not running"
    @pkill -f "target/debug/api" || echo "API was not running"
    @pkill -f "target/debug/worker" || echo "Worker was not running"
    @echo "✅ Cleanup complete."
# ==============================================
# Testing
# ==============================================

# Run all tests
test:
    cargo test --workspace

# Run tests with coverage report
test-coverage:
    cargo tarpaulin --workspace --out Html --output-dir coverage

lint:
    @echo "Checking formatting..."
    cargo fmt --all --check
    @echo "Running clippy..."
    cargo clippy --workspace -- -D warnings
    @echo "✅ All checks passed!"

# Fix all auto-fixable issues (format + clippy)
fix:
    @echo "Formatting code..."
    cargo fmt --all
    @echo "Fixing clippy issues..."
    cargo clippy --workspace --fix --allow-dirty --allow-staged
    @echo "✅ All fixes applied!"

# ==============================================
# Building
# ==============================================

# Build all crates (debug mode)
build:
    cargo build --workspace

# Check if code compiles without building
check:
    cargo check --workspace

# ==============================================
# Database Operations
# ==============================================

# Run database migrations
db-migrate:
    sqlx migrate run

# Rollback last migration
db-rollback:
    sqlx migrate revert

# Create a new migration file
db-new-migration name:
    sqlx migrate add {{name}}

# Reset database (drop, create, migrate)
db-reset:
    @echo "Resetting database..."
    sudo -u postgres psql <<EOF
    DROP DATABASE IF EXISTS televent;
    CREATE DATABASE televent;
    GRANT ALL PRIVILEGES ON DATABASE televent TO televent;
    EOF
    sqlx migrate run
    @echo "✅ Database reset complete"
