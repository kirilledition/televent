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
    @echo "Checking code..."
    cargo check --workspace
    @echo "Checking formatting..."
    cargo fmt --all
    @echo "Running clippy..."
    cargo clippy --allow-dirty --allow-staged --fix --workspace -- -D warnings
    @echo "✅ All checks passed!"

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

# Generate Markdown API documentation from OpenAPI spec
gen-api-docs:
    @echo "Generating OpenAPI JSON..."
    cargo test -p api --lib tests::export_openapi_json -- --nocapture
    @echo "Converting OpenAPI JSON to Markdown..."
    # We use npx to run openapi-markdown-notes or similar if available, 
    # but openapi-generator-cli is more robust.
    # Note: openapi-generator-cli requires java.
    # If java is not available, we might need a different tool.
    # Let's try openapi-markdown (npm package) which is JS based.
    npx -y openapi-markdown -i openapi.json -o API.md
    @echo "Cleaning up..."
    rm openapi.json
    @echo "✅ API documentation generated to API.md"
