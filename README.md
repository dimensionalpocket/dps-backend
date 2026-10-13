# @dimensionalpocket/dps-backend

[![Rust Tests](https://github.com/dimensionalpocket/dps-backend/actions/workflows/test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-backend/actions/workflows/test.yml) [![Docker Build Test](https://github.com/dimensionalpocket/dps-backend/actions/workflows/docker-build-test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-backend/actions/workflows/docker-build-test.yml)

An opinionated authentication, user management, and session handling backend, packaged as a monolith
composable: it serves an [Inertia.js](https://inertiajs.com/) Vue frontend from the root path `/`, owns
the `/health` endpoint, and exposes a REST API under `/{api_path}`.

It can be deployed on its own as a service with a volume to house the SQLite databases, but it can also
be embedded into a larger `axum` application. The library **never binds a socket**: the host owns
`axum::serve` and graceful shutdown. A host can merge its own API routes into the `{api_path}` namespace
via `DpsBackend::enhance_api`, and those routes inherit the backend's session, request-id, tracing, and
CORS layers.

The backend owns the root router (`/`, `/health`, `/{api_path}`, and the fallback). API paths return a
plain-text `404 NOT FOUND`; any other unknown path renders the Inertia `Error` page.

## Features

- Inertia.js frontend served from `/` (Vite-built, code-split pages)
- REST API with authentication, user, role, and site endpoints
- User management with role-based permissions
- Session token management with encrypted tokens
- Password hashing with Argon2
- SQLite database with migrations
- Composable router: merge host API routes with `enhance_api` without owning the listener
- Shared database handles exposed to the host for custom queries and data changes

## Frontend pages

The Inertia page routes are served from the root path and are auth-agnostic shells. Auth is resolved
client-side by the Vue app; "Guard" below is a client-side guard.

| Method | Path | Inertia component | Guard |
|--------|------|-------------------|-------|
| `GET` | `/` | `Home` | auth |
| `GET` | `/login` | `Auth/Login` | none |
| `GET` | `/register` | `Auth/Register` | none |
| `GET` | `/login-success` | `Auth/LoginSuccess` | auth |
| `GET` | `/password-change` | `Auth/PasswordChange` | auth |
| `GET` | `/admin` | `Admin/Home` | admin |
| `GET` | `/admin/users` | `Admin/Users` | admin |
| `GET` | `/admin/sites` | `Admin/Sites` | admin |
| `GET` | `/admin/sites/new` | `Admin/SitesNew` | admin |
| `GET` | `/admin/sites/{id}/edit` | `Admin/SitesEdit` | admin (props: `{id}`) |
| `GET` | `/health` | — (plain `"OK"`) | none |
| — | fallback (non-API) | `Error` (`404`) | none |

In production the compiled assets are served from `/build/` via `ServeDir(frontend/dist)`; in
development the Vite dev server (port `5000`) serves them and writes `frontend/hot` for the backend to
resolve.

## API Endpoints

All routes are nested under `/{api_path}` (default `/api`). Requests and responses are JSON with
camelCase field names. Errors use the envelope `{"error":"<message>"}`.

### System

| Method | Path | Auth | Request | Success |
|--------|------|------|---------|---------|
| `GET` | `{api_path}/server-timestamp` | none | — | `200 {"timestamp":"<ms>"}` |

### Auth

| Method | Path | Auth | Request body | Success |
|--------|------|------|--------------|---------|
| `POST` | `{api_path}/auth/register` | none | `{username,password,passwordConfirmation}` | `201 {user,message}` + `Set-Cookie` |
| `POST` | `{api_path}/auth/login` | none | `{username,password}` | `200 {token,user,message}` + `Set-Cookie` |
| `POST` | `{api_path}/auth/logout` | none | — | `200 {message}` + expired `Set-Cookie` |
| `PATCH` | `{api_path}/auth/password` | session | `{currentPassword,newPassword,newPasswordConfirmation}` | `200 {message,updatedTs}` |
| `GET` | `{api_path}/auth/me` | session | — | `200 {user,sessionIat,sessionExp}` / `401` |

### Sites

| Method | Path | Permission | Request body | Success |
|--------|------|------------|--------------|---------|
| `GET` | `{api_path}/sites` | none | — | `200 [site]` |
| `GET` | `{api_path}/sites/{id}` | `can_view_site_details` | — | `200 site` |
| `POST` | `{api_path}/sites` | `can_create_site` | `{slug,subdomain?,port?,protocol?,metadataJson?}` | `201 site` |
| `PATCH` | `{api_path}/sites/{id}` | `can_update_site` | `{slug?,subdomain?,port?,protocol?,metadataJson?}` | `200 site` |
| `DELETE` | `{api_path}/sites/{id}` | `can_delete_site` | — | `200 site` |

### Users

| Method | Path | Permission | Request body | Success |
|--------|------|------------|--------------|---------|
| `GET` | `{api_path}/users` | `can_list_users` | — | `200 [user]` |
| `GET` | `{api_path}/users/{id}` | `can_view_user_details` | — | `200 user` |
| `PATCH` | `{api_path}/users/{id}` | `can_edit_user` | `{name?,roleId?,password?,passwordConfirmation?,metadataJson?}` | `200 user` |
| `DELETE` | `{api_path}/users/{id}` | `can_delete_user` | — | `200 {success:true}` |

### Roles

| Method | Path | Permission | Request body | Success |
|--------|------|------------|--------------|---------|
| `GET` | `{api_path}/roles` | `can_manage_roles` OR `can_edit_user_role` | — | `200 [role]` |
| `GET` | `{api_path}/roles/{id}` | `can_manage_roles` | — | `200 role` |
| `GET` | `{api_path}/role-permissions` | `can_manage_roles` | — | `200 [String]` |
| `POST` | `{api_path}/roles` | `can_manage_roles` | `{name,permissions:[String]}` | `201 role` |
| `PATCH` | `{api_path}/roles/{id}` | `can_manage_roles` | `{name?,permissions?}` | `200 role` |
| `PUT` | `{api_path}/roles/{id}/default` | `can_manage_roles` | — | `200 role` |
| `DELETE` | `{api_path}/roles/{id}` | `can_manage_roles` | — | `200 {success,id,name}` |

### JSON shapes

- `site`: `{id, slug, subdomain, port, protocol, metadataJson, createdTs, updatedTs}`
- `user`: `{id, uuid, name, role:{id, name, permissions:[String]}, metadataJson, createdTs, updatedTs}`
- `role`: `{id, name, isDefault, permissions:[String], createdTs, updatedTs}`

### Error status codes

| Domain error | Status |
|---|---|
| `ValidationError`, `InvalidPermission` | `400 Bad Request` |
| `AuthenticationError`, invalid credentials, no session | `401 Unauthorized` |
| `AuthorizationError` ("Forbidden") | `403 Forbidden` |
| `SiteNotFound`, `UserNotFound`, `RoleNotFound` | `404 Not Found` |
| `UsernameAlreadyExists`, `RoleNameAlreadyExists`, `SlugAlreadyExists`, `RoleInUse`, `SelfDeletion` | `409 Conflict` |
| Database/configuration/internal errors | `500 Internal Server Error` |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dps-backend = { git = "https://github.com/dimensionalpocket/dps-backend", tag = "0.1.0" }
```

## Usage

### Quickstart (monolith composable)

```rust
use axum::Router;
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // DpsConfig loads from environment variables automatically.
    // Requires DPS_BACKEND_SESSION_SECRET to be set.
    let config = DpsConfig::new();
    let backend = DpsBackend::new(config)?;

    // The backend owns `/`, `/health`, `/{api_path}`, and the fallback.
    // The host contributes only its own API routes (paths relative to the API root).
    let host_api = Router::new();
    let app = backend.enhance_api(host_api).await?;

    // The host owns the listener and graceful shutdown.
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

> Hosts may add their own routes under the backend's `{api_path}` namespace, as long as they do not
> redefine an exact backend method + path (e.g. a duplicate `GET /auth/me`), which panics on merge.
> Distinct paths merge cleanly. See
> [Mounting host routes under the API path](#mounting-host-routes-under-the-api-path).

### Full example (complete configuration)

```rust
use dps_backend::DpsBackend;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = DpsConfig::new();

    // Override defaults
    config.set_backend_port(Some(3000));
    config.set_backend_sqlite_main_file_path("data/production.db");
    config.set_backend_session_secret(Some("your-32-byte-secret-here!!!!"));
    config.set_domain("yourdomain.com");
    config.set_backend_insecure_cookie(false);
    config.set_development_mode(false);
    config.set_backend_sqlite_main_pool_size(Some(10));

    let backend = DpsBackend::new(config)?;

    // `backend.routes().await?` returns the bare root router (no host routes).
    let app = backend.enhance_api(axum::Router::new()).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### Accessing the databases from the host

The backend lazily creates and caches a single `DpsBackendDatabases` instance and shares it with the
embedded router, so the host can borrow the exact same connection pools. Call `DpsBackend::databases()`
to obtain a `&DpsBackendDatabases` and run your own queries — including inserts, updates, and deletes —
against the main and session databases:

```rust
use dps_backend::DpsBackend;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DpsConfig::new();
    let backend = DpsBackend::new(config)?;

    // Lazily created on first call and cached; the same instance is used by
    // `migrate_database()`, `seed_database()`, and `routes()`/`enhance_api()`.
    let databases = backend.databases().await?;

    // Acquire a connection from either pool.
    let mut conn = databases.main().acquire().await?;
    sqlx::query("UPDATE sites SET port = ? WHERE id = ?")
        .bind(8443)
        .bind(1)
        .execute(&mut *conn)
        .await?;

    // Session data is available through `databases.session()`.
    let _session_pool = databases.session();

    let app = backend.enhance_api(axum::Router::new()).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

> Hosts that run raw SQL must add `sqlx` (with the `sqlite` feature) to their own dependencies. The
> `DpsBackendDatabases` handle is `Clone` (cloning it clones the underlying pools), so it can be moved
> into tasks and other threads. The library's own query and service modules are also public if you
> prefer to reuse them with a connection acquired from the pool.

### Mounting host routes under the API path

The session cookie is issued with `Path=/{api_path}` (e.g. `/api`) and `Domain=.{domain}`, so browsers
only send it to requests under that path. The backend owns the root router, so host routes are merged
into the backend's `{api_path}` sub-router with `enhance_api`. Host routes are passed as paths
**relative to the API root** (e.g. `/profile`) and automatically inherit the backend's session,
request-id, tracing, and CORS layers — no host-side middleware is needed.

The only rule is that the host must not redefine an exact backend method + path (e.g. a duplicate
`GET /auth/me`), which panics on merge. Distinct paths under `{api_path}` merge cleanly.

```rust
use axum::{extract::Extension, routing::get, Router};
use dps_backend::middleware::session::SessionContext;
use dps_backend::DpsBackend;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DpsConfig::new();
    let backend = DpsBackend::new(config)?;

    // Paths are relative to the API root; the session middleware is applied by the backend.
    let host_api = Router::new().route("/profile", get(host_profile));

    let app = backend.enhance_api(host_api).await?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn host_profile(
    Extension(session): Extension<SessionContext>,
) -> &'static str {
    if session.authenticated() {
        "profile"
    } else {
        "anonymous"
    }
}
```

> Host handlers can extract `Extension<SessionContext>` to read the validated session, exactly like the
> backend's own handlers.

## Frontend

The Vue 3 frontend lives in [`frontend/`](./frontend) and is served by the backend as an Inertia.js
monolith. Pages are declared in `src/handlers/pages.rs` and mounted in `DpsBackend::build_root`;
component files live under `frontend/src/Pages/`, with shared shells in `frontend/src/Layouts/`.

Auth is client-side: the Inertia page shell carries no auth, and the Vue app resolves the session by
calling `GET /{api_path}/auth/me`. The session cookie stays `HttpOnly` and scoped to `Path=/{api_path}`,
so it is sent on that request but is not readable by JavaScript.

Commands (run inside `frontend/`):

- **Install**: `bun install`
- **Dev server**: `bun run dev` (port `5000`; writes `frontend/hot`)
- **Build**: `bun run build` (writes `frontend/dist`, including `.vite/manifest.json`)

Asset resolution:

- **Dev** (`DPS_DEVELOPMENT_MODE=Y`): the backend reads `frontend/hot` (or falls back to its hardcoded
  `http://localhost:5000` default) and injects a module script tag pointing at the Vite dev server. No
  `/build` static route is mounted.
- **Prod**: the backend reads `frontend/dist/.vite/manifest.json`, injects the hashed
  `/{base}{file}` script and CSS links into the root document, and serves `frontend/dist` under
  `/build`.

To add a page:

1. Create `frontend/src/Pages/<Name>.vue`.
2. Add a route/handler in `src/handlers/pages.rs` and mount it in `DpsBackend::build_root`.
3. Update the "Frontend pages" table above.

## Configuration

The `DpsBackend` uses `DpsConfig` from the `dps-config` crate for configuration management.

### Environment Variables

All configuration is loaded from environment variables via `DpsConfig::new()`:

- `DPS_DOMAIN`: Base domain (default: "dps.localhost")
- `DPS_API_PATH`: API path, without leading slash (default: "api", meaning the backend serves at "/api")
- `DPS_DEVELOPMENT_MODE`: Set to "Y" to enable development features (default: not defined)
- `DPS_BACKEND_PORT`: Server port used by the example host binary (default: 3000 if not set)
- `DPS_BACKEND_SQLITE_MAIN_FILE_PATH`: SQLite database file path (default: "data/main-development.db")
- `DPS_BACKEND_SQLITE_MAIN_POOL_SIZE`: Database connection pool size (default: 1)
- `DPS_BACKEND_SESSION_SECRET`: 32-byte secret for session encryption (required)
- `DPS_BACKEND_INSECURE_COOKIE`: Set to "Y" to enable insecure cookies (default: false)

Frontend asset resolution (asset base, dist path, hot file, entry, and dev-server URL) is fixed in code
and coupled to the frontend build; it is not configurable via environment variables. Embedders can
override it programmatically with `DpsBackend::with_frontend_config`.

The cookie domain is automatically derived as `.{domain}` (e.g., ".dps.localhost").
The cookie path is set to the `api_path` configuration with a leading slash (e.g., "/api").

> `DpsConfig` is an external crate and exposes `backend_*` getters and `DPS_BACKEND_*` environment
> variables.

### Manual Configuration

```rust
let mut config = DpsConfig::new();
config.set_backend_port(Some(8080));
config.set_domain("example.com");
config.set_backend_sqlite_main_pool_size(Some(10));
let backend = DpsBackend::new(config)?;
```

All configuration options except `backend_session_secret` have sensible defaults and are optional.

## Local Development

For local development, set the environment variables (see the "Environment Variables" section) and run
the example host binary:

```bash
# Set environment variables in .env file, then:
cargo run --bin run_local_server
```

The example host (`scripts/run_local_server.rs`) contributes no API routes of its own; it calls
`enhance_api(Router::new())`, owns `axum::serve`, and handles graceful shutdown. The backend owns `/`,
`/health`, `/{api_path}`, and the fallback.

## Database Setup

The library provides a `dps-backend-migrate` binary for database setup and rollback.

### Forward Migrations

- Build (debug): `cargo build --bin dps-backend-migrate`
- Build (release): `cargo build --release --bin dps-backend-migrate`
- Run migrations (development): `cargo run --bin dps-backend-migrate`
- Run with custom SQLite file: `cargo run --bin dps-backend-migrate -- --sqlite-file ./my-auth.db`
- Production: `./target/release/dps-backend-migrate --sqlite-file ./production.db`

### Migration Rollback

Rollback the last migration:
```bash
cargo run --bin dps-backend-migrate -- --revert
```

Rollback multiple migrations:
```bash
cargo run --bin dps-backend-migrate -- --revert 3
```

Rollback in production:
```bash
./target/release/dps-backend-migrate --sqlite-file ./production.db --revert
```

**Important**: Rollback will automatically regenerate the schema dump after reverting migrations.

Docker note:
- Build the migration binary in your builder stage and run it before starting your app in the runtime stage. See the repo Dockerfile or docs for an example.

### Configuration Methods

1. **Environment Variables**:
```bash
export DPS_BACKEND_SQLITE_FILE="data/development.db"
export DPS_BACKEND_MIGRATE_SKIP_SEEDS="false"
```

2. **Command-line Arguments**:
   - `--sqlite-file <PATH>` - SQLite database file path
   - `--skip-seeds` - Skip running seed files
   - `--revert [N]` - Revert last N migrations (default: 1)
   - `--help` - Show all available options

## Development

- Use `mise` for managing Rust toolchains and dependencies. See `.tool-versions` file for required versions.
- To install `rust-analyzer`: `rustup component add rust-analyzer`
  - This is recommended for IDE support (e.g., VSCode with Rust Analyzer extension).

## License

[MIT](./LICENSE)
