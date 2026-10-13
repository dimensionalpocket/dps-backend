#!/bin/bash

# Change to project root directory
cd "$(dirname "$0")/../.."

# Run the migration script (dotenvy handles .env loading)
echo "Running database migrations, seeds, and schema dumps..."
mise exec -- cargo run --bin dps-backend-migrate

echo "Migration script completed."
