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
    cd infrastructure && npx -y supabase start
    @echo "2. Running migrations..."
    cd backend && sqlx migrate run
    @echo "3. Building project..."
    cd backend && cargo build --workspace
    @echo "✅ Setup complete! Run 'just run' to start the bot."

# Start Supabase services
db-start:
    @echo "Starting Supabase..."
    cd infrastructure && npx -y supabase start
    @echo "✅ Supabase is running"

# Check PostgreSQL status
# Check Supabase status
db-status:
    cd infrastructure && npx -y supabase status

# Stop PostgreSQL service
# Stop Supabase services
db-stop:
    cd infrastructure && npx -y supabase stop

# ==============================================
# Run Services
# ==============================================

# Run unified server (all services in one process) - DEFAULT for MVP
run:
    cd backend && API_PORT=3001 cargo run --bin televent

# Run frontend dev server
run-frontend:
    cd frontend && pnpm dev

# Run all tests
test:
    @echo "Running normal tests (libs, bins)..."
    cd backend && cargo test --workspace --lib --bins
    @echo "Running doc tests..."
    cd backend && cargo test --workspace --doc
    @echo "Running integration tests..."
    cd backend && cargo test --workspace --tests

# Run tests with coverage report (HTML)
test-coverage:
    cd backend && cargo llvm-cov --workspace --html --output-dir ../logs/coverage/workspace

# Run coverage for a specific crate
test-crate-coverage crate:
    cd backend && cargo llvm-cov -p {{crate}} --html --output-dir ../logs/coverage/{{crate}}
    
lint:
    @echo "=== Backend === "
    @echo "Checking code..."
    cd backend && cargo check --workspace
    @echo "Checking formatting..."
    cd backend && cargo fmt --all
    @echo "Running clippy..."
    cd backend && cargo clippy --allow-dirty --allow-staged --fix --workspace -- -D warnings

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
    cd infrastructure && npx -y supabase db reset
    @echo "Applying SQLx migrations..."
    cd backend && sqlx migrate run
    @echo "✅ Database reset complete"
    
# Generate TypeScript types from Rust models
gen-types:
    @echo "Generating TypeScript types..."
    cd backend && typeshare . --lang=typescript --output-file=../frontend/src/types/schema.ts
    @echo "✅ Types generated to frontend/src/types/schema.ts"

# Generate OpenAPI JSON
gen-openapi:
    @echo "Generating OpenAPI JSON..."
    cd backend && cargo test -p api --lib tests::export_openapi_json -- --nocapture
    @echo "✅ OpenAPI JSON generated to docs/openapi.json"

upgrade:
    cd backend && cargo upgrade --incompatible --recursive
    cd backend && cargo machete --fix --no-ignore
    cd backend && cargo update
    
# cargo msrv find --write-msrv --min 1.88

# Upgrade frontend dependencies to bleeding edge
upgrade-frontend:
    # 1. Upgrade (Equivalent to `cargo upgrade --incompatible`)
    pnpm up -r --latest
    # 2. Machete (Equivalent to `cargo machete`)
    npx depcheck
    # 3. Update (Equivalent to `cargo update`)
    pnpm install
