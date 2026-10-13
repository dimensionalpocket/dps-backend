#!/usr/bin/env bash
#
# Run the backend and the Vite dev server for local development, and shut both
# down gracefully on SIGINT/SIGTERM.
#
# Usage: ./dev.sh
#
# Requirements:
#   - a .env file at the repo root (loaded by run_local_server)
#   - a migrated/seeded database (cargo run --bin dps-backend-migrate)
#   - frontend dependencies installed (cd frontend && mise exec -- bun install)
#
# App URL: http://dps.localhost:3000

set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

BACKEND_PID=""
FRONTEND_PID=""
SHUTTING_DOWN=0
STARTED_PID=""

log() { printf '[dev] %s\n' "$*"; }

# Start a command in its own session/process group so a single signal reaches
# the command and everything it spawns (e.g. cargo -> server, bun -> vite).
# Sets the global STARTED_PID. Must not be called in a command substitution,
# otherwise the backgrounded process keeps the substitution pipe open.
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

  signal_pid "$FRONTEND_PID" "$signal"
  signal_pid "$BACKEND_PID" "$signal"

  if [ -n "$FRONTEND_PID" ]; then wait "$FRONTEND_PID" 2>/dev/null || true; fi
  if [ -n "$BACKEND_PID" ]; then wait "$BACKEND_PID" 2>/dev/null || true; fi

  log "All processes stopped."
  exit "$code"
}

trap 'cleanup INT 130' INT
trap 'cleanup TERM 143' TERM

if [ ! -f .env ]; then
  log "Warning: .env not found; run_local_server may fail without DPS_BACKEND_SESSION_SECRET."
fi

if [ ! -d frontend/node_modules ]; then
  log "Warning: frontend/node_modules missing; run 'cd frontend && mise exec -- bun install'."
fi

log "Starting Vite dev server (frontend)..."
start_bg frontend mise exec -- bun run dev
FRONTEND_PID="$STARTED_PID"

log "Starting backend..."
start_bg . mise exec -- cargo run --bin run_local_server
BACKEND_PID="$STARTED_PID"

log "Backend PID $BACKEND_PID, frontend PID $FRONTEND_PID"
log "App: http://dps.localhost:3000"

# Wait until either process exits; then bring the other down too.
while kill -0 "$BACKEND_PID" 2>/dev/null && kill -0 "$FRONTEND_PID" 2>/dev/null; do
  sleep 1
done

EXIT_CODE=0
if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
  wait "$BACKEND_PID" 2>/dev/null || EXIT_CODE=$?
  log "Backend exited (code $EXIT_CODE)."
elif ! kill -0 "$FRONTEND_PID" 2>/dev/null; then
  wait "$FRONTEND_PID" 2>/dev/null || EXIT_CODE=$?
  log "Frontend exited (code $EXIT_CODE)."
fi

cleanup TERM "$EXIT_CODE"
