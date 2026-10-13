# Admin Frontend App (separate bundle under /admin)

Date: 2026-10-13

## Goal

Add a **second, independently-built frontend app** to the monolith, served under `/admin`, so that
admin-only code never ships in the main frontend bundle. The main frontend (served from `/`) stays
small; the admin bundle (which will grow with charts, tables, and other heavy tooling) is downloaded
**only when `/admin` is explicitly visited**. This directly reduces bundle size and network egress for
ordinary visits to the root frontend (relevant for Railway egress).

The existing `Admin/*` Inertia pages move from the main frontend to the new admin frontend.

## Context for a fresh implementation session

- The backend is a monolith composable (`DpsBackend`) that owns `/`, `/health`, `/{api_path}` REST,
  and the root fallback. See `docs/llm/plans/2026/10/12-inertia-frontend-monolith.md`.
- `src/frontend/assets.rs` currently resolves a **single** Vite build: `FrontendConfig` +
  `FrontendAssets` (`version()`, `asset_tags()`, `root_html(data_page)`), stored in `AppState.frontend`.
- `src/dps_backend.rs::build_root` mounts one Inertia pages router (`/`, `/login`, `/register`,
  `/login-success`, `/password-change`, `/admin`, `/admin/users`, `/admin/sites`, `/admin/sites/new`,
  `/admin/sites/{id}/edit`) layered with one `VersionLayer`, plus `ServeDir` at `/build` in prod.
- `src/handlers/pages.rs::render_page` renders every page against the single frontend and
  `handlers::pages::not_found` renders the single `Error` page for non-API 404s.
- The main app's `frontend/src/main.ts` boots Inertia with the default `id`/`data-page` of `app`.
- Inertia reads the page and mount element by `id`: `createInertiaApp({ id })` looks up
  `document.querySelector('script[data-page="${id}"][type="application/json"]')` and mounts to
  `#${id}` (default `app`). This is the hook that lets a second app coexist on other routes.
- `frontend/src/Pages` are code-split today, but `Admin/*` still belongs to the same Vite build and
  shares its dependency graph; moving to a second build is what isolates the bundle.
- Follow root `AGENTS.md` (2-space indent, rustfmt, `mise exec -- cargo ...`) and `frontend/AGENTS.md`
  (Bun only, `@/` alias, Inertia conventions).
- Always run Rust commands through `mise exec --`.

## Locked decisions

1. **Path-based, same origin.** The admin app is served at `/admin` (page routes) and
   `/admin/build/` (static assets) by the same `DpsBackend` process. No subdomain, no second server.
2. **Second Inertia app, separate Vite build.** `frontend-admin/` is its own Bun/Vite project with its
   own `Pages/`, `Layouts/`, manifest, and dev server. It boots with `createInertiaApp({ id: 'admin' })`
   and the backend renders its root document with `<div id="admin">` +
   `<script data-page="admin">`.
3. **Kept component names.** Admin pages keep their `Admin/*` Inertia component names
   (`Admin/Home`, `Admin/Users`, ...) and file paths (`frontend-admin/src/Pages/Admin/*.vue`). This
   minimizes edits to handlers and layouts and keeps logs unambiguous.
4. **Cross-app navigation is a full page load.** Main → admin, admin → main, and admin → `/login` are
   plain `<a href>` / `window.location.assign(...)`, never an Inertia visit. Inertia visits stay within
   each app. (Today an Inertia visit across apps would technically fall through as a `409` version
   mismatch → reload, but we don't rely on that.)
5. **Shared auth, duplicated client code.** Both apps authenticate via the same `/api` session cookie
   (`GET /api/auth/me`). The admin app gets its own copies of the small shared surface it needs
   (`lib/api`, `lib/config`, `stores`, `useAuthGuard`, `useToast`, used `components/ui`). A shared
   workspace package is out of scope for v1 (see [Out of scope](#out-of-scope-future-plans)).
6. **Per-app asset version.** Each app has its own manifest-derived version and its own `VersionLayer`,
   so an Inertia request for one app is never validated against the other's version.
7. **Single global dev flag.** `DPS_DEVELOPMENT_MODE` governs both apps; dev uses two Vite servers
   (main `:5000`, admin `:5001`) and two hot files; prod serves both dists from disk.

## Architecture

```
GET /                    -> root -> main pages (Inertia, data-page="app")  -> frontend/dist
GET /login               -> root -> main pages
GET /api/auth/me         -> root -> nested /api router -> REST handler
GET /admin               -> root -> admin pages (Inertia, data-page="admin") -> frontend-admin/dist
GET /admin/users         -> root -> admin pages
GET /admin/build/*.js    -> root -> ServeDir(frontend-admin/dist)   (prod only)
GET /build/*.js          -> root -> ServeDir(frontend/dist)         (prod only)
GET /api/does-not-exist  -> root fallback -> 404 "NOT FOUND"
GET /unknown             -> root fallback -> main Error page (404)
GET /admin/unknown       -> root fallback -> admin Error page (404)
```

Router composition inside `DpsBackend::build_root`:

1. `api` = REST + host routes, then session/trace/request-id/CORS layers, nested under `/{api_path}`
   (unchanged).
2. `main_pages` = `/`, `/login`, `/register`, `/login-success`, `/password-change`, layered with the
   main app's `VersionLayer`.
3. `admin_pages` = `/admin`, `/admin/users`, `/admin/sites`, `/admin/sites/new`,
   `/admin/sites/{id}/edit`, layered with the admin app's `VersionLayer`.
4. `root` = `/health` + `merge(main_pages)` + `merge(admin_pages)` + `merge(api)` + `fallback(not_found)`.
5. In prod, `root` additionally mounts `ServeDir(frontend/dist)` at `/build` and
   `ServeDir(frontend-admin/dist)` at `/admin/build`.
6. `root.with_state(state)`.

`not_found` picks the app by path: `/admin` or `/admin/*` renders the admin `Error` page; everything
else renders the main `Error` page. `/api*` keeps the plain-text `404 NOT FOUND`.

## Page map

| Route | App | Inertia component |
|---|---|---|
| `GET /` | main | `Home` |
| `GET /login` | main | `Auth/Login` |
| `GET /register` | main | `Auth/Register` |
| `GET /login-success` | main | `Auth/LoginSuccess` |
| `GET /password-change` | main | `Auth/PasswordChange` |
| `GET /admin` | admin | `Admin/Home` |
| `GET /admin/users` | admin | `Admin/Users` |
| `GET /admin/sites` | admin | `Admin/Sites` |
| `GET /admin/sites/new` | admin | `Admin/SitesNew` |
| `GET /admin/sites/{id}/edit` | admin | `Admin/SitesEdit` (props `{ id }`) |
| `GET /health` | — | plain `"OK"` |
| fallback (non-API) | by path | `Error` (404) |

## Dependency changes

**None.** The change reuses `inertia_rs` (already a dependency), axum `ServeDir`, and the existing
Vite/Bun/Tailwind/Inertia stack. The admin app will later add chart/table libraries, but that is the
point of isolating it and is out of scope here.

Directory naming: sibling `frontend-admin/` (keeps the existing `frontend/` main app untouched in
location).

---

# Part A — Backend

## Phase A1 — Frontend registry & per-app config (`src/frontend/`) (service)

**Why:** replace the single-frontend assumption with a small registry keyed by app, and teach the root
document renderer about each app's `data-page`/mount id.

Files: `src/frontend/mod.rs`, `src/frontend/assets.rs` (and optionally new `src/frontend/registry.rs`).

### `FrontendApp`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontendApp {
  Main,
  Admin,
}

impl FrontendApp {
  /// Value used for both `<script data-page="...">` and the mount `<div id="...">`.
  pub fn data_page_id(self) -> &'static str {
    match self {
      FrontendApp::Main => "app",
      FrontendApp::Admin => "admin",
    }
  }
}
```

### `FrontendConfig`

Add `app: FrontendApp`. Values are hardcoded per app (no environment variables); embedders/tests
override them with `DpsBackend::with_frontend_config` / `with_admin_frontend_config`. Replace the
current `from_env` with a `main` constructor and add an `admin` constructor:

```rust
#[derive(Debug, Clone)]
pub struct FrontendConfig {
  pub app: FrontendApp,
  pub dev_mode: bool,
  pub base: String,           // prod asset URL prefix
  pub dist_path: PathBuf,
  pub hot_file_path: PathBuf,
  pub entry: String,
  pub dev_server_url: String,
}

impl FrontendConfig {
  pub fn main(dev_mode: bool) -> Self {
    Self {
      app: FrontendApp::Main,
      dev_mode,
      base: "/build/".to_string(),
      dist_path: PathBuf::from("frontend/dist"),
      hot_file_path: PathBuf::from("frontend/hot"),
      entry: "src/main.ts".to_string(),
      dev_server_url: "http://localhost:5000".to_string(),
    }
  }

  pub fn admin(dev_mode: bool) -> Self {
    Self {
      app: FrontendApp::Admin,
      dev_mode,
      base: "/admin/build/".to_string(),
      dist_path: PathBuf::from("frontend-admin/dist"),
      hot_file_path: PathBuf::from("frontend-admin/hot"),
      entry: "src/main.ts".to_string(),
      dev_server_url: "http://localhost:5001".to_string(),
    }
  }
}
```

### `FrontendAssets::root_html`

Rename the parameter and use the app id:

```rust
pub fn root_html(&self, page_json: &str) -> String {
  let id = self.config.app.data_page_id();
  format!(
    concat!(
      "<!doctype html>\n",
      "<html lang=\"en\">\n",
      "  <head>\n",
      "    <meta charset=\"UTF-8\" />\n",
      "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n",
      "    {tags}\n",
      "  </head>\n",
      "  <body>\n",
      "    <script data-page=\"{id}\" type=\"application/json\">{page_json}</script>\n",
      "    <div id=\"{id}\"></div>\n",
      "  </body>\n",
      "</html>\n"
    ),
    tags = self.asset_tags(),
    id = id,
    page_json = page_json,
  )
}
```

Existing `version()` / `asset_tags()` stay unchanged (dev uses `hot/{entry}`, prod uses `{base}{file}`).

### `FrontendRegistry`

```rust
pub struct FrontendRegistry {
  apps: HashMap<FrontendApp, FrontendAssets>,
}

impl FrontendRegistry {
  pub fn load(configs: impl IntoIterator<Item = FrontendConfig>) -> Self {
    let apps = configs
      .into_iter()
      .map(|config| (config.app, FrontendAssets::load(config)))
      .collect();
    Self { apps }
  }

  pub fn get(&self, app: FrontendApp) -> &FrontendAssets {
    self.apps.get(&app).expect("frontend app not configured")
  }
}
```

Export `FrontendApp`, `FrontendConfig`, `FrontendAssets`, `FrontendRegistry` from `src/frontend/mod.rs`.

Tests (`src/frontend/assets.rs`), add to the existing suite:

- `root_html` for `FrontendApp::Admin` contains `data-page="admin"` and `<div id="admin">`.
- `root_html` for `FrontendApp::Main` contains `data-page="app"` and `<div id="app">`.
- `FrontendRegistry::get` returns the right app's version/tags for each config.

Run `RUST_BACKTRACE=1 mise exec -- cargo test frontend --quiet` before moving on.

## Phase A2 — Pages handlers: app-aware rendering & admin fallback (handlers)

File: `src/handlers/pages.rs`.

- Add `app: FrontendApp` to `render_page` and look up the frontend from the registry:

```rust
fn render_page<T: Serialize>(
  state: &AppState,
  inertia: &InertiaRequest,
  app: FrontendApp,
  component: &str,
  props: T,
) -> Response {
  let frontend = state.frontend.get(app);
  inertia
    .render(Inertia::response(component, props), |ctx| {
      Html(frontend.root_html(ctx.data_page())).into_response()
    })
    .unwrap_or_else(|error: InertiaError| {
      tracing::error!(?error, "failed to render Inertia page");
      (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    })
}
```

- Main handlers pass `FrontendApp::Main`; the five admin handlers pass `FrontendApp::Admin`.
- `not_found` selects the app by path:

```rust
let is_admin = path == "/admin" || path.starts_with("/admin/");
let app = if is_admin { FrontendApp::Admin } else { FrontendApp::Main };
let response = render_page(&state, &inertia, app, "Error", json!({ "status": 404 }));
```

Tests (extend the existing `handlers::pages` suite; needs A3 wiring to run end-to-end):

- `GET /admin` (no `X-Inertia`) returns `200` HTML containing `data-page="admin"` and `Admin/Home`.
- `GET /admin/unknown` (no `X-Inertia`) returns `404` HTML containing `data-page="admin"` and `Error`.
- `GET /does-not-exist` still returns `404` HTML with `data-page="app"`.
- `GET /api/does-not-exist` still returns `404` `"NOT FOUND"`.

## Phase A3 — Router wiring: two apps, per-app version, dual static mount (routes)

File: `src/dps_backend.rs` (primary); `src/test_utils/mod.rs`.

### `AppState` / `DpsBackend`

```rust
#[derive(Clone)]
pub struct AppState {
  pub databases: DpsBackendDatabases,
  pub config: Arc<dps_config::DpsConfig>,
  pub frontend: Arc<FrontendRegistry>,
}
```

`DpsBackend` holds both configs and builds the registry in `initialize_state`:

```rust
pub struct DpsBackend {
  pub(crate) config: Arc<dps_config::DpsConfig>,
  databases: OnceCell<DpsBackendDatabases>,
  main_frontend_config: FrontendConfig,
  admin_frontend_config: FrontendConfig,
}

impl DpsBackend {
  // new(): main_frontend_config = FrontendConfig::main(dev); admin_frontend_config = FrontendConfig::admin(dev)
  pub fn with_frontend_config(mut self, config: FrontendConfig) -> Self { self.main_frontend_config = config; self }
  pub fn with_admin_frontend_config(mut self, config: FrontendConfig) -> Self { self.admin_frontend_config = config; self }

  async fn initialize_state(&self) -> Result<AppState, DpsBackendError> {
    Ok(AppState {
      databases: self.databases().await?.clone(),
      config: self.config.clone(),
      frontend: Arc::new(FrontendRegistry::load([
        self.main_frontend_config.clone(),
        self.admin_frontend_config.clone(),
      ])),
    })
  }
}
```

### `build_root`

```rust
let frontends = state.frontend.clone();

let main_pages = Router::new()
  .route("/", get(crate::handlers::pages::home))
  .route("/login", get(crate::handlers::pages::login))
  .route("/register", get(crate::handlers::pages::register))
  .route("/login-success", get(crate::handlers::pages::login_success))
  .route("/password-change", get(crate::handlers::pages::password_change))
  .layer(VersionLayer::dynamic({
    let f = frontends.clone();
    move || f.get(FrontendApp::Main).version()
  }));

let admin_pages = Router::new()
  .route("/admin", get(crate::handlers::pages::admin_home))
  .route("/admin/users", get(crate::handlers::pages::admin_users))
  .route("/admin/sites", get(crate::handlers::pages::admin_sites))
  .route("/admin/sites/new", get(crate::handlers::pages::admin_sites_new))
  .route("/admin/sites/{id}/edit", get(crate::handlers::pages::admin_sites_edit))
  .layer(VersionLayer::dynamic({
    let f = frontends.clone();
    move || f.get(FrontendApp::Admin).version()
  }));

let root = Router::new()
  .route("/health", get(crate::handlers::system::health))
  .merge(main_pages)
  .merge(admin_pages)
  .merge(api)
  .fallback(crate::handlers::pages::not_found);

let root = if self.main_frontend_config.dev_mode {
  root
} else {
  root
    .nest_service("/build", ServeDir::new(&self.main_frontend_config.dist_path))
    .nest_service("/admin/build", ServeDir::new(&self.admin_frontend_config.dist_path))
};

Ok(root.with_state(state))
```

Imports: `crate::frontend::FrontendApp`. `ServeDir` and `VersionLayer` are already imported.

`src/test_utils/mod.rs::create_test_app`: build both dev `FrontendConfig`s (nonexistent hot files,
dist paths `frontend/dist` and `frontend-admin/dist`) via `with_frontend_config` +
`with_admin_frontend_config` so no frontend build is required.

Run `RUST_BACKTRACE=1 mise exec -- cargo test --quiet` before moving on.

## Phase A4 — Backend tests

Extend `src/dps_backend.rs` and, where appropriate, `tests/`.

- `routes()` serves `/login` with `data-page="app"` and `/admin` with `data-page="admin"`.
- Main pages no longer serve admin routes on the main app: `/admin` returns the **admin** root document
  (not the main one) — assert the `data-page` id.
- `enhance_api` host routes are unaffected (`/api/...` still works).
- Prod static mount: temp `frontend-admin/dist` dir + prod config → `GET /admin/build/<file>` returns
  the file.
- Per-app `VersionLayer`: in prod with distinct manifests, an `X-Inertia` request to `/admin` carrying
  the **main** version returns `409` with `X-Inertia-Location` (and vice versa); carrying the matching
  version returns `200`.
- `not_found` for `/admin/unknown` renders the admin `Error` page; `/unknown` renders the main `Error`.

## Part B — Frontend: main app changes

File: `frontend/`.

1. `src/main.ts`: the `layout` callback no longer needs `AdminLayout` (admin pages are gone). Keep
   `AppLayout` for all remaining pages.
2. Delete `src/Pages/Admin/*` (moved to the admin app) and `src/Layouts/AdminLayout.vue`.
3. `src/components/LoginDropdown.vue`: the "Administration" menu item becomes a **full navigation** to
   the admin app:

   ```ts
   function handleGoToAdmin() {
     window.location.assign('/admin')
   }
   ```

   (`link`/`signup` stay as in-app Inertia `Link`s.)
4. No other main-app behavior changes; the main bundle loses the admin pages and layout.

## Part C — Frontend: admin app (new `frontend-admin/`)

### Phase C1 — Scaffold & tooling

New files under `frontend-admin/`:

- `package.json` — name `@dimensionalpocket/dps-backend-admin-frontend`, same dependency set as
  `frontend/package.json` (vue, `@inertiajs/vue3`, pinia, tailwind, shadcn/reka-ui, vue-sonner,
  lucide-vue-next, `@dimensionalpocket/dps-config`, etc.). Scripts mirror the main app.
- `vite.config.ts`:

  ```ts
  import { defineConfig } from 'vite'
  import vue from '@vitejs/plugin-vue'
  import tailwindcss from '@tailwindcss/vite'
  import { writeFileSync, mkdirSync } from 'node:fs'
  import path from 'node:path'

  export default defineConfig(({ command }) => ({
    base: command === 'build' ? '/admin/build/' : '/',
    plugins: [
      vue(),
      tailwindcss(),
      {
        name: 'dps-admin-write-hot-file',
        configureServer(server) {
          const hotFile = path.resolve(__dirname, 'hot')
          const write = (contents: string) => {
            mkdirSync(path.dirname(hotFile), { recursive: true })
            writeFileSync(hotFile, contents)
          }
          server.httpServer?.once('listening', () => write(`http://localhost:${server.config.server.port}`))
          server.httpServer?.once('close', () => { try { write('') } catch {} })
        },
      },
    ],
    resolve: { alias: { '@': path.resolve(__dirname, './src') } },
    build: { outDir: 'dist', manifest: true, rollupOptions: { input: 'src/main.ts' } },
    server: { host: true, port: 5001, strictPort: true, cors: true },
  }))
  ```

  Port `5001` (must match the hardcoded `FrontendConfig::admin` dev-server URL); `base` is `/` in dev
  and `/admin/build/` in prod; hot file is `frontend-admin/hot`.
- `tsconfig.json`, `vue-shim.d.ts`, `components.json`, `src/style.css` — copy/adjust from `frontend/`.
- `.env.example`: `VITE_DPS_API_PATH=api`.

### Phase C2 — Bootstrap, layout, shared code

- `src/main.ts`:

  ```ts
  createInertiaApp({
    id: 'admin',
    resolve: (name) => {
      const page = pages[`./Pages/${name}.vue`]
      if (!page) throw new Error(`Unknown admin page: ${name}`)
      return page() as Promise<{ default: any }>
    },
    layout: () => AdminLayout,
    setup({ el, App, props, plugin }) {
      createApp({ render: () => h(App, props) }).use(plugin).use(createPinia()).mount(el)
    },
  })
  ```

- `src/Layouts/AdminLayout.vue`: a self-contained admin shell — header with a **plain**
  `<a href="/">Back to site</a>`, title, logout control; the existing tab bar (active tab derived from
  `usePage().component` as today); `<slot/>`; footer; `Toaster`. No dependency on the main app's
  `AppLayout`/`LoginDropdown`.
- Copy the shared client surface the admin pages need from `frontend/src/`:
  - `lib/api/*` (`client.ts`, `types.ts`, `auth.ts`, `sites.ts`, `users.ts`)
  - `lib/config.ts` (+ `config.test.ts`), `lib/logout.ts`, `lib/utils.ts`
  - `composables/useAuthGuard.ts`, `composables/useToast.ts`
  - `stores/auth.ts`, `stores/user.ts`, `stores/site.ts`
  - the shadcn `components/ui/*` used by the admin pages (button, card, dialog, field, input, label,
    separator, spinner, table, tabs, sonner)
  - `src/style.css` + `inter-ui` import
- `useAuthGuard` in the admin app performs a **full navigation** to the main app's login, because
  `/login` belongs to the other app:

  ```ts
  // admin app variant
  window.location.assign(`/login?redirect=${encodeURIComponent(page.url)}`)
  ```

  (Add an admin-local variant, or parameterize the shared composable with a `crossApp` flag.)

### Phase C3 — Move admin pages

Move (git mv semantics) `frontend/src/Pages/Admin/*.vue` → `frontend-admin/src/Pages/Admin/*.vue`.
Adjust imports to the admin app's copies (`@/lib/api/...`, `@/stores/...`, `@/composables/...`).
Add `frontend-admin/src/Pages/Error.vue` (reads the `status` prop), mirroring the main app's page.

Component names/paths are unchanged (`Admin/Home`, `Admin/Users`, `Admin/Sites`, `Admin/SitesNew`,
`Admin/SitesEdit`, `Error`), so backend handler component strings do not change.

Run `mise exec -- bun install && mise exec -- bun run build` in `frontend-admin/` and confirm
`frontend-admin/dist/.vite/manifest.json` contains the `src/main.ts` entry.

## Part D — Tooling & docs

### Phase D1 — Dev/build scripts

- `.gitignore` (root): add
  ```
  frontend-admin/node_modules/
  frontend-admin/dist/
  frontend-admin/hot
  ```
- `dev.sh`: start three processes via `mise exec --`:
  - `( cd frontend && mise exec -- bun run dev )` on `:5000`
  - `( cd frontend-admin && mise exec -- bun run dev )` on `:5001`
  - `mise exec -- cargo run --bin run_local_server`
  Keep the existing `setsid` + signal handling; track a third PID and signal it too.
- `preview.sh`: build **both** frontends (`frontend` and `frontend-admin`), verify both manifests, then
  run the backend in prod mode. Keep `DPS_DEVELOPMENT_MODE=N`.
- `Dockerfile`: add a second Bun stage and copy both bundles:
  ```dockerfile
  FROM oven/bun:1.4.2 AS admin-frontend-builder
  WORKDIR /app
  ARG VITE_DPS_API_PATH=api
  ENV VITE_DPS_API_PATH=${VITE_DPS_API_PATH}
  COPY frontend-admin/package.json frontend-admin/bun.lock ./
  RUN bun install --frozen-lockfile
  COPY frontend-admin/ ./
  RUN bun run build
  # ... in the final stage:
  COPY --from=admin-frontend-builder /app/dist /app/frontend-admin/dist
  ```
- `.dockerignore`: add `frontend-admin/node_modules/`, `frontend-admin/dist/`, `frontend-admin/hot`.

### Phase D2 — Docs

- Root `README.md`:
  - Split the "Frontend pages" table into "Main frontend pages" and "Admin frontend pages" (with the
    app column), per the page map above.
  - Document the two-app model: separate bundles, `/admin` + `/admin/build`, cross-app full navigation,
    shared `/api` auth.
  - Note that per-app asset config (base, dist path, hot file, entry, dev-server URL) is hardcoded, not
    env-driven.
  - Update the "Frontend" section (dev: two Vite servers `:5000`/`:5001`; prod: two static mounts).
- Root `AGENTS.md`: note the two frontend apps (`frontend/`, `frontend-admin/`) and that `/admin` is
  owned by the admin app with cross-app links as full navigations.
- `frontend/AGENTS.md` and a new `frontend-admin/AGENTS.md` (or extend the shared guidance): document
  the admin app's `id: 'admin'`, `Pages/Admin/`, and cross-app navigation rule.

---

## Verification checklist

- `mise exec -- cargo test --quiet` (backend unit + integration).
- `mise exec -- cargo clippy --allow-dirty --fix && mise exec -- cargo fmt`.
- `cd frontend && mise exec -- bun test && mise exec -- bun run build`.
- `cd frontend-admin && mise exec -- bun test && mise exec -- bun run build`.
- `./dev.sh`: `/` loads the main app, `/admin` loads the admin app (distinct `data-page`), the admin
  bundle is fetched only on `/admin`, and Ctrl-C stops all three processes.
- `./preview.sh`: `/` injects `/build/...`, `/admin` injects `/admin/build/...`, both asset sets return
  `200`, and `/admin/unknown` renders the admin `Error` page.
- Confirm the main app bundle does **not** contain admin page chunks (check `frontend/dist/.vite/manifest.json`).

## Out of scope (future plans)

- Extracting `lib/api`, `stores`, `composables`, and `components/ui` into a shared Bun workspace package
  (`frontend-shared/`) consumed by both apps; **recommended as a follow-up** once the admin app's shared
  surface stabilizes, to remove duplication.
- Server-side admin authorization beyond the client-side `useAuthGuard({ admin: true })` (would need
  server-side Inertia auth / shared props and a root-scoped session cookie).
- SSR for either app.
- Long-lived cache headers for `/build/*` and `/admin/build/*` (hashed assets are content-addressed, so
  correctness holds without them).
- Subdomain-based (host-routed) admin app.
- Adding the actual admin chart/table libraries (unovis, etc.) — the reason for the split, but a
  separate plan.
