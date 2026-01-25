# Default command shows all available commands
default:
    @just --list

# ==============================================
# Quick Start Commands
# ==============================================

# Initial setup for dev container (PostgreSQL + migrations)
setup-dev:
    @echo "Setting up development environment..."
    @echo "1. Starting Supabase..."
    npx -y supabase start
    @echo "2. Running migrations..."
    sqlx migrate run
    @echo "3. Building project..."
    cargo build --workspace
    @echo "✅ Setup complete! Run 'just run' to start the bot."

# Start Supabase services
db-start:
    @echo "Starting Supabase..."
    npx -y supabase start
    @echo "✅ Supabase is running"

# Check PostgreSQL status
# Check Supabase status
db-status:
    npx -y supabase status

# Stop PostgreSQL service
# Stop Supabase services
db-stop:
    npx -y supabase stop

# ==============================================
# Run Services
# ==============================================

# Run unified server (all services in one process) - DEFAULT for MVP
run:
    cargo run --bin televent

# Run all tests
test:
    @echo "Running normal tests (libs, bins)..."
    cargo test --workspace --lib --bins
    @echo "Running doc tests..."
    cargo test --workspace --doc
    @echo "Running integration tests..."
    cargo test --workspace --tests

# Run tests with coverage report (HTML)
test-coverage:
    cargo llvm-cov --workspace --html --output-dir logs/coverage/workspace

# Run coverage for a specific crate
test-crate-coverage crate:
    cargo llvm-cov -p {{crate}} --html --output-dir logs/coverage/{{crate}}
    
lint:
    @echo "=== Backend === "
    @echo "Checking code..."
    cargo check --workspace
    @echo "Checking formatting..."
    cargo fmt --all
    @echo "Running clippy..."
    cargo clippy --allow-dirty --allow-staged --fix --workspace -- -D warnings

# Lint frontend only
lint-frontend:
    cd frontend && pnpm lint

# Format frontend only
fmt-frontend:
    cd frontend && pnpm format

# Reset database (drop, create, migrate)
# Reset database (Supabase db reset)
db-reset:
    @echo "Resetting database..."
    npx -y supabase db reset
    @echo "Applying SQLx migrations..."
    sqlx migrate run
    @echo "✅ Database reset complete"
    
# Generate TypeScript types from Rust models
gen-types:
    @echo "Generating TypeScript types..."
    typeshare . --lang=typescript --output-file=frontend/src/types/schema.ts
    @echo "✅ Types generated to frontend/src/types/schema.ts"

upgrade:
    cargo upgrade --incompatible --recursive
    cargo machete --fix --no-ignore
    cargo update
    
# cargo msrv find --write-msrv --min 1.85

# Upgrade frontend dependencies to bleeding edge
upgrade-frontend:
    # 1. Upgrade (Equivalent to `cargo upgrade --incompatible`)
    pnpm up -r --latest
    # 2. Machete (Equivalent to `cargo machete`)
    npx depcheck
    # 3. Update (Equivalent to `cargo update`)
    pnpm install
