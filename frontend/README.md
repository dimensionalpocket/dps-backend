# @dimensionalpocket/dps-backend-frontend

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

The Vue 3 frontend served by [`dps-backend`](../README.md) as an Inertia.js
monolith composable. It talks to the backend's REST API (under `/{api_path}`)
on the same origin and uses Inertia for server-routed page navigation.

Part of the [DPS ecosystem](https://github.com/dimensionalpocket/dps-readme).

Stack: Bun, Vue 3, TypeScript, Vite 7, Inertia.js, TailwindCSS 4, Pinia,
shadcn-vue (reka-ui), vue-sonner.

## Commands

- **Install**: `bun install`
- **Dev server**: `bun run dev` (serves on `http://localhost:5000`)
- **Build**: `bun run build` (writes `dist/`, including `.vite/manifest.json`)
- **Tests**: `bun test`

> This is a Bun project. Never use `npm`, `node`, `typescript`, or `tsc`.

## How it fits together

The Rust backend owns the root router. In production it serves the built
bundle from `frontend/dist` under `/build/` and renders the root HTML document
itself (`FrontendAssets::root_html`), injecting the Vite asset tags. There is no
`index.html` in this project — the server renders the shell.

Navigation is Inertia-driven: page components live in `src/Pages/`, layouts in
`src/Layouts/`, and the backend declares one route per page (see
`src/handlers/pages.rs` and `src/dps_backend.rs` in the parent crate). Pages are
code-split; Vite emits one async chunk per page.

Auth is client-side. The Inertia shell is auth-agnostic; the Vue app calls
`GET /api/auth/me` (the `HttpOnly` session cookie is scoped to `Path=/api`) and
guards pages through `useAuthGuard()`.

## Configuration

Environment variables are prefixed with `VITE_` and read via
[`@dimensionalpocket/dps-config`](https://github.com/dimensionalpocket/dps-config)
in `src/lib/config.ts`.

| Environment Variable | Description | Default Value |
|----------------------|-------------|---------------|
| `VITE_DPS_API_PATH` | Path (without leading slash) for the REST API. | `api` |

The API is served on the same origin, so only `api_path` is used.

## Dev flow and hot file

`bun run dev` starts the Vite dev server on port `5000`. When the server starts
it writes its URL to `frontend/hot` (gitignored, intentionally outside `dist/`).
The backend reads that file in development mode and injects a
`<script type="module" src="{hot}/src/main.ts">` tag into the root document. If
`frontend/hot` is absent, the backend falls back to
`DPS_BACKEND_FRONTEND_DEV_SERVER_URL` (default `http://localhost:5000`).

## Adding a page

1. Create `src/Pages/<Name>.vue`.
2. Add a route/handler in the parent crate (`src/handlers/pages.rs` and
   `src/dps_backend.rs`).
3. Update the backend `README.md` frontend page table.

## License

[MIT](LICENSE)
