# Deployment Guide

This guide describes how to deploy the Televent unified server (Backend + Frontend) to Railway.

## Prerequisites

1.  **Railway Account**: [railway.app](https://railway.app/)
2.  **Supabase Project**: Ensure you have a Supabase project with PostgreSQL.
3.  **Telegram Bot Token**: From [@BotFather](https://t.me/BotFather).
4.  **GitHub Repository**: Ensure this code is pushed to a GitHub repository connected to Railway.

## Deployment Steps

### 1. Database Setup (Supabase)

Televent uses SQLx with offline mode, meaning migrations are embedded in the binary. However, for the initial run, you should ensure the database is reachable.

1.  Get your **Transaction Connection Pooler** string from Supabase (Settings -> Database -> Connection Pooling).
    *   Format: `postgres://postgres.xxxx:[password]@aws-0-region.pooler.supabase.com:6543/postgres?pgbouncer=true`
    *   **Crucial**: Add `?pgbouncer=true` if using the transaction pooler (port 6543).
    *   If using direct connection (port 5432), set `DATABASE_MAX_CONNECTIONS` to a lower value (e.g., 5-10) in Railway.

### 2. Railway Project Setup

1.  **Create a New Project** on Railway.
2.  **Deploy from GitHub**: Select your repository.
3.  **Variables**: Configure the following Environment Variables in Railway *before* the build completes (or redeploy after setting them).

### Environment Variables Checklist

| Variable | Description | Example / Default |
| :--- | :--- | :--- |
| `DATABASE_URL` | **Required**. Connection string to PostgreSQL. | `postgres://user:pass@host:6543/db?pgbouncer=true` |
| `TELEGRAM_BOT_TOKEN` | **Required**. Token from BotFather. | `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11` |
| `RUST_LOG` | Logging level. | `info,api=debug,bot=debug,worker=debug` |
| `API_HOST` | Host to bind to. | `0.0.0.0` (Default in Dockerfile) |
| `API_PORT` | Port to bind to. | `3000` (Default in Dockerfile) |
| `PORT` | **Required by Railway**. Railway injects this. | `3000` (Make sure it matches `API_PORT` or just use `API_PORT` if Railway maps it) |
| `API_BURST_SIZE` | Rate limit burst size. | `50` |
| `API_PERIOD_MS` | Rate limit period in ms. | `1000` |
| `CALDAV_BURST_SIZE` | CalDAV rate limit burst. | `20` |
| `CALDAV_PERIOD_MS` | CalDAV rate limit period. | `60000` |
| `WORKER_POLL_INTERVAL_SECS` | Worker poll interval. | `10` |
| `SMTP_HOST` | SMTP server for emails (Optional). | `smtp.example.com` |
| `SMTP_PORT` | SMTP port. | `587` |
| `SMTP_USERNAME` | SMTP username. | `user@example.com` |
| `SMTP_PASSWORD` | SMTP password. | `password` |
| `SMTP_FROM` | From address. | `noreply@example.com` |

**Note on PORT**: Railway sets the `PORT` environment variable and expects the app to listen on it.
Televent uses `API_PORT`. You can set `API_PORT` to reference `${PORT}` in Railway, or just set `API_PORT` to `3000` and Railway will detect the exposed port from Dockerfile (`EXPOSE 3000`).
*Recommendation*: Set `API_PORT=3000` in variables (or leave default) and ensure Railway "Networking" section maps public port to 3000.

### 3. Build & Deploy

Railway should automatically detect the `Dockerfile` at the root and start the build.
The build process involves:
1.  Building the Frontend (Next.js static export).
2.  Building the Backend (Rust release binary).
3.  Packaging them into a lightweight runtime image.

### 4. Verification

1.  Open the public URL provided by Railway (e.g., `https://televent-production.up.railway.app`).
2.  You should see the Frontend (Next.js app).
3.  Test the API: `https://.../api/health` should return `200 OK`.
4.  Test the Bot: Send `/start` to your bot on Telegram.

## Maintenance

### SQLx Offline Mode

If you modify SQL queries in the Rust code, you **must** update the offline query cache before committing:

1.  Make sure your local `.env` has a valid `DATABASE_URL`.
2.  Run:
    ```bash
    cd backend
    cargo sqlx prepare
    ```
3.  Commit the updated `.sqlx` directory.

If you don't do this, the Railway build will fail because it cannot connect to the database during build time (and shouldn't).

### Migrations

Migrations are automatically run on startup by the server. Ensure the database user has permissions to create tables.
