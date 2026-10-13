# AGENTS.md

## General Instructions

- Be concise and direct.
- Treat all questions as genuine inquiries for information; don't interpret questions as implying mistakes.

## Build/Lint/Test Commands

Run every Rust command through mise by default: prefix `cargo`/`rustc` with `mise exec --`
(e.g. `mise exec -- cargo test`). The bare `rustc`/`cargo` on PATH may be a different toolchain.

- **Build**: `mise exec -- cargo build`
- **Build release**: `mise exec -- cargo build --release`
- **Test all**: `mise exec -- cargo test --quiet`
- **Test single test**: `RUST_BACKTRACE=1 mise exec -- cargo test <test_name> --quiet`
  - Note: never use `--verbose` as it clutters output
- **Linter**: `mise exec -- cargo clippy --allow-dirty --fix && mise exec -- cargo fmt`
- **Run local server**: `mise exec -- cargo run --bin run_local_server`
- **Database migrations**: `mise exec -- cargo run --bin dps-backend-migrate`

## Working Guidelines

### Before Starting Tasks
- Read the `README.md` to get project context and progress
- When needing current date/time, run `date +%Y-%m-%d@%H:%M` for YYYY-MM-DD@HH:MM format (24-hour, no AM/PM)
- Git operations (commits, branches, merges, rebases) are NOT part of agent tasks

### Working with Plans
- Save plans in `docs/llm/plans/YYYY/MM/` directory structure
- Name files as `DD-task-name.md` (day + dash + lowercase task with dashes)
- Include implementation details: files to modify/create, code samples for new functions
- If applicable, suggest existing dependencies that can help with the implementation during plan creation; if multiple dependencies exist, list pros/cons and recommend one
- Never include git operations, changelogs, PR descriptions, version bumps, or release notes in plans
- Never worry about backwards compatibility if the version is pre-1.0.0
- If the plan involves creating, updating, or deleting REST handlers/routes, add a final phase on the plan to update the `README.md` REST endpoint table after all implementation is done
- When writing plans that involve creating handlers, queries, services, and/or controllers in the same plan, split into sub-tasks in this order, where each subtask is fully tested before moving to the next:
  - Queries first (if any)
  - Services second (if any)
  - Controllers third (if any)
  - REST handlers last
- Do not code immediately after writing plans - wait for user review/approval

### Implementing Plans
- Only start implementation after explicit user command
- During implementation, STOP and inform user if deviating from the plan in any way
- Never implement anything not explicitly mentioned in the plan
- After implementation is fully finished and all tests pass, run the linter command and fix any issues

## Tool Usage

### File Editing
When using the Edit tool to modify files:
- Preserve ALL existing comments and documentation
- Make surgical, targeted edits - only change the specific lines that need to change
- Comments that are unrelated to your change must remain intact

### Bun Scripting

- For large-scale replacement tasks with predictable patterns, write Bun scripts in `scripts/` to make project-wide changes in one go.
- Avoid resorting to Bun scripts if the task is small enough (e.g., changes to a couple of files), or patterns are not predictable enough.
- If for some reason Bun is not available if you need to write a script, STOP and tell the user; do not try to write the script in another language, neither try to install Bun yourself.

### Git Access

- Do not run commit/push commands without explicit permission from user. 
- Usually the user will review the code manually after plans are finished, so never commit/push after implementing plans.
- Do not use Git worktrees unless explicitly instructed to by user or plans.
- Use semantic commit messages (feat:, fix:, etc) regardless of commit message history. Keep messages short and concise. No need for long descriptions in a new line.

## Project Structure

### Domains

Domain usage is more of a way to organize code than strict boundaries. Cross-domain calls are common and acceptable when needed.

- `auth` - workflows related to authentication such as sign-up, login/logout, password recovery, etc
- `user` - user CRUD operations
- `role` - role CRUD operations and permission checks (authorization)
- `site` - site CRUD operations
- `session` - session management and validation
- `password` - password hashing, validation, and other checks
- `shutdown` - application shutdown handling

### Types

- App-wide types live in `src/types/`
- HTTP request/response DTOs live in `src/types/http/`

### API Setup
- The library is a monolith composable `axum` router.
- `DpsBackend` owns the root router: the Inertia page routes at `/`, `GET /health`, `/{api_path}`, and the root fallback.
- `DpsBackend::routes()` returns the full root router (no host routes); `DpsBackend::enhance_api(host_api).await?` merges host API routes (paths relative to the API root) into the `/{api_path}` sub-router and returns the root router. Host routes inherit the backend's session/request-id/tracing/CORS layers.
- `DpsBackend` can optionally own the server: `serve(app)` binds `0.0.0.0:<bind port>` (resolved by `DpsConfig` as `DPS_BACKEND_BIND_PORT` -> `PORT` -> `3000`; distinct from the public `DPS_BACKEND_PORT`, which only feeds URLs) and `serve_listener(listener, app)` / `serve_with_shutdown(listener, app, signal)` cover custom listeners and shutdown. `routes()` / `enhance_api()` are still available for hosts that own `axum::serve` themselves.
- API paths under `/{api_path}` return a plain-text `404 NOT FOUND`; other unknown paths render the Inertia `Error` page.
- All API routes are nested under `/{api_path}` (default `/api`) and are declared in `src/dps_backend.rs::backend_api_router`.
- Inertia page handlers live in `src/handlers/pages.rs`; REST handlers live in `src/handlers/`; the API is REST-only (no GraphQL).
- Frontend asset resolution lives in `src/frontend/` (`FrontendConfig`, `FrontendAssets`) and the Vue app lives in `frontend/`.

### Middleware
- Middlewares live in `src/middleware/`
- Middlewares handle session object injection, request ID injection, and logging
  - The session object and request ID are attached to request extensions and read by handlers via `Extension<...>`
- Middlewares never halt the request; they only add context data

### Handlers (REST)
- REST handlers live in `src/handlers/` (one module per resource: `auth.rs`, `sites.rs`, `users.rs`, `roles.rs`, `system.rs`)
- Handlers are thin wrappers that call **at most** a single controller (or service) method
- Handlers handle HTTP-specific concerns (extraction, status codes, headers, error mapping)
  - They do NOT contain business logic, database access, or session management
- The session context is read from request extensions via `Extension<SessionContext>`
- Errors are returned via the `ApiError` enum in `src/handlers/error.rs`, which maps domain errors to HTTP status codes and the `{"error":"..."}` envelope
- Routes are declared in `src/dps_backend.rs::DpsBackend::routes`
- Naming conventions:
  - Handler functions are snake_case verbs matching the REST action (e.g., `list`, `show`, `create`, `update`, `remove`, `login`, `register`, `me`)
  - Request DTOs are suffixed with `Request` (e.g., `AddSiteRequest`); response DTOs with `Response` (e.g., `SiteResponse`)
  - All JSON DTOs use `#[serde(rename_all = "camelCase")]`
- Adding a route:
  1. Add the request/response DTOs in `src/types/http/`
  2. Add the handler in the relevant `src/handlers/*.rs` module
  3. Mount it in `DpsBackend::routes`
  4. Update the `README.md` REST endpoint table

### Orchestration Layer
- Controllers handle high-level workflows that may involve multiple services, authentication, and authorization
- They make the connection between REST handlers and the service layer, keeping handlers thin
- Service controllers live in `src/controllers/<domain>/`
  - One controller per handler
  - Each controller has a single static `run(...)` method
- Controllers handle the common pattern of: authentication → authorization → business logic (calling other services and/or queries)
- Controller inputs are extracted by the handler and passed down: database pool (if database access required), session context, and request inputs
- For Controllers with database usage, they extract a connection from the pool and reuse that connection to call any services and queries it needs
- Naming conventions:
  - Controller structs are named after the handler (which also describes the workflow) and suffixed with `Controller` (e.g., `AuthLoginController`, `SetDefaultRoleController`, etc)
  - Controller filenames are in snake_case matching the controller name without the suffix (e.g., `src/controllers/auth/auth_login.rs`)
  - Exception: when the controller refers to a "getter" workflow (such as `User` - one user, `Users` - multiple users, etc), use `Get` prefix (e.g., `GetUserController`, `GetUsersController`) and `get_` prefix for filenames (e.g., `get_user.rs`, `get_users.rs`)
    - This helps avoiding name conflicts with the domain name itself (e.g., `UserController` could be confused with user domain)

### Service Layer
- Services live in `src/services/<domain>/`
  - One struct per action (e.g., `CreateUserService`, `UpdateUserService`, etc
  - Each service has a single static public method named `run(...)`
- Services contain core business logic and can interact with the database layer via Query objects
  - Services should not contain SQL queries directly; if a query doesn't exist, create a new Query object in `src/queries/`
- Services can also call other services as needed
- Service inputs are a database connection (not a pool) and any parameters needed for the business logic

### Database
- Query objects live in `src/queries/`
  - Use `sqlx` to build dynamic queries
- Query objects work with a database connection (not a pool)
  - Alternatively, a transaction can be passed to the query if the caller has an active transaction
- The objects returned by queries (models) live in `src/models/`
- The project uses multiple databases:
  - The "main" database for core data (users, roles, sites, etc)
  - A separate "session" database for session management (not yet implemented)
- The Database instances are managed by `*Database` structs in `src/database/` folder.
  - There are multiple database structs: `MainDatabase` and `SessionDatabase`.
- Use async/await for database operations
- Database migrations live in `config/databases/<database_name>/migrations`
  - Each migration has two files: `.sql` (forward) and `.down.sql` (rollback)
  - Migration files contain native SQL code

### Role Permissions

- All valid role permissions are defined in a static array at `src/models/role.rs:ROLE_PERMISSIONS`. This array serves as the whitelist of all allowed permissions in the system.
- When adding new permissions or removing existing ones:
  1. Update the `ROLE_PERMISSIONS` array in `src/models/role.rs`
  2. The `is_valid_role_permission()` function will automatically validate against the updated array
  3. All permission checks throughout the codebase use this validation
- Permission checks are performed through `RoleService::check_user_permission()` which validates that the permission exists in the static array before checking the user's role.

### Other DPS Dependencies

Fetch the following URLs for the READMEs with the specs for each DPS dependency used by this project (e.g., knowing which properties/methods are exposed by DpsConfig):

- DpsConfig: https://raw.githubusercontent.com/dimensionalpocket/dps-config/refs/heads/main/README.md

### Testing

- Unit tests live in the same files as the code they test, within `#[cfg(test)]` modules
- Integration and other higher-level tests live in the `tests/` directory
- Test both success and error paths
- Use `tokio::test` for async tests
- Use `serial_test` crate for tests that rely on mutable ENV variables, or tests that otherwise cannot run in parallel
- **Timestamp Testing**: Database timestamp columns (`created_ts`, `updated_ts`) store seconds since Unix epoch. Any tests requiring timestamp differences must use delays of at least 1 second (e.g., `tokio::time::sleep(tokio::time::Duration::from_secs(1)).await`)

#### DB Test Macro

- Prefer `#[dps_backend_db_test]` (from `dps-backend-test-macros`) for tests that need a SQLite pool
- The macro wraps the function in `#[tokio::test]` and injects:
  - `databases` (main + session)
  - `main_pool` (`SqlitePool`, cloned from `databases.main()`)
- Usage in unit tests (inside `#[cfg(test)] mod tests { ... }`):

```rust
use dps_backend_test_macros::dps_backend_db_test;

#[dps_backend_db_test]
async fn test_something() {
  // use `main_pool` (and `databases` if needed)
}
```

- Usage in integration tests (`tests/*.rs`): add this once per file so the macro can resolve `crate::test_utils`:

```rust
use dps_backend::test_utils as test_utils;
use dps_backend_test_macros::dps_backend_db_test;

#[dps_backend_db_test]
async fn test_something() {
  // use `main_pool`
}
```

- Optional macro args:
  - `pool_size = <int>`
  - `configure_sqlite = <bool>`

## Code Style Guidelines

### General
- Bias towards simplicity
- Do not install any new dependencies unless they are specified in the plan

### Formatting
- Follow the rules configured in `rustfmt.toml`
- Follow rustfmt defaults for all other formatting

### Imports
- Prefer specific imports over `use *;`
- **Never use full paths in function bodies** - always import first
- **Exceptions**: Types named "Error" should use full paths in code (e.g., `sqlx::Error`) and don't need to be imported directly

```rust
use crate::models::Role;
use crate::services::UserService;
use crate::services::UserError;
use sqlx::{SqlitePool, Row};
use axum::{extract::State, Json};
use std::collections::HashMap;

fn example() -> Result<(), UserError> {
    let role = Role { ... };                    // ✅ Use imported type directly
    let pool = SqlitePool::connect(...).await?; // ✅ Use imported type directly
    let map = HashMap::new();                   // ✅ Use imported type directly
    // let role = crate::models::Role { ... };   ❌ No full paths
    // let error = sqlx::Error::RowNotFound;     // ✅ Error types use full paths
}
```

### Types & Naming
- Use PascalCase for structs, enums, and types
- Use snake_case for functions, variables, and modules
- Use SCREAMING_SNAKE_CASE for constants

### Error Handling
- Create custom error enums for each service (e.g., UserError, SessionError)
- Implement Display and Error traits for custom errors
- Use From traits for error conversion
- Return Result<T, CustomError> from service methods
- Use ? operator for error propagation

### Logging Security
- **NEVER** log passwords, API keys, tokens, or other sensitive data
- Always add sensitive parameters to the `skip` list in `#[instrument]` macros
  - E.g.: `#[instrument(skip(state, request), fields(username = %request.username))]` pattern for auth handlers (never log request bodies for auth routes)
- Only log non-sensitive identifiers like usernames for debugging purposes
- Verify no sensitive data is logged by running the password logging tests

### Test Utilities

- Shared test utilities are available in `src/test_utils/mod.rs`
- Use `create_test_user_with_*()`, `create_test_user_with_*_and_password()`, `create_test_user_full_with_*()`, `create_test_role_with_*()`, `create_test_role_model_with_*()`, and `create_test_user_via_api()` for creating test data (suffixes change based on what's used to create data: a DpsBackendDatabases instance, or a connection)
- Database setup utilities (`create_test_databases()`, `create_test_databases_with_pool_size()`, etc.) are also in `src/test_utils/mod.rs`
- Do not create local `create_test_*` functions in test modules - use the shared utilities instead

For HTTP-level (router) tests, build a fully-migrated backend in-process with `create_test_app()` and drive it with `tower::ServiceExt::oneshot`. No TCP listener or `axum::serve` is used:

```rust
use dps_backend::test_utils;
use tower::ServiceExt;

let app = test_utils::create_test_app().await;
let response = app
  .oneshot(Request::builder().uri("/api/server-timestamp").body(Body::empty()).unwrap())
  .await
  .unwrap();
```

Rules for database usage in tests:

- Use tempfile for test databases
- **Pool Usage**: Exactly one SQLite pool per test - never multiple pools within single tests
- **Unit Tests**: Use direct pool access via `create_test_databases*()` from `src/test_utils/mod.rs`
- **Configurable Tests**: Use `create_test_databases_with_config(configure_sqlite: bool)` for optional SQLite configuration
- **Integration Tests**: Use a full in-process app via `create_test_app()` in test files (no listener)
- **Ownership**: Clean ownership with automatic temp file cleanup via `NamedTempFile` dropping
- **Pool Size**: Unit tests use defaults, integration tests set to 1 connection
- **No Multiple Pools**: No tests use multiple pools within same test function
- **Arc Usage**: Not needed in tests - `SqlitePool` implements `Clone` internally and tests use single-threaded `&pool` references
