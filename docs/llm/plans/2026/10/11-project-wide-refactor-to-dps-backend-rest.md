# Project-wide refactor: dps-auth-api → dps-backend (REST, embeddable router)

## Date Created

2026-10-11@14:23

## Summary

A major project-wide refactor of this crate:

1. **Rename** `dps-auth-api` → `dps-backend` (Cargo package, crate identifier, GitHub URL, binaries, Docker
   artifacts, docs).
2. **Remove GraphQL entirely** (`async-graphql`, `async-graphql-axum`, `src/graphql/**`) and expose the same
   capabilities as **REST routes** built on `axum`.
3. **Rename orchestrators → controllers** (identical behavior, only the layer name and struct suffix change).
4. **Behavior change**: the library no longer starts a server. Instead it receives a host-provided
   `axum::Router` and **enhances** it with its routes. `DpsBackend` never binds a socket or owns a listener.
5. **Tests become unit-oriented**: prefer calling controllers directly, and where HTTP is needed use
   `tower::ServiceExt::oneshot` against the `Router` in-process (no TCP listener, no server).
6. **Unchanged**: SQLite behavior, the controller→service→query layering, and the `{api_path}` prefix.

Version is `0.1.0`, so **no backwards-compatibility guarantees are required**.

## Verification findings (feasibility)

The requested behavior was verified against `axum` 0.8.6 (already a dependency):

| Question | Answer |
|---|---|
| Can a host axum app be enhanced with new routes? | **Yes.** `axum::Router::merge(other)` and `Router::nest(path, other)` compose routers. `Router` is the "server instance" unit in axum 0.8 — there is no mutable server struct. |
| Does the host need to share a state type? | **Not necessarily.** A router built with `.with_state(state)` returns `Router<S2>` for *any* chosen `S2`, so `DpsBackend::enhance<S>(Router<S>) -> Router<S>` can merge into a host router regardless of the host's own state type. |
| Does enhancing require binding a port / starting a server? | **No.** Routes are just a data structure until the host calls `axum::serve`. |
| Can tests hit endpoints without a server? | **Yes.** `tower::ServiceExt::oneshot` drives a `Router<()>` in-process. The current suite already does this. Alternatively, controllers can be unit-tested directly with a `Databases`/connection. |
| Does the library need to keep `axum::serve`? | **No.** The example host binary (`run_local_server`) owns binding + `axum::serve` + graceful shutdown. |

The idiomatic pattern (axum docs, "Merging routers with state" / "Nesting routers with state"):

```rust
// library: returns Router<S> generic over the host's state type
let app = host_router.merge(backend.routes::<S>().await?);
```

Note: axum routers are immutable/functional, so "enhancing a server" means **returning the enhanced
`Router`**, not mutating in place. The host writes `let app = backend.enhance(app).await?;`.

### Constraint discovered: `dps-config` naming

`DpsConfig` is an **external crate** and only exposes `auth_api_*` getters and `DPS_AUTH_API_*` env vars
(e.g. `get_auth_api_port()`, `DPS_AUTH_API_SESSION_SECRET`). This repo cannot rename those. **Decision: keep
using `get_auth_api_*` / `DPS_AUTH_API_*` unchanged** (a rename inside `dps-config` is a separate future PR).

## Decisions (confirmed)

- **Root & health**: the library registers **only** `{api_path}`-prefixed routes. The example host binary owns
  `GET /` and `GET /health` (keeps Docker/Railway health checks working).
- **Config/env**: keep `auth_api_*` / `DPS_AUTH_API_*` as-is.
- **Test macros**: rename crate `dps-auth-test-macros` → `dps-backend-test-macros` and macro
  `#[dps_auth_db_test]` → `#[dps_backend_db_test]`.
- **REST verbs**: `POST` for creates/auth actions, `PATCH` for partial updates, `DELETE` with path `{id}` for
  removals; `PUT` only for the idempotent `set-default-role` action.

## Rename rules (mechanical)

| From | To |
|---|---|
| `dps-auth-api` (package, repo URL, Docker tag) | `dps-backend` |
| `dps_auth_api` (crate identifier / imports) | `dps_backend` |
| `DpsAuthApi` | `DpsBackend` |
| `DpsAuthApiError` | `DpsBackendError` |
| `src/dps_auth_api.rs` | `src/dps_backend.rs` |
| `dps-auth-api-migrate` (binary) | `dps-backend-migrate` |
| `config/scripts/dps_auth_api_migrate.rs` | `config/scripts/dps_backend_migrate.rs` |
| `dps-auth-test-macros` | `dps-backend-test-macros` |
| `#[dps_auth_db_test]` | `#[dps_backend_db_test]` |
| `src/orchestrators/` | `src/controllers/` |
| `XxxOrchestrator` | `XxxController` |
| README `@dimensionalpocket/dps-auth-api` + badge URLs | `@dimensionalpocket/dps-backend` |
| Dockerfile `/app/dps-auth-api` | `/app/dps-backend` |
| workflow tag `dps-auth-api:test` | `dps-backend:test` |
| `run_local_server` binary name | **keep** (it is the host example; path can stay `scripts/run_local_server.rs`) |

Because the replacement is large but highly predictable, implement it with a Bun script
(`scripts/rename-dps-auth-api-to-dps-backend.ts`) rather than by hand, followed by a manual review pass.

**Rename scope — the script MUST update:**

- `Cargo.toml`, `Cargo.lock` (regenerated), `crates/dps-auth-test-macros/Cargo.toml` + `src/lib.rs`
- `src/**` (imports, `DpsAuthApi*`, `#[dps_auth_db_test]`; file rename `src/dps_auth_api.rs` →
  `src/dps_backend.rs`)
- `tests/**`, `scripts/run_local_server.rs`
- `config/scripts/dps_auth_api_migrate.rs` → `config/scripts/dps_backend_migrate.rs`,
  `config/scripts/migrate.sh`
- `config/databases/main/schema.sql` (generated header comment)
- `Dockerfile`, `start.sh`, `.github/workflows/*.yml`
- `README.md`, `AGENTS.md`

**Rename scope — the script MUST NOT touch (historical / unrelated):**

- `docs/llm/plans/**` (historical plans intentionally reference the old name)
- `docs/ROADMAP.md`, `docs/TODO.md`
- `scripts/*.ts` other than the new rename script (historical one-off refactor scripts)
- `CHANGELOG.md`, `.release-please-manifest.json`, `release-please-config.json`, `railway.toml`, `loadtest.yml`
- Any string matching `DPS_AUTH_API_*`, `get_auth_api_*`, `dps-auth-session`, `DpsAuthSession*`,
  `DPS_AUTH_SESSION`

**Generated schema string** (easy to miss): `src/database/sqlite_database.rs:246` emits
`-- Generated automatically by dps-auth-api-migrate` into the schema dump, `config/databases/main/schema.sql:2`
contains it, and `src/database/main_database.rs:190,312` assert it. Rename the emitted string, update the
assertions, and **regenerate `schema.sql`** with `cargo run --bin dps-backend-migrate` so the committed file
matches the generator.

## Target architecture

```
src/
  dps_backend.rs          # DpsBackend + DpsBackendError + enhance()/routes()
  controllers/            # was orchestrators/ (same run(...) behavior)
    auth/ role/ site/ user/ server/?
  handlers/               # was graphql/resolvers/ — thin REST handlers
    auth.rs  sites.rs  users.rs  roles.rs  system.rs  mod.rs
  routes.rs               # assembles the {api_path} Router (or inlined in dps_backend.rs)
  middleware/             # request_id.rs, session.rs (unchanged, minus async-graphql deps)
  types/                  # + serde request/response DTOs, error envelope
  models/ queries/ services/ database/ utils/   # unchanged
  test_utils/             # reworked for HTTP/controller testing
lib.rs                    # pub use dps_backend::{DpsBackend, DpsBackendError};
```

Deleted: `src/graphql/**` (resolvers, types, schema), `src/handlers/graphql.rs`, and GraphQL-only test helpers
(`create_test_query_schema`, `create_test_mutation_schema`, `TestEmptyQuery`, `EmptyMutation` re-export).
`src/handlers/rest.rs` (`root_handler`/`health_handler`/`not_found_handler`) is **moved into the example host
binary** — the library no longer owns `/`, `/health`, or the fallback — and its unit tests move with it.

## REST API surface (`{api_path}` default = `/api`)

All routes are nested under `/{api_path}`. JSON in/out, camelCase fields.

### System

| Method | Path | Controller | Auth | Request | Success |
|---|---|---|---|---|---|
| GET | `/api/server-timestamp` | `ServerTimestampController` (new, thin) or `ServerService` directly | none | — | `200 {"timestamp":"<ms>"}` |

### Auth

| Method | Path | Controller | Auth | Request body | Success |
|---|---|---|---|---|---|
| POST | `/api/auth/register` | `AuthRegisterController` | none | `{username,password,passwordConfirmation}` | `201 {user,message}` + `Set-Cookie` |
| POST | `/api/auth/login` | `AuthLoginController` | none | `{username,password}` | `200 {token,user,message}` + `Set-Cookie` |
| POST | `/api/auth/logout` | (none) | none | — | `200 {message}` + expired `Set-Cookie` |
| PATCH | `/api/auth/password` | `AuthChangePasswordController` | session | `{currentPassword,newPassword,newPasswordConfirmation}` | `200 {message,updatedTs}` |
| GET | `/api/auth/me` | `AuthMeController` | session | — | `200 {user,sessionIat,sessionExp}` / `401` |

### Sites

| Method | Path | Controller | Permission | Request body | Success |
|---|---|---|---|---|---|
| GET | `/api/sites` | `GetSitesController` | none | — | `200 [site]` |
| GET | `/api/sites/{id}` | `GetSiteController` | `can_view_site_details` | — | `200 site` |
| POST | `/api/sites` | `AddSiteController` | `can_create_site` | `{slug,subdomain?,port?,protocol?,metadataJson?}` | `201 site` |
| PATCH | `/api/sites/{id}` | `UpdateSiteController` | `can_update_site` | `{slug?,subdomain?,port?,protocol?,metadataJson?}` | `200 site` |
| DELETE | `/api/sites/{id}` | `RemoveSiteController` | `can_delete_site` | — | `200 site` |

### Users

| Method | Path | Controller | Permission | Request body | Success |
|---|---|---|---|---|---|
| GET | `/api/users` | `GetUsersController` | `can_list_users` | — | `200 [user]` |
| GET | `/api/users/{id}` | `GetUserController` | `can_view_user_details` | — | `200 user` |
| PATCH | `/api/users/{id}` | `UpdateUserController` | `can_edit_user` | `{name?,roleId?,password?,passwordConfirmation?,metadataJson?}` | `200 user` |
| DELETE | `/api/users/{id}` | `DeleteUserController` | `can_delete_user` | — | `200 {success:true}` |

### Roles

| Method | Path | Controller | Permission | Request body | Success |
|---|---|---|---|---|---|
| GET | `/api/roles` | `GetRolesController` | `can_manage_roles` OR `can_edit_user_role` | — | `200 [role]` |
| GET | `/api/roles/{id}` | `GetRoleController` | `can_manage_roles` | — | `200 role` |
| GET | `/api/role-permissions` | `GetRolePermissionsController` | `can_manage_roles` | — | `200 [String]` |
| POST | `/api/roles` | `AddRoleController` | `can_manage_roles` | `{name,permissions:[String]}` | `201 role` |
| PATCH | `/api/roles/{id}` | `UpdateRoleController` | `can_manage_roles` | `{name?,permissions?}` | `200 role` |
| PUT | `/api/roles/{id}/default` | `SetDefaultRoleController` | `can_manage_roles` | — | `200 role` |
| DELETE | `/api/roles/{id}` | `RemoveRoleController` | `can_manage_roles` | — | `200 {success,id,name}` |

Notes:

- `removeSite`/`deleteUser` currently take the id in the GraphQL body; in REST it is the `{id}` path param.
- Update handlers inject the `{id}` path param into the controller input: `UpdateUserInput.id`,
  `UpdateRoleDataInput.id` (via the request DTO), and the site update input are populated from `Path(id)`, not
  from the body. `removeSite` / `deleteUser` / `setDefaultRole` pass `Path(id)` straight through to their
  controller.
- `authMe` previously returned HTTP 200 with `null` when unauthenticated. REST returns `401`.
- Business errors that GraphQL returned as HTTP 200 now map to real status codes (see below).
- `serverTimestamp` returns `{"timestamp":"<millis-as-string>"}` (the service returns a string).

### JSON response shapes (serde)

- `site`: `{id, slug, subdomain, port, protocol, metadataJson, createdTs, updatedTs}`
- `user`: `{id, uuid, name, role:{id, name, permissions:[String]}, metadataJson, createdTs, updatedTs}`
- `role`: `{id, name, isDefault, permissions:[String], createdTs, updatedTs}`
- `UserRole.id` changes from the GraphQL `ID` (string) to an integer `id`.
- error envelope: `{"error":"<message>"}`

### New HTTP types (serde DTOs)

Create under `src/types/http/` and re-export from `src/types/mod.rs`:

- Requests (`#[derive(Deserialize)] #[serde(rename_all = "camelCase")]`): `RegisterRequest`, `LoginRequest`,
  `ChangePasswordRequest`, `AddSiteRequest`, `UpdateSiteRequest`, `UpdateUserRequest`, `AddRoleRequest`,
  `UpdateRoleRequest`.
- Responses (`#[derive(Serialize)] #[serde(rename_all = "camelCase")]`): `SiteResponse`, `UserResponse`,
  `UserRoleResponse` (id `i64`), `RoleResponse`, `SiteSummaryResponse`, `UserSummaryResponse`,
  `RoleSummaryResponse`, `RegisterResponse`, `LoginResponse`, `LogoutResponse`, `MeResponse`,
  `ChangePasswordResponse`, `RemoveRoleResponse`, `TimestampResponse`, `DeleteUserResponse`,
  `RolePermissionsResponse`, `ErrorResponse`.
- Port the existing conversion logic from `src/graphql/resolvers/*Response` (`From<models::…>`) into these
  types. Reuse `src/types/user/update_user_input.rs::UpdateUserInput` (already a plain struct) for update
  handlers.
- `SessionContext` stays in `middleware/session.rs`, minus `from_context` (which depends on
  `async_graphql::Context`); handlers read it via `Extension<SessionContext>`.

### Error → HTTP status mapping

| Domain error | Status |
|---|---|
| `ValidationError`, `InvalidPermission` | `400 Bad Request` |
| `AuthenticationError`, "No valid session", invalid credentials | `401 Unauthorized` |
| `AuthorizationError` ("Forbidden") | `403 Forbidden` |
| `SiteNotFound`, `UserNotFound`, `RoleNotFound` | `404 Not Found` |
| `UsernameAlreadyExists`, `RoleNameAlreadyExists`, `SlugAlreadyExists`, `RoleInUse`, `SelfDeletion` | `409 Conflict` |
| `DatabaseError`, `PasswordHashingFailed`, `SessionError`, `AuthSessionError`, `ConfigurationError` | `500 Internal Server Error` |

Create a single `ApiError` type (in `src/handlers/error.rs`) that implements `From` for each domain error and
`IntoResponse`. Handlers return `Result<impl IntoResponse, ApiError>` and use `?` freely.

```rust
pub enum ApiError {
  Session(SessionError),
  User(UserError),
  Role(RoleError),
  Site(SiteError),
}

impl From<SessionError> for ApiError { fn from(e: SessionError) -> Self { Self::Session(e) } }
// also From<UserError>, From<RoleError>, From<SiteError>

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let (status, message) = match &self {
      ApiError::Session(e) => match e {
        SessionError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, "Authentication required"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
      },
      ApiError::User(e) => match e {
        UserError::UsernameAlreadyExists(_) | UserError::SelfDeletion => (StatusCode::CONFLICT, e_as_str),
        UserError::UserNotFound(_) => (StatusCode::NOT_FOUND, e_as_str),
        UserError::ValidationError(_) => (StatusCode::BAD_REQUEST, e_as_str),
        UserError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, e_as_str),
        UserError::AuthorizationError(_) => (StatusCode::FORBIDDEN, e_as_str),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
      },
      ApiError::Role(e) => match e {
        RoleError::RoleNotFound(_) => (StatusCode::NOT_FOUND, e_as_str),
        RoleError::RoleNameAlreadyExists(_) | RoleError::RoleInUse(_) => (StatusCode::CONFLICT, e_as_str),
        RoleError::ValidationError(_) | RoleError::InvalidPermission(_) => (StatusCode::BAD_REQUEST, e_as_str),
        RoleError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, e_as_str),
        RoleError::AuthorizationError(_) => (StatusCode::FORBIDDEN, e_as_str),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
      },
      ApiError::Site(e) => match e {
        SiteError::SiteNotFound(_) => (StatusCode::NOT_FOUND, e_as_str),
        SiteError::SlugAlreadyExists(_) => (StatusCode::CONFLICT, e_as_str),
        SiteError::ValidationError(_) => (StatusCode::BAD_REQUEST, e_as_str),
        SiteError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, e_as_str),
        SiteError::AuthorizationError(_) => (StatusCode::FORBIDDEN, e_as_str),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
      },
    };
    if status.is_server_error() {
      tracing::error!(error = ?self, "API error");
    }
    (status, Json(ErrorResponse { error: message.to_string() })).into_response()
  }
}
```

Never surface raw `DatabaseError`/internal strings to clients (log them, return `"Internal server error"`).
`login` must return `401 "Invalid credentials"` (not `500`) — map `SessionError::AuthenticationError` inside
the login handler or add a dedicated `ApiError::InvalidCredentials` variant.

## `DpsBackend` design (server enhancement)

```rust
use axum::{routing::{get, post, patch, put, delete}, Router};
use axum::middleware::from_fn;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_http::cors::CorsLayer;
use tracing::Level;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
  pub databases: crate::database::Databases,
  pub config: Arc<dps_config::DpsConfig>,
}

pub struct DpsBackend {
  pub(crate) config: Arc<dps_config::DpsConfig>,
}

impl DpsBackend {
  pub fn new(dps_config: dps_config::DpsConfig) -> Result<Self, DpsBackendError> {
    // identical validation to DpsAuthApi::new (secret present + 32 bytes)
  }

  /// Build the backend's own routes, generic over the host router's state type.
  pub async fn routes<S>(&self) -> Result<Router<S>, DpsBackendError>
  where
    S: Clone + Send + Sync + 'static,
  {
    let state = self.initialize_state().await?;         // Databases + Arc<DpsConfig>
    let api_path = format!("/{}", self.config.get_api_path());

    let api = Router::new()
      .route("/server-timestamp", get(crate::handlers::system::server_timestamp))
      .route("/auth/register", post(crate::handlers::auth::register))
      .route("/auth/login", post(crate::handlers::auth::login))
      .route("/auth/logout", post(crate::handlers::auth::logout))
      .route("/auth/password", patch(crate::handlers::auth::change_password))
      .route("/auth/me", get(crate::handlers::auth::me))
      .route("/sites", get(crate::handlers::sites::list).post(crate::handlers::sites::create))
      .route(
        "/sites/{id}",
        get(crate::handlers::sites::show)
          .patch(crate::handlers::sites::update)
          .delete(crate::handlers::sites::remove),
      )
      .route("/users", get(crate::handlers::users::list))
      .route(
        "/users/{id}",
        get(crate::handlers::users::show)
          .patch(crate::handlers::users::update)
          .delete(crate::handlers::users::remove),
      )
      .route("/roles", get(crate::handlers::roles::list).post(crate::handlers::roles::create))
      .route(
        "/roles/{id}",
        get(crate::handlers::roles::show)
          .patch(crate::handlers::roles::update)
          .delete(crate::handlers::roles::remove),
      )
      .route("/roles/{id}/default", put(crate::handlers::roles::set_default))
      .route("/role-permissions", get(crate::handlers::roles::permissions))
      // session middleware applies to every backend route
      .layer(from_fn(crate::middleware::session::create_session_middleware(
        self.config.get_auth_api_session_secret_bytes().unwrap(),
      )))
      // backend-only observability; intentionally NOT applied to host routes
      .layer(
        ServiceBuilder::new()
          .layer(from_fn(crate::middleware::request_id::request_id_middleware))
          .layer(
            TraceLayer::new_for_http()
              .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
              .on_request(DefaultOnRequest::new().level(Level::INFO))
              .on_response(DefaultOnResponse::new().level(Level::INFO)),
          )
          .layer(CorsLayer::very_permissive()),
      );

    Ok(Router::new().nest(&api_path, api).with_state(state))
  }

  /// Enhance a host-provided router with the backend routes.
  pub async fn enhance<S>(&self, router: Router<S>) -> Result<Router<S>, DpsBackendError>
  where
    S: Clone + Send + Sync + 'static,
  {
    let routes = self.routes::<S>().await?;
    Ok(router.merge(routes))
  }
}
```

Kept public methods (unchanged behavior, moved from `DpsAuthApi`): `initialize_database`,
`migrate_database`, `seed_database`, plus a new private `initialize_state` that creates main+session pools.

### Example host binary (`scripts/run_local_server.rs`)

```rust
use axum::{http::StatusCode, routing::get, Router};
use dps_backend::services::{LogShutdownStartService, WaitForShutdownSignalService};
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenvy::dotenv().ok();
  // tracing init ...

  let config = DpsConfig::new();
  let port = config.get_auth_api_port().unwrap_or(3000);
  let backend = DpsBackend::new(config)?;

  // Host owns root, health check, and fallback.
  let host = Router::new()
    .route("/", get(|| async { "OK" }))
    .route("/health", get(|| async { "OK" }))
    .fallback(not_found);

  let app = backend.enhance(host).await?; // Router<()>

  let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
  Ok(())
}

async fn not_found() -> (StatusCode, &'static str) {
  (StatusCode::NOT_FOUND, "NOT FOUND")
}

async fn shutdown_signal() {
  let signal = WaitForShutdownSignalService::run().await;
  LogShutdownStartService::run(signal);
}
```

`LogShutdownStartService` / `WaitForShutdownSignalService` **stay in the library** (re-exported from
`services`) and are *called by the host binary*; the library no longer wires them into a server.

## Implementation — single atomic change

This is **one atomic, non-phased change**. The rename script alone breaks the build; GraphQL must be deleted
before the crate compiles without `async-graphql`; and `DpsBackend` must exist before `lib.rs` can re-export
it. There is **no intermediate green build** — the tree is broken from the moment the rename runs until every
step below is done. The list is an authoring/review checklist only. Run `cargo build` + `cargo test --quiet`
once, at the end.

Ordered checklist:

1. **Project rename** — create and run `scripts/rename-dps-auth-api-to-dps-backend.ts` using the include/exclude
   scope in "Rename rules". Rename `src/dps_auth_api.rs` → `src/dps_backend.rs`; rename `src/orchestrators/` →
   `src/controllers/` and every `XxxOrchestrator` → `XxxController` (files, module decls, `pub use`s, tests);
   rename the test-macros crate/macro.
2. **Dependencies** — remove `async-graphql` and `async-graphql-axum` from `Cargo.toml`.
3. **HTTP types** — create the serde request/response DTOs (see "JSON response shapes").
4. **Handlers** — create `src/handlers/{auth,sites,users,roles,system,error}.rs` + `mod.rs` implementing the
   route table; move `root_handler`/`health_handler`/`not_found_handler` out of the library into the example
   host.
5. **`DpsBackend`** — implement `AppState`, `DpsBackend`, `DpsBackendError`, `routes<S>`, `enhance<S>`
   (`AppState` is defined here, not in the handlers); remove `start`/`bind_listener`/`build_router`/
   `create_shutdown_handler`.
6. **`lib.rs`** — declare `controllers`/`handlers`/`routes` (drop `graphql`); re-export
   `DpsBackend`/`DpsBackendError`.
7. **Delete GraphQL** — `src/graphql/**`, `src/handlers/graphql.rs`; drop the `async_graphql` import and
   `SessionContext::from_context` from `middleware/session.rs`; delete GraphQL test helpers.
8. **Tests** — rewrite `tests/*.rs` and `src/test_utils/mod.rs` per "Testing strategy"; add
   `tests/route_tests.rs`.
9. **Schema artifact** — regenerate `config/databases/main/schema.sql` after changing the emitted header string.
10. **Docs/CI/Docker** — `Dockerfile`, `start.sh`, workflows, `README.md` (full REST endpoint table).
11. **Verify** — `cargo build`; `cargo test --quiet`; `cargo clippy --allow-dirty --fix && cargo fmt`.

> AGENTS.md layering note: no new queries or services are introduced (controllers are renames of existing
> orchestrators and all query/service code is unchanged), so the usual "queries → services → orchestrators →
> resolvers" sub-task ordering collapses into the single change above.

## Testing strategy

Run the full suite once at the end (there is no intermediate green state).

- **Layer 1 (preferred): controller unit tests.** The existing orchestrator tests already call `run(...)`
  directly against `Databases` with `#[dps_backend_db_test]`; keep and extend them.
- **Layer 2 (HTTP, no server): router tests via `oneshot`.**
  - Add a `test_utils` helper `async fn create_test_app() -> Router` that builds a `DpsBackend` on a temp DB,
    runs migrations/seeds, merges into `Router::new().route("/", ...).route("/health", ...).fallback(...)`, and
    returns `Router<()>`. No socket.
  - Rewrite `tests/integration_tests.rs` (REST calls instead of GraphQL), `tests/logging_tests.rs` (REST
    routes), `tests/session_custom_functions_test.rs` and `tests/database_integration_tests.rs` (imports/names
    only). Delete `tests/get_query_support_tests.rs` (GraphQL GET) and `tests/playground_tests.rs` (GraphQL
    playground).
  - Rename `test_utils::create_test_user_via_mutation` → `create_test_user_via_api`, posting to
    `POST /api/auth/register` (or calling `AuthRegisterController` directly).
- **HTTP client**: stay with `tower::ServiceExt::oneshot` + `axum::body::to_bytes` (already available, no new
  deps). The `axum-test` crate is more ergonomic but adds a dependency — **recommended against**.
- Add `tests/route_tests.rs` asserting every route is mounted (method+path is not 404) and the
  status/content-type contract.
- Cookie behavior (Domain/Path/Secure/HttpOnly/expiry) asserted via response headers; password-logging tests
  retargeted at REST login/register. No test opens a TCP port or calls `axum::serve`.

## Dependencies

- Remove: `async-graphql`, `async-graphql-axum`.
- Add: none required. (`serde`/`serde_json`/`tower`/`axum` already present.)
- Keep: `dps-config`, `dps-auth-session`, `sqlx`, `tokio`, `tower-http`, `tracing*`, `argon2`, `bcrypt`, etc.

## Risks & mitigations

- **Rename collisions**: exhaustive Bun script + manual grep for `DPS_AUTH_API_`/`get_auth_api_` to avoid
  touching config names.
- **Router state mismatch**: `routes<S>`/`enhance<S>` are generic, so hosts with their own state still work;
  covered by a test merging into a `Router<HostState>`.
- **Behavior change (HTTP status codes / `authMe` 401)**: documented in the endpoint table; clients must adapt
  (pre-1.0).
- **`UserRole.id` type change** (string → int): documented in response shapes.
- **Route overlap panics** when merging: the host example uses only `/`, `/health`, fallback; the backend owns
  `/{api_path}/*`. If a host defines `/{api_path}/...`, merge will panic — document that embedders must not
  claim the `{api_path}` namespace.

## Open items / notes

- `serverTimestamp` controller is a thin addition for handler consistency; it can be skipped (handler calls
  `ServerService` directly) if preferred — recommend adding it.
- Whether the host example should also expose `GET {api_path}/health` is left out intentionally (host owns
  health).
- `dps-config`'s `auth_api_*` naming is a known follow-up in that repo, out of scope here.

## Files summary

**Modify**: `Cargo.toml`, `Cargo.lock`, `README.md`, `AGENTS.md`, `Dockerfile`, `start.sh`, `src/lib.rs`,
`src/dps_backend.rs` (was `src/dps_auth_api.rs`), `src/middleware/session.rs`, `src/database/sqlite_database.rs`
(schema header string), `src/database/main_database.rs` (assertions), `src/test_utils/mod.rs`,
`config/scripts/dps_backend_migrate.rs` (was `dps_auth_api_migrate.rs`), `config/scripts/migrate.sh`,
`config/databases/main/schema.sql`, all `tests/*.rs`, `.github/workflows/docker-build-test.yml`,
`crates/dps-backend-test-macros/**`, `scripts/run_local_server.rs`.

**Rename**: `src/orchestrators/` → `src/controllers/` (+ `*Orchestrator` → `*Controller`);
`crates/dps-auth-test-macros` → `crates/dps-backend-test-macros`.

**Create**: `src/handlers/{auth,sites,users,roles,system,error}.rs`, `src/routes.rs` (optional),
`src/types/http/*`, `scripts/rename-dps-auth-api-to-dps-backend.ts`.

**Delete/move**: `src/graphql/**`, `src/handlers/graphql.rs`, GraphQL test helpers,
`tests/get_query_support_tests.rs`, `tests/playground_tests.rs`; `src/handlers/rest.rs` moves into the example
host binary.
