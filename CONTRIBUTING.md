# Contributing to Televent

Thank you for your interest in contributing to Televent! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Quality Standards](#code-quality-standards)
- [Testing Guidelines](#testing-guidelines)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and professional in all interactions.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Docker and Docker Compose
- PostgreSQL 16 (via Docker)
- Just (command runner)

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/kirilledition/televent.git
cd televent

# Set up the development environment
just setup

# Run tests to verify setup
just test
```

## Development Workflow

### Project Structure

```
televent/
├── crates/
│   ├── core/      # Domain logic (pure Rust, no I/O)
│   ├── api/       # Axum server (CalDAV + REST)
│   ├── bot/       # Telegram bot
│   ├── worker/    # Outbox consumer
│   ├── mailer/    # Email sender
│   └── web/       # Dioxus frontend
├── migrations/    # SQLx database migrations
└── docs/          # Documentation
```

### Common Commands

```bash
# Development
just dev-api        # Run API server with hot reload
just dev-bot        # Run Telegram bot with hot reload
just dev-worker     # Run worker with hot reload
just dev-web        # Run web UI with hot reload

# Testing
just test           # Run all tests
just test-coverage  # Run tests with coverage report

# Code Quality
just fmt            # Format code
just lint           # Run linter
just quality        # Comprehensive quality checks
just check-antipatterns  # Check for unwrap/expect/println

# Database
just db-reset       # Drop and recreate database
just db-migrate     # Apply migrations
just db-create-migration <name>  # Create new migration
```

## Code Quality Standards

### Critical Rules (Zero Tolerance)

These rules are enforced by Clippy and CI:

1. **NO `unwrap()` or `expect()`**: Use `?` operator or explicit error handling
   - Rationale: We handle calendars - data loss from panics is unacceptable
   - Example: Use `value.ok_or(Error::NotFound)?` instead of `value.unwrap()`

2. **NO `println!` or `print!`**: Use `tracing::info!` or `tracing::error!`
   - Rationale: We need structured logs for debugging production CalDAV sync issues
   - Example: Use `tracing::info!(user_id = %id, "User logged in")` instead of `println!("User logged in")`

3. **Use newtypes for IDs**: Always use `UserId`, `CalendarId`, `EventId` instead of raw `Uuid`
   - Rationale: CalDAV spec has many ID types - type confusion causes sync corruption
   - Example: Use `user_id: UserId` instead of `user_id: Uuid`

4. **Always use Tokio runtime**: No other async runtimes
   - Rationale: Entire stack (Axum, SQLx, Teloxide) is tokio-based

### Code Style

- **Format**: Run `cargo fmt` before committing (enforced by CI)
- **Linting**: Run `cargo clippy -- -D warnings` (enforced by CI)
- **Line length**: Maximum 100 characters (configured in .editorconfig)
- **Imports**: Group std, external crates, internal crates (rustfmt handles this)

### Error Handling

- **Libraries (core, mailer)**: Use `thiserror` for typed errors
  ```rust
  #[derive(Error, Debug)]
  pub enum CalendarError {
      #[error("Event not found: {0}")]
      EventNotFound(EventId),
  }
  ```

- **Binaries (api, bot, worker)**: Use `anyhow` for quick error propagation
  ```rust
  async fn handle_request() -> anyhow::Result<Response> {
      let user = get_user(&id).await.context("Failed to get user")?;
      Ok(Response::new(user))
  }
  ```

### Documentation

- **Public APIs**: Must have doc comments
  ```rust
  /// Creates a new user account
  ///
  /// # Arguments
  /// * `telegram_id` - The user's Telegram ID
  ///
  /// # Errors
  /// Returns an error if the user already exists
  pub async fn create_user(telegram_id: i64) -> Result<User> {
      // ...
  }
  ```

- **Complex logic**: Add inline comments explaining "why", not "what"
- **Security-critical code**: Always document assumptions and constraints

## Testing Guidelines

### Test Coverage Targets

- **Core crate**: 80% coverage (enforced)
- **API handlers**: 60% coverage
- **Overall**: 70% coverage minimum

### Test Types

1. **Unit Tests**: Test individual functions in isolation
   ```rust
   #[test]
   fn test_user_id_creation() {
       let id1 = UserId::new();
       let id2 = UserId::new();
       assert_ne!(id1, id2);
   }
   ```

2. **Integration Tests**: Test API endpoints and database operations
   ```rust
   #[tokio::test]
   async fn test_create_event_endpoint() {
       let app = create_test_app().await;
       let response = app.post("/api/events")
           .json(&event_data)
           .send()
           .await;
       assert_eq!(response.status(), 201);
   }
   ```

3. **CalDAV Compliance Tests**: Run before merging CalDAV changes
   ```bash
   just test-caldav
   ```

### Writing Good Tests

- **Arrange-Act-Assert**: Structure tests clearly
- **One assertion per test**: Keep tests focused
- **Descriptive names**: `test_create_event_returns_201` not `test1`
- **Clean up**: Use test fixtures and tear down properly

## Database Migrations

### Creating Migrations

```bash
# Create a new migration
just db-create-migration add_user_email_field

# Edit the generated SQL file in migrations/
# Always include:
# - Proper indexes
# - Foreign key constraints
# - Comments for documentation

# Apply the migration
just db-migrate

# Generate SQLx offline query metadata
cargo sqlx prepare
```

### Migration Guidelines

- **Never modify existing migrations** that have been merged to main
- **Always include rollback capability** (if possible)
- **Test migrations** both up and down
- **Document breaking changes** in the migration file
- **Add appropriate indexes** for query patterns

## Commit Messages

### Format

```
<type>: <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```
feat: Add device password generation endpoint

Implement POST /api/device-passwords endpoint that generates
a new device password for CalDAV authentication.

- Add password generation with crypto-secure random
- Hash passwords with Argon2id
- Store in device_passwords table
- Return plaintext password only once

Closes #123
```

```
fix: Prevent event end time before start time

Add database constraint to ensure event.end >= event.start.
This prevents invalid events from being created.

Fixes #456
```

## Pull Request Process

### Before Submitting

1. **Create a feature branch**: `git checkout -b feature/my-feature`
2. **Make changes**: Follow code quality standards
3. **Write tests**: Add tests for new functionality
4. **Run quality checks**: `just quality && just check-antipatterns`
5. **Update documentation**: If adding public APIs
6. **Commit changes**: Follow commit message format
7. **Push to GitHub**: `git push origin feature/my-feature`

### PR Template

When creating a PR, include:

- **Description**: What does this PR do?
- **Motivation**: Why is this change needed?
- **Testing**: How was this tested?
- **Screenshots**: For UI changes
- **Breaking changes**: Any API changes?
- **Related issues**: Closes #123

### Review Process

1. **Automated checks**: CI must pass (format, lint, test)
2. **Code review**: At least one approval required
3. **Security review**: For security-related changes
4. **CalDAV compliance**: For CalDAV changes, run compliance tests
5. **Merge**: Squash and merge to main

### After Merge

- **Delete branch**: Clean up feature branches
- **Monitor**: Check CI/CD pipeline
- **Verify**: Test in staging environment

## Security Considerations

### Reporting Vulnerabilities

Do not create public issues for security vulnerabilities. Email security@televent.app or create a private security advisory.

### Security Checklist

Before submitting a PR:

- [ ] No hardcoded secrets or credentials
- [ ] All passwords hashed with Argon2id
- [ ] All SQL queries use parameterized queries
- [ ] User input validated and sanitized
- [ ] Error messages don't expose internal details
- [ ] Authentication required for sensitive endpoints
- [ ] Rate limiting applied to public endpoints
- [ ] GDPR considerations addressed
- [ ] Security-relevant operations logged

## Getting Help

- **Documentation**: Check `/docs` directory
- **Issues**: Search existing issues on GitHub
- **Discussions**: Use GitHub Discussions for questions
- **Chat**: Join our Discord server (link in README)

## License

By contributing to Televent, you agree that your contributions will be licensed under the MIT License.
