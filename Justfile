# Default command shows all available commands
default:
    @just --list

# ==============================================
# Quick Start Commands
# ==============================================

# Initial setup for dev container (PostgreSQL + migrations)
setup-dev:
    @echo "Setting up development environment..."
    @echo "1. Starting PostgreSQL via Docker..."
    docker-compose up -d db
    @echo "2. Waiting for PostgreSQL to be ready..."
    @until docker-compose exec db pg_isready -U televent; do sleep 1; done
    @echo "3. Running migrations..."
    sqlx migrate run
    @echo "4. Building project..."
    cargo build --workspace
    @echo "✅ Setup complete! Run 'just bot' to start the bot."

# Start PostgreSQL service via Docker
db-start:
    @echo "Starting PostgreSQL..."
    docker-compose up -d db
    @echo "✅ PostgreSQL is running"

# Check PostgreSQL status
db-status:
    @docker-compose ps db

# Stop PostgreSQL service
db-stop:
    @docker-compose stop db

# ==============================================
# Run Services
# ==============================================

# Run unified server (all services in one process) - DEFAULT for MVP
run:
    cargo run --bin televent

# Alias for clarity
server: run

# Build release binary for production
build-release:
    cargo build --release --bin televent

# Run Telegram bot (standalone mode)
bot:
    cargo run --bin bot

# Run API server (standalone mode)
api:
    cargo run --bin api

# Run background worker (standalone mode)
worker:
    cargo run --bin worker

# Run all services separately in parallel (legacy mode)
run-separate:
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
    docker-compose down -v db
    docker-compose up -d db
    @echo "Waiting for PostgreSQL to be ready..."
    @until docker-compose exec db pg_isready -U televent; do sleep 1; done
    sqlx migrate run
    @echo "✅ Database reset complete"
