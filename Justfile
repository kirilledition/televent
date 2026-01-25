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

# Run all tests
test:
    cargo test --workspace

# Run tests with coverage report (HTML)
test-coverage:
    cargo llvm-cov --workspace --html --output-dir logs

lint:
    @echo "Checking code..."
    cargo check --workspace
    @echo "Checking formatting..."
    cargo fmt --all
    @echo "Running clippy..."
    cargo clippy --allow-dirty --allow-staged --fix --workspace -- -D warnings
    @echo "✅ All checks passed!"

# Reset database (drop, create, migrate)
db-reset:
    @echo "Resetting database..."
    docker-compose down -v db
    docker-compose up -d db
    @echo "Waiting for PostgreSQL to be ready..."
    @until docker-compose exec db pg_isready -U televent; do sleep 1; done
    sqlx migrate run
    @echo "✅ Database reset complete"
