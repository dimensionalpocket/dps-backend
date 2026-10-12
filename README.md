# @dimensionalpocket/dps-backend

[![Rust Tests](https://github.com/dimensionalpocket/dps-backend/actions/workflows/test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-backend/actions/workflows/test.yml) [![Docker Build Test](https://github.com/dimensionalpocket/dps-backend/actions/workflows/docker-build-test.yml/badge.svg)](https://github.com/dimensionalpocket/dps-backend/actions/workflows/docker-build-test.yml)

An opinionated authentication, user management, and session handling backend, exposed as an embeddable
`axum` REST router.

It is intended to be deployed as a microservice with a volume to house the SQLite databases, but it can
also be embedded into a larger `axum` application. The library **never binds a socket**: it enhances a
host-provided router with its own `{api_path}`-prefixed routes, and the host owns `axum::serve` and
graceful shutdown.

## Features

- REST API with authentication, user, role, and site endpoints
- User management with role-based permissions
- Session token management with encrypted tokens
- Password hashing with Argon2
- SQLite database with migrations
- Embeddable router: enhance any host `axum::Router` without owning the listener
- Shared database handles exposed to the host for custom queries and data changes

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

### Quickstart (embeddable router)

```rust
use axum::{routing::get, Router};
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // DpsConfig loads from environment variables automatically.
    // Requires DPS_BACKEND_SESSION_SECRET to be set.
    let config = DpsConfig::new();
    let backend = DpsBackend::new(config)?;

    // The host owns the listener, root, health check, fallback, and graceful shutdown.
    let host = Router::new()
        .route("/", get(|| async { "OK" }))
        .route("/health", get(|| async { "OK" }));

    let app = backend.enhance(host).await?;

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

> Embedders must not define their own routes under the backend's `{api_path}` namespace, since merging
> overlapping routes panics.

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

    // `backend.routes::<()>().await?` returns just the `{api_path}` routes if you
    // prefer to merge them yourself, or use `backend.enhance(host).await?`.
    let app = backend.enhance(axum::Router::new()).await?;
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
    // `migrate_database()`, `seed_database()`, and `routes()`/`enhance()`.
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

    let app = backend.enhance(axum::Router::new()).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

> Hosts that run raw SQL must add `sqlx` (with the `sqlite` feature) to their own dependencies. The
> `DpsBackendDatabases` handle is `Clone` (cloning it clones the underlying pools), so it can be moved
> into tasks and other threads. The library's own query and service modules are also public if you
> prefer to reuse them with a connection acquired from the pool.

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

The example host (`scripts/run_local_server.rs`) owns `GET /`, `GET /health`, the fallback handler,
`axum::serve`, and graceful shutdown.

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
