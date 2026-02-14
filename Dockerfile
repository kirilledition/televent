# Stage 1: Build Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy workspace config (if any) and package files
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy frontend source
COPY frontend/ ./

# Build frontend (Next.js static export)
# Set API URL to relative path for same-origin requests
ENV NEXT_PUBLIC_API_URL=/api
RUN pnpm build


# Stage 2: Build Backend
FROM rust:1.81-slim-bookworm AS backend-builder
WORKDIR /app/backend

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy backend files (including .sqlx for offline mode)
COPY backend/ ./

# Set SQLx offline mode
ENV SQLX_OFFLINE=true

# Build release binary
RUN cargo build --release --bin televent


# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/backend/target/release/televent ./televent-server

# Copy frontend static files
COPY --from=frontend-builder /app/frontend/out ./public

# Set environment variables
ENV RUST_LOG=info
ENV STATIC_DIR=/app/public
ENV API_HOST=0.0.0.0
ENV API_PORT=3000

# Expose port
EXPOSE 3000

# Run the server
CMD ["./televent-server"]
