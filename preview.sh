#!/usr/bin/env bash
#
# Production simulation for local development.
#
# Builds the frontend with Vite, then runs the backend in production mode so it
# serves the compiled bundle from frontend/dist via ServeDir at /build (instead
# of proxying to the Vite dev server). No Vite dev server is started.
#
# Usage: ./preview.sh
#
# Requirements:
#   - a .env file at the repo root (loaded by run_local_server)
#   - a migrated/seeded database (cargo run --bin dps-backend-migrate)
#
# App URL: http://dps.localhost:3000

set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

BACKEND_PID=""
SHUTTING_DOWN=0
STARTED_PID=""

log() { printf '[preview] %s\n' "$*"; }

# Start a command in its own session/process group so a single signal reaches
# the command and everything it spawns. Sets the global STARTED_PID.
start_bg() {
  local dir="$1"
  shift
  if command -v setsid >/dev/null 2>&1; then
    ( cd "$dir" && exec setsid "$@" ) &
  else
    ( cd "$dir" && exec "$@" ) &
  fi
  STARTED_PID=$!
}

# Signal a process group, falling back to the bare PID.
signal_pid() {
  local pid="$1" signal="$2"
  [ -n "$pid" ] || return 0
  kill -s "$signal" -- "-$pid" 2>/dev/null || kill -s "$signal" -- "$pid" 2>/dev/null || true
}

cleanup() {
  local signal="$1"
  local code="${2:-0}"

  if [ "$SHUTTING_DOWN" -eq 1 ]; then
    return
  fi
  SHUTTING_DOWN=1

  log "Received $signal, shutting down..."

  if [ -n "$BACKEND_PID" ]; then
    signal_pid "$BACKEND_PID" "$signal"
    wait "$BACKEND_PID" 2>/dev/null || true
  fi

  log "Backend stopped."
  exit "$code"
}

trap 'cleanup INT 130' INT
trap 'cleanup TERM 143' TERM

if [ ! -f .env ]; then
  log "Warning: .env not found; the backend may fail without DPS_BACKEND_SESSION_SECRET."
fi

if [ ! -d frontend/node_modules ]; then
  log "Warning: frontend/node_modules missing; run 'cd frontend && mise exec -- bun install'."
fi

log "Building frontend (bun run build)..."
if ! ( cd frontend && mise exec -- bun run build ); then
  log "Frontend build failed; aborting."
  exit 1
fi

if [ ! -f frontend/dist/.vite/manifest.json ]; then
  log "Frontend manifest missing (frontend/dist/.vite/manifest.json); aborting."
  exit 1
fi

# Force production asset resolution regardless of .env. dotenvy does not
# override environment variables that are already set, so this wins.
export DPS_DEVELOPMENT_MODE=N

log "Starting backend in production mode (serving /build from frontend/dist)..."
start_bg . mise exec -- cargo run --bin run_local_server
BACKEND_PID="$STARTED_PID"

log "Backend PID $BACKEND_PID"
log "App: http://dps.localhost:3000"

# Run until the backend exits; SIGINT/SIGTERM is handled by the trap.
wait "$BACKEND_PID" 2>/dev/null
EXIT_CODE=$?

if [ "$SHUTTING_DOWN" -eq 0 ]; then
  log "Backend exited (code $EXIT_CODE)."
  cleanup TERM "$EXIT_CODE"
fi
