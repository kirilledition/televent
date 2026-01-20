# Televent Development Commands
#
# This file contains ALL development commands for the project.
# No need for separate scripts - everything is here!
#
# Quick start:
#   just bot          - Run Telegram bot
#   just dev-bot      - Run bot with auto-reload
#   just test         - Run all tests
#   just fmt          - Format code
#   just lint         - Check code quality
#   just --list       - See all available commands
#
# The dev container automatically:
#   - Installs PostgreSQL
#   - Creates database and user
#   - Runs migrations
#   - Builds the project
#   - Creates .env file template
#
# You just need to:
#   1. Add your Telegram bot token to .env
#   2. Run: just bot

# Default command shows all available commands
default:
    @just --list

# ==============================================
# Quick Start Commands
# ==============================================

# Initial setup for dev container (PostgreSQL + migrations)
setup-dev:
    @echo "Setting up development environment..."
    @echo "1. Installing PostgreSQL..."
    sudo apt-get update -qq
    sudo apt-get install -y postgresql postgresql-contrib
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

# Run all services in parallel (requires tmux or separate terminals)
all:
    @echo "Run these commands in separate terminals:"
    @echo "  Terminal 1: just bot"
    @echo "  Terminal 2: just api"
    @echo "  Terminal 3: just worker"

# ==============================================
# Development with Auto-Reload
# ==============================================

# Run bot with auto-reload on code changes
dev-bot:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo watch -x 'run --bin bot'

# Run API with auto-reload on code changes
dev-api:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo watch -x 'run --bin api'

# Run worker with auto-reload on code changes
dev-worker:
    #!/usr/bin/env bash
    unset DATABASE_URL
    cargo watch -x 'run --bin worker'

# Run web UI with hot reload (Dioxus)
dev-web:
    cd crates/web && dx serve --hot-reload

# ==============================================
# Testing
# ==============================================

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run tests for specific crate
test-crate crate:
    cargo test -p {{crate}}

# Run specific test by name
test-name name:
    cargo test {{name}} --workspace -- --nocapture

# Run tests with coverage report
test-coverage:
    cargo tarpaulin --workspace --out Html --output-dir coverage

# Run CalDAV compliance tests (requires caldav-tester)
test-caldav:
    @echo "Running CalDAV compliance tests..."
    @echo "Note: Requires caldav-tester to be installed"
    # python testcaldav.py --server localhost:3000 --all

# ==============================================
# Code Quality
# ==============================================

# Format all code
fmt:
    cargo fmt --all

# Check formatting without changing files
fmt-check:
    cargo fmt --all --check

# Run clippy (linter)
clippy:
    cargo clippy --workspace -- -D warnings

# Run clippy and auto-fix issues
clippy-fix:
    cargo clippy --workspace --fix --allow-dirty

# Check for unused dependencies
check-deps:
    cargo +nightly udeps --workspace

# Lint everything (format check + clippy)
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

# Build all crates (release mode - optimized)
build-release:
    cargo build --workspace --release

# Build specific crate
build-crate crate:
    cargo build -p {{crate}}

# Check if code compiles without building
check:
    cargo check --workspace

# Clean build artifacts
clean:
    cargo clean

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

# Generate SQLx offline query metadata (for compile-time verification)
db-prepare:
    cargo sqlx prepare --workspace

# Check SQLx offline metadata is up to date
db-prepare-check:
    cargo sqlx prepare --workspace --check

# Open PostgreSQL console
db-console:
    PGPASSWORD=dev psql -h localhost -U televent -d televent

# Show database tables
db-tables:
    @PGPASSWORD=dev psql -h localhost -U televent -d televent -c "\dt"

# Show users in database
db-users:
    @PGPASSWORD=dev psql -h localhost -U televent -d televent -c "SELECT id, telegram_id, telegram_username, created_at FROM users;"

# Show events in database
db-events:
    @PGPASSWORD=dev psql -h localhost -U televent -d televent -c "SELECT id, summary, start, end FROM events ORDER BY start;"

# Show pending outbox messages
db-outbox:
    @PGPASSWORD=dev psql -h localhost -U televent -d televent -c "SELECT id, message_type, status, scheduled_at FROM outbox_messages WHERE status = 'pending' ORDER BY scheduled_at;"

# Backup database to file
db-backup:
    @mkdir -p backups
    @PGPASSWORD=dev pg_dump -h localhost -U televent televent | gzip > backups/televent_$(date +%Y%m%d_%H%M%S).sql.gz
    @echo "✅ Backup created in backups/"

# Restore database from backup file
db-restore file:
    @gunzip -c {{file}} | PGPASSWORD=dev psql -h localhost -U televent televent
    @echo "✅ Database restored from {{file}}"

# ==============================================
# Docker Operations (for production setup)
# ==============================================

# Start Docker services (Mailpit, Jaeger, PostgreSQL)
docker-up:
    docker-compose up -d

# Stop Docker services
docker-down:
    docker-compose down

# View Docker logs
docker-logs:
    docker-compose logs -f

# Stop and remove Docker volumes (clean slate)
docker-clean:
    docker-compose down -v

# ==============================================
# Development Tools
# ==============================================

# View Mailpit UI (email testing - http://localhost:8025)
mailpit:
    @echo "Opening Mailpit UI at http://localhost:8025"
    @xdg-open http://localhost:8025 || open http://localhost:8025 || echo "Please open http://localhost:8025 in your browser"

# View Jaeger UI (distributed tracing - http://localhost:16686)
jaeger:
    @echo "Opening Jaeger UI at http://localhost:16686"
    @xdg-open http://localhost:16686 || open http://localhost:16686 || echo "Please open http://localhost:16686 in your browser"

# Watch for file changes and run tests
watch-test:
    cargo watch -x test

# Watch for file changes and check compilation
watch-check:
    cargo watch -x check

# ==============================================
# Logs and Debugging
# ==============================================

# Run bot with debug logging
bot-debug:
    #!/usr/bin/env bash
    unset DATABASE_URL
    RUST_LOG=debug cargo run --bin bot

# Run API with debug logging
api-debug:
    #!/usr/bin/env bash
    unset DATABASE_URL
    RUST_LOG=debug cargo run --bin api

# Run worker with debug logging
worker-debug:
    #!/usr/bin/env bash
    unset DATABASE_URL
    RUST_LOG=debug cargo run --bin worker

# Run bot with trace logging (very verbose)
bot-trace:
    #!/usr/bin/env bash
    unset DATABASE_URL
    RUST_LOG=trace cargo run --bin bot

# ==============================================
# CI/CD and Quality Checks
# ==============================================

# Run full CI pipeline (format, lint, test, sqlx check)
ci:
    @echo "Running CI checks..."
    @echo "1. Format check..."
    cargo fmt --all --check
    @echo "2. Clippy..."
    cargo clippy --workspace -- -D warnings
    @echo "3. Tests..."
    cargo test --workspace
    @echo "4. SQLx metadata check..."
    cargo sqlx prepare --workspace --check
    @echo "✅ All CI checks passed!"

# Run CI pipeline and fix issues automatically
ci-fix:
    @echo "Running CI with auto-fix..."
    cargo fmt --all
    cargo clippy --workspace --fix --allow-dirty --allow-staged
    cargo test --workspace
    @echo "✅ CI complete with fixes applied!"

# Security audit (check for vulnerabilities in dependencies)
audit:
    cargo audit

# Update all dependencies to latest compatible versions
update:
    cargo update

# Update dependencies to latest versions (including breaking changes)
upgrade:
    @echo "Note: This requires 'cargo-edit' installed"
    @echo "Install with: cargo install cargo-edit"
    cargo upgrade

# ==============================================
# Documentation
# ==============================================

# Generate and open documentation
docs:
    cargo doc --workspace --no-deps --open

# Generate documentation without opening
docs-build:
    cargo doc --workspace --no-deps

# ==============================================
# Utilities
# ==============================================

# Show environment configuration
env:
    @echo "Environment Configuration:"
    @echo "=========================="
    @cat .env 2>/dev/null || echo "No .env file found"

# Show project structure
tree:
    @tree -L 3 -I 'target|node_modules|dist'

# Count lines of code
loc:
    @tokei

# Show Git status and recent commits
status:
    @git status
    @echo ""
    @echo "Recent commits:"
    @git log --oneline -5

# Initialize Git hooks (if you have pre-commit)
hooks:
    @pre-commit install || echo "pre-commit not installed"

# ==============================================
# Quick Commands Reference
# ==============================================

# Show common commands with descriptions
help:
    @echo "Televent - Common Commands"
    @echo "=========================="
    @echo ""
    @echo "🚀 Quick Start:"
    @echo "  just setup-dev    - First-time setup (install PostgreSQL, create DB, run migrations)"
    @echo "  just bot          - Run Telegram bot"
    @echo "  just api          - Run API server"
    @echo "  just worker       - Run background worker"
    @echo ""
    @echo "💻 Development:"
    @echo "  just dev-bot      - Run bot with auto-reload"
    @echo "  just dev-api      - Run API with auto-reload"
    @echo "  just test         - Run all tests"
    @echo "  just fmt          - Format all code"
    @echo "  just lint         - Check formatting and run clippy"
    @echo "  just fix          - Auto-fix formatting and clippy issues"
    @echo ""
    @echo "🗄️  Database:"
    @echo "  just db-start     - Start PostgreSQL"
    @echo "  just db-console   - Open database console"
    @echo "  just db-migrate   - Run migrations"
    @echo "  just db-reset     - Reset database (drop + create + migrate)"
    @echo "  just db-tables    - Show all tables"
    @echo "  just db-users     - Show users"
    @echo "  just db-events    - Show events"
    @echo ""
    @echo "🔧 Utilities:"
    @echo "  just build        - Build all crates"
    @echo "  just clean        - Clean build artifacts"
    @echo "  just ci           - Run CI checks (format, lint, test)"
    @echo "  just audit        - Security audit"
    @echo ""
    @echo "📖 Full list: just --list"
