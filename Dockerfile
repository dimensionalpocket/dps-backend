# ---- Frontend Build Stage ----
FROM oven/bun:1.4.2 AS frontend-builder

WORKDIR /app

# REST API path baked into the client bundle (must match DPS_API_PATH at runtime).
ARG VITE_DPS_API_PATH=api
ENV VITE_DPS_API_PATH=${VITE_DPS_API_PATH}

# Install dependencies first for better layer caching.
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile

# Build the Vite bundle (emits dist/ and dist/.vite/manifest.json).
COPY frontend/ ./
RUN bun run build

# ---- Backend Build Stage ----
FROM rust:1.99.0-bookworm AS builder

WORKDIR /app

# Copy source code and Cargo files
COPY . .

# Build all binaries in release mode
RUN cargo build --release

# ---- Final Stage ----
FROM debian:bookworm-slim

# Install necessary system dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy both compiled binaries from builder
COPY --from=builder /app/target/release/run_local_server /app/dps-backend
COPY --from=builder /app/target/release/dps-backend-migrate /app/dps-backend-migrate

# Copy database configuration (migrations, seeds, schema)
COPY --from=builder /app/config/databases /app/config/databases

# Copy the built frontend bundle; the backend serves it at /build from this path.
COPY --from=frontend-builder /app/dist /app/frontend/dist

# Copy startup script (will be made executable before copying)
COPY start.sh /app/start.sh

RUN mkdir -p data

# Set default port if not provided (hosts like Railway inject PORT at runtime).
ENV PORT=3000

# Expose port from environment variable
EXPOSE $PORT

# Run the startup script
CMD ["./start.sh"]
