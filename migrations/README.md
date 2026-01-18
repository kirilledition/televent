# Database Migrations

SQLx migrations for Televent. Migrations are applied in order by filename.

## Creating a New Migration

```bash
just db-create-migration <descriptive_name>
```

Example:
```bash
just db-create-migration create_users_table
```

This creates a new migration file: `migrations/<timestamp>_<descriptive_name>.sql`

## Applying Migrations

```bash
just db-migrate
```

## Rolling Back

```bash
just db-rollback
```

## Important Notes

- Always create migrations after changing `crates/core/src/models.rs`
- Run `cargo sqlx prepare` after creating migrations to generate offline query metadata
- Never modify existing migrations that have been merged to main
- SQLx validates queries at compile time - missing migrations break the build

## Migration Naming Convention

Use descriptive names that explain what the migration does:
- `create_users_table`
- `add_timezone_to_users`
- `create_events_index`
- `add_device_passwords`
