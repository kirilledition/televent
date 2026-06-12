# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS frontend
WORKDIR /app/frontend
RUN corepack enable
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

FROM rust:1.88-bookworm AS backend
WORKDIR /app/backend
COPY backend/ ./
RUN cargo build --release --bin televent

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend /app/backend/target/release/televent /usr/local/bin/televent
COPY --from=backend /app/backend/migrations /app/migrations
COPY --from=frontend /app/frontend/out /app/frontend/out

ENV API_HOST=0.0.0.0 \
    FRONTEND_STATIC_DIR=/app/frontend/out \
    ENABLE_FILE_LOGGING=false

CMD ["televent"]
