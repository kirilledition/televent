# Default command shows all available commands
root := justfile_directory()

default:
    @just --list

# ==============================================
# Quick Start Commands
# ==============================================

# Initial setup for dev container (PostgreSQL + migrations)
setup-dev:
    @echo "Setting up development environment..."
    @echo "1. Starting Supabase..."
    cd {{root}}/infrastructure && npx -y supabase start
    @echo "2. Running migrations..."
    cd {{root}}/backend && sqlx migrate run
    @echo "3. Building project..."
    cd {{root}}/backend && cargo build --workspace
    @echo "✅ Setup complete! Run 'just run' to start the bot."

# Start Supabase services
db-start:
    @echo "Starting Supabase..."
    cd {{root}}/infrastructure && npx -y supabase start
    @echo "✅ Supabase is running"

# Check PostgreSQL status
# Check Supabase status
db-status:
    cd {{root}}/infrastructure && npx -y supabase status

# Stop PostgreSQL service
# Stop Supabase services
db-stop:
    cd {{root}}/infrastructure && npx -y supabase stop

# ==============================================
# Run Services
# ==============================================

# Run unified server (API, bot, and worker in one Railway-ready process)
run:
    cd {{root}}/backend && API_PORT=3001 cargo run --bin televent

# Run frontend dev server
run-frontend:
    cd {{root}}/frontend && pnpm dev

# Run fast backend tests that do not require DATABASE_URL
test:
    @echo "Running non-DB backend tests..."
    cd {{root}}/backend && cargo test -p televent-domain --lib
    cd {{root}}/backend && cargo test -p televent-application --lib
    cd {{root}}/backend && cargo test -p api --lib
    cd {{root}}/backend && cargo test -p bot --lib event_parser
    cd {{root}}/backend && cargo test -p worker --lib decode_claimed_jobs_marks_invalid_payload_failed
    @echo "Running doc tests..."
    cd {{root}}/backend && cargo test --workspace --doc

# Run the full backend test suite. Requires DATABASE_URL for sqlx::test cases.
test-db:
    @if [ -z "${DATABASE_URL:-}" ]; then echo "DATABASE_URL must be set for DB-backed tests"; exit 1; fi
    cd {{root}}/backend && cargo test --workspace

# Run tests with coverage report (HTML)
test-coverage:
    cd {{root}}/backend && cargo llvm-cov --workspace --html --output-dir ../logs/coverage/workspace

# Run coverage for a specific crate
test-crate-coverage crate:
    cd {{root}}/backend && cargo llvm-cov -p {{crate}} --html --output-dir ../logs/coverage/{{crate}}
    
lint:
    @echo "=== Backend === "
    @echo "Checking code..."
    cd {{root}}/backend && cargo check --workspace --tests
    @echo "Checking formatting..."
    cd {{root}}/backend && cargo fmt --all --check
    @echo "Running clippy..."
    cd {{root}}/backend && cargo clippy --workspace --all-targets -- -D warnings

# Lint frontend only
lint-frontend:
    cd {{root}}/frontend && pnpm lint

# Type-check frontend only
typecheck-frontend:
    cd {{root}}/frontend && pnpm typecheck

# Format frontend only
fmt-frontend:
    cd {{root}}/frontend && pnpm format

# Build the unified Railway Docker image
build-docker:
    docker build --pull --tag televent:ci {{root}}

# Reset database (drop, create, migrate)
# Reset database (Supabase db reset)
db-reset:
    @echo "Resetting database..."
    cd {{root}}/infrastructure && npx -y supabase db reset
    @echo "Applying SQLx migrations..."
    cd {{root}}/backend && sqlx migrate run
    @echo "✅ Database reset complete"
    
# Generate TypeScript types from API OpenAPI DTOs
gen-types:
    @echo "Generating OpenAPI JSON..."
    cd {{root}}/backend && cargo test -p api --lib tests::export_openapi_json -- --nocapture
    @echo "Generating TypeScript API types..."
    node {{root}}/frontend/scripts/generate-schema.mjs
    @echo "✅ Types generated to frontend/src/types/schema.ts"

# Generate OpenAPI JSON
gen-openapi:
    @echo "Generating OpenAPI JSON..."
    cd {{root}}/backend && cargo test -p api --lib tests::export_openapi_json -- --nocapture
    @echo "✅ OpenAPI JSON generated to docs/openapi.json"

upgrade:
    cd {{root}}/backend && cargo upgrade --incompatible --recursive
    cd {{root}}/backend && cargo machete --fix --no-ignore
    cd {{root}}/backend && cargo update
    
# cargo msrv find --write-msrv --min 1.88

# Upgrade frontend dependencies to bleeding edge
upgrade-frontend:
    # 1. Upgrade (Equivalent to `cargo upgrade --incompatible`)
    cd {{root}}/frontend && pnpm up -r --latest
    # 2. Machete (Equivalent to `cargo machete`)
    cd {{root}}/frontend && npx depcheck
    # 3. Update (Equivalent to `cargo update`)
    cd {{root}}/frontend && pnpm install
