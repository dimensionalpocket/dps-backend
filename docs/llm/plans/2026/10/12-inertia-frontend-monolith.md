# Inertia Frontend Monolith Composable

Date: 2026-10-12

## Goal

Turn `dps-backend` into a **monolith composable**: it serves the DPS Vue frontend from the root path
`/` (via Inertia.js) and keeps its REST API under `/{api_path}`. A host application can still merge
its own backend routes into the `{api_path}` namespace through a new `enhance_api` entry point, and
those host routes inherit the backend's session/request-id/tracing/CORS stack.

Concretely:

- `DpsBackend` owns `/` (Inertia pages), `/health`, `/{api_path}` (REST), and the root fallback.
- `DpsBackend::enhance_api(host_api)` merges host routes (paths relative to the API root) into the
  API sub-router, then returns the fully built root `Router<()>`.
- The existing `enhance(host)` direction (backend merges into host) is removed.
- The frontend (copied from `dps-auth-web` into `frontend/`) is migrated from GraphQL + vue-router
  to REST + Inertia.

## Context for a fresh implementation session

Read these before starting; the plan assumes them.

Backend (Rust, axum 0.8, MSRV 1.88):

- `src/dps_backend.rs` currently exposes `DpsBackend::new`, `routes<S>()` (returns the `/{api_path}`
  routes only), `enhance<S>(host)` (merges backend routes into the host), `databases()`, and
  `initialize_state()`. `AppState` lives here (`pub`, not yet re-exported).
- `src/test_utils/mod.rs::create_test_app()` builds a host router (`/`, `/health`, fallback) and calls
  `enhance`. Many tests call `create_test_app()`.
- Existing tests that will be affected: `src/dps_backend.rs`
  (`test_routes_server_timestamp`, `test_enhance_preserves_host_routes`, `test_enhance_adds_backend_routes`),
  `tests/integration_tests.rs` (`test_root_endpoint` asserts `/` body is `"OK"`; `test_health_endpoint`
  asserts `/health` body is `"OK"`), `tests/logging_tests.rs` (uses `/health`),
  `tests/route_tests.rs` (`test_unmounted_route_returns_fallback` asserts `GET /api/does-not-exist` is
  `404` with body `"NOT FOUND"`).
- **`/health` must keep working**: `railway.toml` (`healthcheckPath = "/health"`) and
  `.github/workflows/docker-build-test.yml` (`curl .../health`) depend on it. Since the backend now owns
  `/`, it must serve `/health` itself.
- Follow root `AGENTS.md`: 2-space indent, rustfmt, services → handlers → routes ordering, run
  `cargo clippy --allow-dirty --fix && cargo fmt` after implementation.

Frontend (Bun + Vue 3 + TS + Vite 7):

- Copied to `frontend/`. Entry `src/main.ts`; shell `src/App.vue`; views in `src/views/`; Pinia stores
  in `src/stores/`; GraphQL wrapper `src/lib/auth-wrapper.ts`; vue-router in `src/lib/router.ts`.
- Follow `frontend/AGENTS.md`: Bun only (never npm/node/tsc), `@/` alias, 2-space indent, single quotes,
  no semicolons, `<script>` before `<template>` before `<style>`, no `scoped` styles.

## Current state (post-copy)

- Data layer is **GraphQL** via `@urql/core` / `@urql/vue` (`src/lib/auth-wrapper.ts`,
  `src/lib/graphql-auth-client.ts`). The backend is now REST-only.
- Routing is **vue-router** with `meta.requiresAuth` / `meta.requiresAdmin` guards
  (`src/lib/router.ts`).
- Views render inside a single shell `src/App.vue` (navbar + `<router-view/>` + footer + toast).
- API URL is computed by `src/lib/utils/getAuthApiUrl.ts` from `VITE_DPS_*` env. It does **not** use
  DpsConfig yet.
- Stack: Bun, Vue 3, TS, Vite 7, TailwindCSS 4, shadcn-vue (reka-ui), Pinia, vue-sonner,
  `@vueuse/head`, `lucide-vue-next`, `@tanstack/vue-table`.
- Admin views use only `user.role.name`; the role CRUD helpers in `auth-wrapper.ts` are currently
  unused by any view (see Phase B2).
- Already cleaned up post-copy: removed `frontend/LICENSE`, `.roo`, `opencode.json`, `.vscode`,
  `.tool-versions`, `docs/drafts`, `docs/llm`, `docs/backend-graphql-schema.md` (kept
  `docs/roadmap.md`). Bun `1.4.2` added to the root `.tool-versions`. `frontend/AGENTS.md` deduped
  against the root `AGENTS.md`.
- Still to update in Phase B4: `frontend/README.md` and the stale GraphQL/router references in
  `frontend/AGENTS.md`.

## Locked decisions

1. **Inertia crate:** `inertia_rs` 0.4.0, `default-features = false, features = ["axum"]`. Axum-native,
   framework-neutral core (no forced session model), Inertia v3 metadata + redirect/location, MIT, lean
   deps. SSR and async prop resolvers not needed now. Rejected: `axum-inertia` (too bare),
   `inertia-omega` (alpha, unpublished, heavy defaults), `veer` (unproven).
2. **No SSR, no session-store integration.** Flash/validation via a session store is out of scope; the
   app uses a stateless encrypted cookie (`dps-auth-session`).
3. **Auth is client-side; cookie scope stays `Path=/{api_path}`.** Inertia page routes are
   auth-agnostic shells. The Vue app resolves auth by calling `GET /api/auth/me` (the cookie is sent
   because the path matches) and stores it in Pinia; guards/redirects are client-side. `HttpOnly`
   prevents JS access.
4. **Inertia is server-routed.** The root fallback renders a 404 Inertia `Error` page for non-API
   paths; unknown API paths keep the existing plain-text `"NOT FOUND"` 404.
5. **Vite is the frontend toolchain.** Vite dev server serves modules in dev; the Rust server serves
   the built bundle from disk in prod.
6. **Frontend migrates to REST + Inertia.** Remove `vue-router` and `@urql/*`; use plain `fetch` for
   `/api` and Inertia's `Link`/`router`/`usePage` for navigation.
7. **`api_path` comes from DpsConfig (Bun).** `@dimensionalpocket/dps-config` pinned to the
   `dps-backend` branch (confirmed: `getApiPath()` + `(env, envPrefix)` constructor exist). Base REST
   path is `/{api_path}` on the same origin.
8. **Pages are code-split.** Inertia resolves pages lazily (one chunk per page), fetched on demand.
9. Pre-1.0: no backwards-compatibility shims. `enhance` is removed, not deprecated.

## Architecture

Request flow after this plan:

```
GET /                   -> root router -> pages router (Inertia) -> Home shell (HTML/JSON)
GET /health             -> root router -> plain "OK" (kept for deploy/CI healthchecks)
GET /build/assets/*.js  -> root router -> ServeDir(frontend/dist)  (prod only)
GET /api/auth/me        -> root router -> nested /api router -> REST handler
GET /api/host-route     -> root router -> nested /api router -> host handler (merged via enhance_api)
GET /api/does-not-exist -> root fallback -> 404 "NOT FOUND" (plain)
GET /unknown            -> root fallback -> Inertia Error page (404)
```

The page shell carries no auth. After it loads, the Vue app calls `GET /api/auth/me` — the `Path=/api`
cookie is sent on that request — and guards routes client-side.

Router composition order (inside `DpsBackend::routes`/`enhance_api`):

1. Build backend REST `api` router (`Router<AppState>`), then `api = api.merge(host_api)`.
2. `api = api.layer(session).layer(request_id + trace + cors)` — host routes inherit all of it.
3. `api = Router::new().nest(&format!("/{api_path}"), api)`.
4. Build `pages` router (Inertia page routes), layered with `VersionLayer` only. No session middleware,
   no server-side shared props — page routes are auth-agnostic shells.
5. `root = Router::new().route("/health", get(health)).merge(pages).merge(api).fallback(not_found)`.
6. In prod, `root = root.nest_service("/build", ServeDir::new(dist_path))`.
7. `root.with_state(state)`.

### Fallback rules (verified against `axum-0.8` source)

- `nest` hoists an inner router's **custom** fallback into the outer router's fallback map under the
  prefix, and does **not** flip the outer `default_fallback` flag. So a nested `/api` router that sets
  a fallback (e.g. a host fallback) merges cleanly into the root.
- `merge` panics only when **both** sides carry a custom fallback. The backend does **not** set a
  fallback on `api`, so `host_api` is free to set its own.
- API 404s are handled by the **smart root fallback** (`not_found`), not a fallback on `api`:
  - path is `/{api_path}` or starts with `/{api_path}/` → `(404, "NOT FOUND")` (unchanged behavior);
  - otherwise → render the Inertia `Error` page with status `404`.
- The root fallback is set last on `root`; it is the only non-prefixed custom fallback there.

## Page map (backend routes ↔ frontend components)

Component names match files under `frontend/src/Pages/`. "Guard" is a client-side guard only.

| Route | Inertia component | Guard |
|---|---|---|
| `GET /` | `Home` | auth |
| `GET /login` | `Auth/Login` | none |
| `GET /register` | `Auth/Register` | none |
| `GET /login-success` | `Auth/LoginSuccess` | auth |
| `GET /password-change` | `Auth/PasswordChange` | auth |
| `GET /admin` | `Admin/Home` | admin |
| `GET /admin/users` | `Admin/Users` | admin |
| `GET /admin/sites` | `Admin/Sites` | admin |
| `GET /admin/sites/new` | `Admin/SitesNew` | admin |
| `GET /admin/sites/{id}/edit` | `Admin/SitesEdit` | admin (props: `{ id }`) |
| `GET /health` | — (plain `"OK"`) | none |
| fallback (non-API) | `Error` (404) | none |

## Dependency changes

`Cargo.toml`:

```toml
inertia_rs = { version = "0.4.0", default-features = false, features = ["axum"] }
tower-http = { version = "0.5", features = ["cors", "trace", "fs"] }
```

`frontend/package.json` (run `bun install` after):

- **Add:** `@inertiajs/vue3` (latest), `@dimensionalpocket/dps-config`
  (`github:dimensionalpocket/dps-config#dps-backend`).
- **Remove:** `vue-router`, `@urql/vue`, `@urql/exchange-retry`, `@vueuse/head`
  (`auth-wrapper.ts` also imports `@urql/core` directly; both go).
- **Keep:** `vue`, `pinia`, `tailwindcss` + `@tailwindcss/vite`, shadcn/reka-ui deps, `vue-sonner`,
  `lucide-vue-next`, `@tanstack/vue-table`, `@vueuse/core`, `inter-ui`, `class-variance-authority`,
  `clsx`, `tailwind-merge`.

---

# Part A — Backend

## Phase A1 — Frontend assets service (service)

**Why:** resolves Vite asset tags and the root HTML for dev and prod, and exposes the asset version
used by Inertia's stale-asset check.

New files: `src/frontend/mod.rs`, `src/frontend/assets.rs`; register `pub mod frontend;` in `src/lib.rs`.

### `FrontendConfig`

```rust
#[derive(Debug, Clone)]
pub struct FrontendConfig {
  pub dev_mode: bool,
  pub base: String,           // prod asset URL prefix, "/build/"
  pub dist_path: PathBuf,     // "frontend/dist"
  pub hot_file_path: PathBuf, // "frontend/hot"
  pub entry: String,          // Vite entry key, "src/main.ts"
  pub dev_server_url: String, // fallback when hot file is absent
}

impl FrontendConfig {
  pub fn from_env(dev_mode: bool) -> Self {
    Self {
      dev_mode,
      base: env_or("DPS_BACKEND_FRONTEND_BASE", "/build/"),
      dist_path: PathBuf::from(env_or("DPS_BACKEND_FRONTEND_DIST_PATH", "frontend/dist")),
      hot_file_path: PathBuf::from(env_or("DPS_BACKEND_FRONTEND_HOT_FILE_PATH", "frontend/hot")),
      entry: env_or("DPS_BACKEND_FRONTEND_ENTRY", "src/main.ts"),
      dev_server_url: env_or("DPS_BACKEND_FRONTEND_DEV_SERVER_URL", "http://localhost:5000"),
    }
  }
}
```

`dev_mode` comes from `DpsConfig::get_development_mode()`. The hot file lives at `frontend/hot` (NOT
`frontend/public/hot`) so Vite does not copy it into `dist/` during build. Dev server default is
`5000` to match the existing Vite config.

### `FrontendAssets`

Construction is **infallible** so tests and API-only deployments do not require a frontend build.

```rust
#[derive(Debug, Deserialize)]
struct ManifestChunk {
  file: String,
  #[serde(default)]
  css: Vec<String>,
}

pub struct FrontendAssets {
  config: FrontendConfig,
  manifest: Option<HashMap<String, ManifestChunk>>,
  version: String,
}

impl FrontendAssets {
  pub fn load(config: FrontendConfig) -> Self { /* prod: read frontend/dist/.vite/manifest.json */ }
  pub fn version(&self) -> String { self.version.clone() }
  pub fn asset_tags(&self) -> String { /* dev: hot file; prod: manifest */ }
  pub fn root_html(&self, data_page: &str) -> String { /* full document */ }
}
```

Behavior:

- **Dev:** read `hot_file_path` (trimmed). Tag = `{hot}/{entry}` (Vite dev base `/`). If absent, fall
  back to `dev_server_url`. Version = `"dev"`.
- **Prod:** read `.vite/manifest.json`, look up `entry`, emit `{base}{file}` script and `{base}{css}`
  links. Version = manifest build id, else entry file hash, else `"prod"`.
- **Missing manifest (prod):** log a warning, return empty tags, never panic.
- `root_html` emits `<script data-page="app" type="application/json">{data_page}</script>` and
  `<div id="app"></div>`, plus the asset tags in `<head>`.

Tests (`src/frontend/assets.rs`):

- dev with a temp hot file → tags contain hot URL + entry; version `"dev"`.
- dev without hot file → tags contain `dev_server_url`.
- prod with a temp manifest → script + css tags use `base` + hashed file; version non-empty.
- prod with missing manifest → `asset_tags()` empty, no panic.
- `root_html` contains `data-page="app"`, the JSON, and `<div id="app">`.

Run `RUST_BACKTRACE=1 cargo test frontend --quiet` before moving on.

## Phase A2 — Inertia page handlers, `/health`, fallback (handlers)

New file: `src/handlers/pages.rs`; register `pub mod pages;` in `src/handlers/mod.rs`. Add `health` to
`src/handlers/system.rs`.

Imports used by this module: `inertia_rs::axum::{InertiaError, InertiaRequest}`, `inertia_rs::Inertia`.

Shared render helper (thin; handlers stay wrappers):

```rust
fn render_page<T: Serialize>(
  state: &AppState,
  inertia: &InertiaRequest,
  component: &str,
  props: T,
) -> Response {
  let frontend = state.frontend.clone();
  inertia
    .render(
      Inertia::response(component, props),
      |ctx| Html(frontend.root_html(ctx.data_page())).into_response(),
    )
    .unwrap_or_else(|error: InertiaError| {
      tracing::error!(?error, "failed to render Inertia page");
      (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    })
}
```

Handlers (all thin; one per page in the page map). Examples:

```rust
/// `GET /admin/sites/{id}/edit`
#[instrument(skip(state, inertia))]
pub async fn admin_sites_edit(
  State(state): State<AppState>,
  Path(id): Path<i64>,
  inertia: InertiaRequest,
) -> Response {
  render_page(&state, &inertia, "Admin/SitesEdit", json!({ "id": id }))
}

/// Root fallback: API paths keep the plain-text 404; everything else renders the Inertia Error page.
pub async fn not_found(
  State(state): State<AppState>,
  uri: Uri,
  inertia: InertiaRequest,
) -> Response {
  let api_prefix = format!("/{}", state.config.get_api_path());
  let path = uri.path();
  if path == api_prefix || path.starts_with(&format!("{api_prefix}/")) {
    return (StatusCode::NOT_FOUND, "NOT FOUND").into_response();
  }
  let response = render_page(&state, &inertia, "Error", json!({ "status": 404 }));
  (StatusCode::NOT_FOUND, response).into_response()
}
```

`src/handlers/system.rs`:

```rust
/// `GET /health`
#[instrument]
pub async fn health() -> &'static str { "OK" }
```

Props note: page routes are auth-agnostic shells; auth is resolved client-side (Part B). Handlers pass
only route params (`id`).

Tests (`src/handlers/pages.rs`):

- Each route without `X-Inertia` returns `200` HTML containing `data-page="app"` and its component.
- `GET /login` with `X-Inertia: true` **and** `X-Inertia-Version: dev` returns `200` JSON containing
  `"component":"Auth/Login"`. (The version header is required: `VersionLayer` returns `409` when an
  Inertia request's version is missing or stale. In test/dev mode the version is `"dev"`.)
- `GET /admin/sites/7/edit` renders `Admin/SitesEdit` with `{"id":7}` in the page props.
- `not_found` for `/api/does-not-exist` → `404` body `"NOT FOUND"`; for `/does-not-exist` → `404`
  Inertia `Error`.

Run `RUST_BACKTRACE=1 cargo test handlers::pages --quiet` before moving on.

## Phase A3 — Router refactor: `enhance_api`, root ownership, static serving (routes)

File: `src/dps_backend.rs` (primary); `src/lib.rs`, `src/test_utils/mod.rs`,
`scripts/run_local_server.rs`, and the test files listed in "Context".

### `AppState` and re-export

Add the `frontend` field and re-export `AppState` from `src/lib.rs` so hosts can name it in
`enhance_api`:

```rust
// src/lib.rs
pub use dps_backend::{AppState, DpsBackend, DpsBackendError};
```

```rust
#[derive(Clone)]
pub struct AppState {
  pub databases: DpsBackendDatabases,
  pub config: Arc<dps_config::DpsConfig>,
  pub frontend: Arc<crate::frontend::FrontendAssets>,
}
```

### `DpsBackend`

- Add field `frontend_config: FrontendConfig` (default `FrontendConfig::from_env(config.get_development_mode())`),
  plus `with_frontend_config(self, config: FrontendConfig) -> Self`.
- `initialize_state` builds `FrontendAssets::load(self.frontend_config.clone())` and stores it in
  `AppState`.
- Extract the current REST route declarations (from today's `routes()`) into a free fn
  `backend_api_router() -> Router<AppState>` (routes only, no layers).

New/changed methods (note: `routes` is no longer generic over host state):

```rust
/// Full root router: `/health`, Inertia pages at "/", REST at "/{api_path}", root fallback.
pub async fn routes(&self) -> Result<Router, DpsBackendError>;

/// Merge host routes (paths relative to the API root, e.g. "/reports") into the backend's
/// {api_path} router, then return the full root router. Host routes inherit the backend's
/// session, request-id, tracing, and CORS layers.
pub async fn enhance_api(&self, host_api: Router<AppState>) -> Result<Router, DpsBackendError>;
```

Builder internals:

```rust
async fn build_root(&self, host_api: Router<AppState>) -> Result<Router, DpsBackendError> {
  let state = self.initialize_state().await?;
  let api_path = format!("/{}", self.config.get_api_path());
  let session_secret = self.config.get_backend_session_secret_bytes()
    .ok_or(DpsBackendError::MissingRequiredConfig { field: "backend_session_secret".into() })?;

  // 1-2. REST + host routes, then shared layers.
  let api = backend_api_router()
    .merge(host_api)
    .layer(from_fn(create_session_middleware(session_secret)))
    .layer(ServiceBuilder::new()
      .layer(from_fn(crate::middleware::request_id::request_id_middleware))
      .layer(TraceLayer::new_for_http() /* as today */)
      .layer(CorsLayer::very_permissive()));

  // 3. Nest under /{api_path}.
  let api = Router::new().nest(&api_path, api);

  // 4. Inertia pages. Auth-agnostic shells: no session middleware, no shared props.
  let frontend = state.frontend.clone();
  let pages = Router::new()
    .route("/", get(crate::handlers::pages::home))
    .route("/login", get(crate::handlers::pages::login))
    .route("/register", get(crate::handlers::pages::register))
    .route("/login-success", get(crate::handlers::pages::login_success))
    .route("/password-change", get(crate::handlers::pages::password_change))
    .route("/admin", get(crate::handlers::pages::admin_home))
    .route("/admin/users", get(crate::handlers::pages::admin_users))
    .route("/admin/sites", get(crate::handlers::pages::admin_sites))
    .route("/admin/sites/new", get(crate::handlers::pages::admin_sites_new))
    .route("/admin/sites/{id}/edit", get(crate::handlers::pages::admin_sites_edit))
    .layer(VersionLayer::dynamic({ let f = frontend.clone(); move || f.version() }));

  // 5. Health + root + fallback.
  let root = Router::new()
    .route("/health", get(crate::handlers::system::health))
    .merge(pages)
    .merge(api)
    .fallback(crate::handlers::pages::not_found);

  // 6. Static assets (prod only).
  let root = if self.frontend_config.dev_mode {
    root
  } else {
    root.nest_service("/build", ServeDir::new(&self.frontend_config.dist_path))
  };
  Ok(root.with_state(state))
}
```

Imports: `inertia_rs::axum::VersionLayer`, `tower_http::services::ServeDir`.

Remove `enhance`. Update call sites:

- `scripts/run_local_server.rs`:

  ```rust
  let backend = DpsBackend::new(config)?;
  let host_api = Router::new();                 // host contributes only API routes
  let app = backend.enhance_api(host_api).await?;
  axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
  ```

- `src/test_utils/mod.rs::create_test_app`: build via `backend.routes()` (no host router), and set a
  dev `FrontendConfig` (nonexistent hot file) via `with_frontend_config` so no frontend build is needed.

Test updates (see "Context" for current expectations):

- `src/dps_backend.rs`: remove `test_enhance_preserves_host_routes` / `test_enhance_adds_backend_routes`;
  keep/adjust `test_routes_server_timestamp` (`routes()` now returns the root router; `/api/...` still
  works). Add `enhance_api` tests below.
- `tests/integration_tests.rs`: `test_root_endpoint` now expects `200` HTML (contains `data-page="app"`),
  not body `"OK"`. `test_health_endpoint` stays (backend serves `/health` → `"OK"`).
- `tests/logging_tests.rs`: unchanged (uses `/health`).
- `tests/route_tests.rs`: unchanged (`/api/does-not-exist` still `404 "NOT FOUND"`).

New tests (`src/dps_backend.rs` and `tests/`):

- `routes()` serves `/login` (HTML), `/api/server-timestamp` (JSON), `/health` (`"OK"`), and the 404
  fallback.
- `enhance_api` host route `/reports` → `GET /api/reports` resolves to the host handler.
- Host route receives `Extension<SessionContext>` (false without cookie, true with a valid cookie).
- Duplicate exact path (`GET /auth/me`) panics on merge (documented).
- Host routes do not leak to root (e.g. `GET /reports` is 404).
- Static assets: temp `dist` dir, prod mode, `GET /build/<file>` returns the file.

Run `RUST_BACKTRACE=1 cargo test --quiet` before moving on.

---

# Part B — Frontend migration

## Phase B1 — Tooling & config

Files: `frontend/package.json`, `frontend/vite.config.ts` (renamed from `.js`), delete
`frontend/index.html`, new `frontend/.env.example`, new `frontend/src/lib/config.ts`.

1. `package.json`: apply the dependency changes above; rename the package to
   `@dimensionalpocket/dps-backend-frontend`; keep scripts (`bun --bun vite`, etc.); run `bun install`.
2. `vite.config.ts`:

   ```ts
   import { defineConfig } from 'vite'
   import vue from '@vitejs/plugin-vue'
   import tailwindcss from '@tailwindcss/vite'
   import { writeFileSync, mkdirSync } from 'node:fs'
   import path from 'node:path'

   export default defineConfig(({ command }) => ({
     base: command === 'build' ? '/build/' : '/',
     plugins: [
       vue(),
       tailwindcss(),
       {
         name: 'dps-write-hot-file',
         configureServer(server) {
           const hotFile = path.resolve(__dirname, 'hot')
           const write = (contents: string) => {
             mkdirSync(path.dirname(hotFile), { recursive: true })
             writeFileSync(hotFile, contents)
           }
           server.httpServer?.once('listening', () => {
             write(`http://localhost:${server.config.server.port}`)
           })
           server.httpServer?.once('close', () => { try { write('') } catch {} })
         },
       },
     ],
     resolve: { alias: { '@': path.resolve(__dirname, './src') } },
     build: { outDir: 'dist', manifest: true, rollupOptions: { input: 'src/main.ts' } },
     server: { host: true, port: 5000, strictPort: true, cors: true },
   }))
   ```

   Port stays `5000` (matches `DPS_BACKEND_FRONTEND_DEV_SERVER_URL`). `base` is `/` in dev (backend tags
   `{hot}/src/main.ts`) and `/build/` in prod. Hot file is `frontend/hot` (gitignored).
3. Delete `index.html` — the root document is server-rendered by `FrontendAssets::root_html`.
4. `frontend/.env.example`:

   ```
   VITE_DPS_API_PATH=api
   ```

5. `src/lib/config.ts` — DpsConfig Bun:

   ```ts
   import { DpsConfig } from '@dimensionalpocket/dps-config'

   const config = new DpsConfig(import.meta.env, 'VITE_')
   export const apiBase = `/${config.getApiPath()}`
   ```

   Only `api_path` is used (same-origin); `getBackendUrl()` is irrelevant.

Tests: `src/lib/config.test.ts` asserts `apiBase === '/api'` for `VITE_DPS_API_PATH=api` and `/v1` for
`v1`.

Toolchain: Bun `1.4.2` is pinned in the root `.tool-versions` (the frontend has no `.tool-versions`).

## Phase B2 — REST data layer + stores

**Why:** the backend is REST-only; the GraphQL client must be replaced.

New files under `frontend/src/lib/api/`:

- `client.ts`:

  ```ts
  import { apiBase } from '@/lib/config'

  export class ApiError extends Error {
    constructor(public status: number, message: string) { super(message) }
  }

  export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await fetch(`${apiBase}${path}`, {
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', ...(init.headers ?? {}) },
      ...init,
    })
    if (!response.ok) {
      let message = response.statusText
      try { const body = await response.json(); if (body?.error) message = body.error } catch {}
      throw new ApiError(response.status, message)
    }
    if (response.status === 204) return undefined as T
    return response.json() as Promise<T>
  }
  ```

  (Optionally keep the old 502-retry behavior with a small retry loop.)
- `types.ts` — REST DTOs (`Site`, `User`, `Role`, `AuthMe`, `AuthLogin`, `AuthRegister`, ...), camelCase,
  `id: number`.
- `auth.ts` — `me()`, `login()`, `register()`, `logout()`, `changePassword()` mapping to
  `GET /auth/me`, `POST /auth/login`, `POST /auth/register`, `POST /auth/logout`, `PATCH /auth/password`.
- `sites.ts` — `list`, `create`, `update(id)`, `remove(id)`.
- `users.ts` — `list`, `update(id)`, `remove(id)`.
- `roles.ts` — optional. The copied views do **not** use role CRUD (AdminUsers only reads
  `user.role.name`); add it only if a view needs it. If added: `list`, `get(id)`, `create`, `update(id)`,
  `setDefault(id)`, `remove(id)`, `permissions()`.

Delete: `src/lib/auth-wrapper.ts`, `src/lib/graphql-auth-client.ts`, `src/lib/utils/getAuthApiUrl.ts`
+ `getAuthApiUrl.test.ts`.

Update stores (`auth.ts`, `site.ts`, `user.ts`) to call the new API modules. Keep each store's public
shape (`isAuthenticated`, `isAdmin`, `canAccessAdmin`, `ensureSession`, `login`, `register`, `logout`,
`me`, `changePassword`, etc.) so views change minimally. Normalize types to REST (`Role.id: number`,
not the old `string`).

Tests: `src/lib/api/client.test.ts` (mock `fetch`) for success, `{error}` envelope, and 204.

## Phase B3 — Inertia app migration

**Why:** Inertia replaces vue-router; page navigation and auth guarding become Inertia-driven.

1. `src/main.ts` — Inertia bootstrap, keep Pinia + styles, drop `@vueuse/head`:

   ```ts
   import './style.css'
   import 'inter-ui/inter-variable.css'
   import { createApp, h } from 'vue'
   import { createPinia } from 'pinia'
   import { createInertiaApp, resolvePageComponent } from '@inertiajs/vue3'
   import AppLayout from '@/Layouts/AppLayout.vue'
   import AdminLayout from '@/Layouts/AdminLayout.vue'

   createInertiaApp({
     // Lazy (non-eager) glob: Vite emits one async chunk per page, fetched on demand.
     resolve: (name) =>
       resolvePageComponent(
         `./Pages/${name}.vue`,
         import.meta.glob('./Pages/**/*.vue'),
       ).then((page: any) => {
         page.default.layout = page.default.layout
           ?? (name.startsWith('Admin/') ? [AppLayout, AdminLayout] : AppLayout)
         return page
       }),
     setup({ el, App, props, plugin }) {
       createApp({ render: () => h(App, props) })
         .use(plugin)
         .use(createPinia())
         .mount(el)
     },
   })
   ```

   Pages are code-split per route; the Inertia server response stays a small JSON page object.
   Long-lived `/build/*` cache headers are deferred.
2. Layouts:
   - Move the shell from `src/App.vue` into `src/Layouts/AppLayout.vue` (header/footer/toaster),
     replacing `<router-view/>` with `<slot/>`. Move `useColorMode` (from `@vueuse/core`) and the
     `useTitle` call (replace with Inertia `<Head title="Account center" />`).
   - Move `src/views/admin/AdminApp.vue` into `src/Layouts/AdminLayout.vue`, replacing `<router-view/>`
     with `<slot/>` and `router-link` with Inertia `Link`. Its active-tab computation currently uses
     `useRoute().name`; derive it from `usePage().component` (e.g. `Admin/Users` → tab `admin-users`)
     or `usePage().url`.
   - Delete `src/App.vue`.
3. Move views → pages (component names must match the page map):

   | Current | New |
   |---|---|
   | `views/HomeView.vue` | `Pages/Home.vue` |
   | `views/LoginView.vue` | `Pages/Auth/Login.vue` |
   | `views/RegisterView.vue` | `Pages/Auth/Register.vue` |
   | `views/LoginSuccessView.vue` | `Pages/Auth/LoginSuccess.vue` |
   | `views/PasswordChangeView.vue` | `Pages/Auth/PasswordChange.vue` |
   | `views/admin/AdminHomeView.vue` | `Pages/Admin/Home.vue` |
   | `views/admin/AdminUsersView.vue` | `Pages/Admin/Users.vue` |
   | `views/admin/AdminSitesView.vue` | `Pages/Admin/Sites.vue` |
   | `views/admin/AdminSitesNewView.vue` | `Pages/Admin/SitesNew.vue` |
   | `views/admin/AdminSitesEditView.vue` | `Pages/Admin/SitesEdit.vue` |

   Also add `Pages/Error.vue` (404 shell; reads the `status` prop).
4. Replace vue-router APIs (all in `views/*`, `components/LoginDropdown.vue`, `lib/logout.ts`):
   - `router-link :to="{ name }"` → Inertia `<Link href="/admin/users">` (URL strings; Inertia `Link`
     has no named routes).
   - `useRouter()` → `router` from `@inertiajs/vue3` (`router.visit('/path')`).
   - `useRoute()` → `usePage()` (`page.url`, `page.props`); route params come as page props
     (`defineProps<{ id: number }>()` in `Admin/SitesEdit`).
   - Login redirect query: parse `new URL(usePage().url, location.origin).searchParams.get('redirect')`;
     default to `/login-success` when absent (matches current behavior).
   - `@vueuse/head` `useTitle` → Inertia `<Head title="..." />`.
   - `logoutWithToast` currently takes `(router, toast)`; use Inertia's `router` internally
     (`router.visit('/login')`) and drop the router arg.
5. Client-side auth guard — `src/composables/useAuthGuard.ts`:

   ```ts
   import { onMounted } from 'vue'
   import { router, usePage } from '@inertiajs/vue3'
   import { useAuthStore } from '@/stores/auth'

   export function useAuthGuard(options: { admin?: boolean } = {}) {
     const auth = useAuthStore()
     const page = usePage()
     onMounted(async () => {
       await auth.ensureSession()
       if (!auth.isAuthenticated) {
         router.visit(`/login?redirect=${encodeURIComponent(page.url)}`)
         return
       }
       if (options.admin && !auth.canAccessAdmin) router.visit('/')
     })
   }
   ```

   Call it in `Home`, `Auth/LoginSuccess`, `Auth/PasswordChange` (auth) and `Admin/*` (`{ admin: true }`).
6. Delete `src/lib/router.ts`. Keep `src/lib/logout.ts` (updated above).

Note: guards run after the shell renders (brief flash) — accepted for client-side auth.

## Phase B4 — Cleanup & frontend docs

- Obsolete files/docs already removed (see Current state). Remaining: update the stale GraphQL/router
  references in `frontend/AGENTS.md` (it still mentions `src/App.vue`, `src/views/`, Vue Router, and
  `docs/backend-graphql-schema.md`).
- Rewrite `frontend/README.md`: describe the Inertia + REST frontend, DpsConfig `api_path`, dev/build
  commands, and the dev-server/hot-file flow.
- Update `frontend/AGENTS.md`: remove GraphQL/vue-router references; document Inertia conventions
  (`Pages/`, `Layouts/`, `Link href` URL strings instead of named routes, `usePage`, `useAuthGuard`),
  REST `fetch` usage, and keep the Bun/Tailwind/shadcn guidance.
- Root `.gitignore` additions:

  ```
  frontend/node_modules/
  frontend/dist/
  frontend/hot
  ```

- Dockerfile (note, not implemented here): add a Bun build stage that runs `bun install && bun run build`
  in `frontend/` and copies `frontend/dist` into the runtime image.

---

# Part C — Documentation (root)

Final phase (after all implementation and tests pass), per root `AGENTS.md`.

Update `README.md`:

- Replace the "embeddable router / host owns root" framing with the monolith-composable model: backend
  owns `/`, `/health`, `/{api_path}`, and the fallback; host merges routes via `enhance_api`.
- Rewrite "Quickstart" and "Mounting host routes under the API path" to use `enhance_api`, noting host
  routes inherit the backend's session/request-id/tracing/CORS.
- Document that auth is client-side (`GET /api/auth/me`); the session cookie remains
  `Path=/{api_path}` and `HttpOnly`.
- Add a "Frontend" section: layout, Bun/Vite commands, dev vs prod asset resolution, and how to add a
  page (create `Pages/X.vue` + a route/handler in `src/handlers/pages.rs` + `src/dps_backend.rs`).
- Add the frontend env vars (`DPS_BACKEND_FRONTEND_DIST_PATH`, `DPS_BACKEND_FRONTEND_HOT_FILE_PATH`,
  `DPS_BACKEND_FRONTEND_ENTRY`, `DPS_BACKEND_FRONTEND_BASE`, `DPS_BACKEND_FRONTEND_DEV_SERVER_URL`).
- Update the REST endpoint table (routes stay under `{api_path}`) and add a "Frontend pages" table from
  the page map.

Update `AGENTS.md` (root) project-structure/API-setup notes for the new root/`enhance_api` ownership,
`/health`, and the `frontend/` directory.

---

## Out of scope (future plans)

- SSR (`inertia_rs` async resolvers / SSR are not yet available).
- Flash messages and validation-error bags (need a session store).
- Server-side Inertia auth / shared props and server-side page guards (would require scoping the
  session cookie to `/`; intentionally avoided).
- Long-lived cache headers for `/build/*` (hashed assets are content-addressed, so correctness holds
  without them; add `Cache-Control: public, max-age=31536000, immutable` later).
- Docker multi-stage Bun build.
- Per-page authorization beyond client-side guards.
