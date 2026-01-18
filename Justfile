# Justfile for Televent development

# Default command shows help
default:
    @just --list

# Initial setup - start services and prepare database
setup:
    docker-compose up -d
    @echo "Waiting for PostgreSQL to be ready..."
    @sleep 5
    cargo sqlx database create || true
    cargo sqlx migrate run
    cargo build --workspace

# Run all tests
test:
    cargo test --workspace

# Run tests with coverage
test-coverage:
    cargo tarpaulin --workspace --out Html --output-dir coverage

# Lint and format checks
lint:
    cargo fmt --check
    cargo clippy --workspace -- -D warnings

# Auto-fix formatting
fmt:
    cargo fmt

# Database operations
db-reset:
    cargo sqlx database drop -y || true
    cargo sqlx database create
    cargo sqlx migrate run

db-migrate:
    cargo sqlx migrate run

db-rollback:
    cargo sqlx migrate revert

db-create-migration name:
    cargo sqlx migrate add {{name}}

db-prepare:
    cargo sqlx prepare

# Development servers
dev-api:
    cargo watch -x 'run -p api'

dev-bot:
    cargo watch -x 'run -p bot'

dev-worker:
    cargo watch -x 'run -p worker'

dev-web:
    cd crates/web && dx serve --hot-reload

# Build all binaries
build:
    cargo build --workspace

build-release:
    cargo build --workspace --release

# Docker operations
docker-up:
    docker-compose up -d

docker-down:
    docker-compose down

docker-logs:
    docker-compose logs -f

docker-clean:
    docker-compose down -v

# Database console
db-console:
    psql postgresql://televent:dev@localhost:5432/televent

# View Mailpit (email testing UI)
mailpit:
    @echo "Opening Mailpit UI at http://localhost:8025"
    @xdg-open http://localhost:8025 || open http://localhost:8025 || echo "Please open http://localhost:8025 in your browser"

# View Jaeger (tracing UI)
jaeger:
    @echo "Opening Jaeger UI at http://localhost:16686"
    @xdg-open http://localhost:16686 || open http://localhost:16686 || echo "Please open http://localhost:16686 in your browser"

# Clean build artifacts
clean:
    cargo clean

# Run CalDAV compliance tests (requires caldav-tester)
test-caldav:
    @echo "Running CalDAV compliance tests..."
    @echo "Note: Requires caldav-tester to be installed"
    # python testcaldav.py --server localhost:3000 --all

# Full CI check (format, lint, test)
ci:
    cargo fmt --check
    cargo clippy --workspace -- -D warnings
    cargo test --workspace
    cargo sqlx prepare --check

# Security audit
audit:
    cargo audit

# Code quality check (extended)
quality:
    @echo "Running comprehensive code quality checks..."
    cargo fmt --check
    cargo clippy --workspace -- -D warnings -W clippy::pedantic
    cargo test --workspace
    @echo "Checking for TODO/FIXME comments..."
    @rg "TODO|FIXME" --type rust || echo "No TODOs found"
    @echo "Quality checks complete!"

# Check for common anti-patterns
check-antipatterns:
    @echo "Checking for unwrap() usage..."
    @rg "\.unwrap\(\)" crates/ --type rust || echo "No unwrap() found ✓"
    @echo "Checking for expect() usage..."
    @rg "\.expect\(" crates/ --type rust || echo "No expect() found ✓"
    @echo "Checking for println! usage..."
    @rg "println!" crates/ --type rust || echo "No println! found ✓"
    @echo "Checking for panic! usage..."
    @rg "panic!" crates/ --type rust || echo "No panic! found ✓"
    @echo "Anti-pattern checks complete!"

