# DpsBackend Owns `axum::serve` and Graceful Shutdown

Date: 2026-10-13

## Goal

Let `DpsBackend` optionally own the server lifecycle (`axum::serve` + graceful shutdown) so an
embedding host can go from "build router" to "running server" with one call, instead of hand-wiring a
`TcpListener`, `axum::serve`, and the shutdown signal that the backend already ships.

The keywords are **optionally** and **additively**:

- `routes()` / `enhance_api()` keep returning a plain `Router` and keep working exactly as today, so
  in-process `tower::ServiceExt::oneshot` tests (and hosts that want to own the listener, terminate
  TLS themselves, bind a Unix socket, or use a different server) are unaffected.
- A new `DpsBackend::serve*` family is added on top. It is opt-in sugar; there is no new requirement.

This reverses a documented architectural decision ("`DpsBackend` never binds a socket"), so the plan
also updates the rustdoc, `README.md`, and `AGENTS.md` to describe the new optional capability.

## Context for a fresh implementation session

- `src/dps_backend.rs`
  - `AppState` (line 21) is the shared state; `DpsBackend` (line 33) holds `config`, a lazy
    `databases` `OnceCell`, and `frontend_config`.
  - `build_root` (line 190) returns `Router<()>` (state already applied via `root.with_state(state)`),
    aliased as `Router`.
  - `routes()` (line 176) and `enhance_api()` (line 186) are the two public builders.
  - `DpsBackendError` (line 40) already has a `NetworkError(String)` variant.
- The shutdown services already exist in the backend and are only *wired* by the host today:
  - `WaitForShutdownSignalService::run() -> &'static str` (`src/services/shutdown/wait_for_shutdown_signal.rs`)
  - `LogShutdownStartService::run(signal)` (`src/services/shutdown/log_shutdown_start.rs`)
  - Both re-exported from `src/services/mod.rs` (lines 32-33).
- `scripts/run_local_server.rs` (lines 52-68) is the reference host. It calls `enhance_api`, binds
  `0.0.0.0:{port}` using `config.get_backend_port().unwrap_or(3000)`, calls `axum::serve`, and wires
  `shutdown_signal()` from the two shutdown services. This is exactly the boilerplate `serve()` removes.
- `dps-config` (branch `dps-backend`) exposes `get_backend_port() -> Option<u16>`, already populated
  from `DPS_BACKEND_PORT`; there is **no host getter**. Bind host is therefore hardcoded to `0.0.0.0`
  for the convenience method; callers needing another host use the listener variant.
- **Port resolution must be internal**, with the fallback chain `DPS_BACKEND_PORT` -> `PORT` ->
  `3000`. `DPS_BACKEND_PORT` comes from `get_backend_port()`; `PORT` is a generic (not Railway-only)
  environment variable and must be read by the backend, not by the host. As of today nothing reads
  `PORT` (only `scripts/run_local_server.rs` uses `config.get_backend_port().unwrap_or(3000)`).
- `dps-config` does **not** read `PORT`, and updating `DpsConfig` is out of scope for this plan, so the
  `PORT` fallback lives in `dps-backend`.
- Dependencies: `axum = "0.8"` and `tokio = { features = ["full"] }` are already present. The only new
  dependency is the dev-only `serial_test` (see Design decision 8).
- Follow root `AGENTS.md` (2-space indent, rustfmt, `mise exec -- cargo ...`, no comments unless asked).

## Design decisions

1. **Keep the pure builders.** `routes()` and `enhance_api()` signatures and behavior are unchanged.
   All existing tests (`src/dps_backend.rs` tests module, `src/test_utils/mod.rs:351`) keep passing.
2. **`serve` takes the already-built `Router`.** This matches the user's mental model ("host calls
   `enhance_api`, then calls a method that runs the server") and keeps router construction separate from
   process lifecycle. `serve` does the binding using the internal port resolution (decision 7).
3. **Three levels of control, same primitive.**
   - `serve(app)` — bind `0.0.0.0:<port>` (`DPS_BACKEND_PORT` -> `PORT` -> `3000`) + default
     SIGINT/SIGTERM shutdown. The 90% case; replaces the boilerplate in `run_local_server.rs`.
   - `serve_listener(listener, app)` — caller owns the bind (Unix socket, `127.0.0.1`, TLS front proxy,
     port `0`, socket options) but the backend still owns `axum::serve` + default shutdown.
   - `serve_with_shutdown(listener, app, signal)` — caller also supplies the shutdown future (drain
     background tasks, coordinate with a `CancellationToken`, etc.).
   The first two delegate to the third. No stored/boxed future state is needed on `DpsBackend`, so the
   struct is untouched.
4. **`serve*` take `&self`.** No ownership change is needed since shutdown is passed as a parameter;
   hosts may still use `backend` (e.g. `databases()`) after calling `serve`.
5. **Errors map to `DpsBackendError::NetworkError`.** Both the bind and `axum::serve` failures become
   `NetworkError(e.to_string())`.
6. **`serve_with_shutdown` bound is `F: Future<Output = ()> + Send + 'static`**, matching what
   `axum::serve(...).with_graceful_shutdown(...)` requires.
7. **Port resolution is internal** via a private `resolve_port(&self) -> u16` helper with the chain
   `get_backend_port()` (`DPS_BACKEND_PORT`) -> `PORT` env -> `3000`. `PORT` is read at serve time, is
   intentionally host-agnostic (not Railway-specific), and is never surfaced as a `DpsConfig` property.
   `PORT` parsing is best-effort: a missing or non-`u16` value falls through to `3000`.
8. **Add `serial_test` as a dev-dependency.** The `PORT` fallback requires mutating a process-wide env
   var in tests; `AGENTS.md` ("Testing") prescribes `serial_test` for such tests.

## API

Add to `impl DpsBackend` in `src/dps_backend.rs` (place after `enhance_api`, before `build_root`), plus
a private free function for the default signal and the imports it needs:

```rust
use std::future::Future;
use tokio::net::TcpListener;
use tracing::info;

/// Default graceful-shutdown future: resolves on SIGINT/SIGTERM and logs the
/// reason via the backend's own shutdown services.
async fn default_shutdown_signal() {
  let signal = crate::services::WaitForShutdownSignalService::run().await;
  crate::services::LogShutdownStartService::run(signal);
}
```

```rust
impl DpsBackend {
  /// Resolves the port to bind, in order of precedence:
  /// `DPS_BACKEND_PORT` (via config) -> `PORT` env var -> `3000`.
  fn resolve_port(&self) -> u16 {
    self
      .config
      .get_backend_port()
      .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()))
      .unwrap_or(3000)
  }

  // ... serve / serve_listener / serve_with_shutdown ...
}
```

```rust
impl DpsBackend {
  /// Bind the router to `0.0.0.0:<port>` and serve it with graceful shutdown on
  /// SIGINT/SIGTERM. The port is resolved from `DPS_BACKEND_PORT`, then the
  /// generic `PORT` environment variable, then `3000`.
  ///
  /// Convenience over [`DpsBackend::serve_listener`] for the common case. Use
  /// `routes()` / `enhance_api()` and `serve_listener` when you need to control
  /// the bind address, TLS termination, or the socket type.
  ///
  /// # Errors
  ///
  /// Returns [`DpsBackendError::NetworkError`] if the bind or the server loop fails.
  pub async fn serve(&self, app: Router) -> Result<(), DpsBackendError> {
    let port = self.resolve_port();
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
      .await
      .map_err(|e| DpsBackendError::NetworkError(e.to_string()))?;
    info!("Server running on http://{addr}");
    self.serve_listener(listener, app).await
  }

  /// Serve a router on a caller-provided listener with the default
  /// SIGINT/SIGTERM graceful shutdown.
  ///
  /// # Errors
  ///
  /// Returns [`DpsBackendError::NetworkError`] if the server loop fails.
  pub async fn serve_listener(
    &self,
    listener: TcpListener,
    app: Router,
  ) -> Result<(), DpsBackendError> {
    self.serve_with_shutdown(listener, app, default_shutdown_signal()).await
  }

  /// Serve a router on a caller-provided listener with a caller-provided
  /// shutdown future.
  ///
  /// The future resolves when the server should stop accepting connections and
  /// wait for in-flight requests to finish.
  ///
  /// # Errors
  ///
  /// Returns [`DpsBackendError::NetworkError`] if the server loop fails.
  pub async fn serve_with_shutdown<F>(
    &self,
    listener: TcpListener,
    app: Router,
    signal: F,
  ) -> Result<(), DpsBackendError>
  where
    F: Future<Output = ()> + Send + 'static,
  {
    axum::serve(listener, app)
      .with_graceful_shutdown(signal)
      .await
      .map_err(|e| DpsBackendError::NetworkError(e.to_string()))
  }
}
```

Update the existing doc comments that claim the library never serves:

- `DpsBackend` struct doc (`src/dps_backend.rs:27-32`): replace "The library never binds a socket or
  starts a server; the host owns `axum::serve` and graceful shutdown." with wording that the backend
  *can* serve via `serve*`, and that `routes()` / `enhance_api()` remain available for hosts that own
  the listener.
- `routes()` doc (`src/dps_backend.rs:171-175`): replace "the host calls `axum::serve`" with a note that
  the returned router can be passed to `serve*` or served by the host directly.

## Ergonomics

**Before** (`scripts/run_local_server.rs`, ~16 lines of boilerplate):

```rust
use axum::Router;
use dps_backend::services::{LogShutdownStartService, WaitForShutdownSignalService};
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let config = DpsConfig::new();
  let port = config.get_backend_port().unwrap_or(3000);
  let backend = DpsBackend::new(config)?;

  let host_api = Router::new();
  let app = backend.enhance_api(host_api).await?;

  let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
  println!("Server running on http://0.0.0.0:{port}");
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  Ok(())
}

async fn shutdown_signal() {
  let signal = WaitForShutdownSignalService::run().await;
  LogShutdownStartService::run(signal);
}
```

**After** (host, default bind + default shutdown):

```rust
use axum::Router;
use dps_backend::DpsBackend;
use dps_config::DpsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let config = DpsConfig::new();
  let backend = DpsBackend::new(config)?;

  let app = backend.enhance_api(Router::new()).await?;
  backend.serve(app).await?;

  Ok(())
}
```

**After** (host owns the listener — e.g. loopback only, or fronted by TLS):

```rust
use axum::Router;
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let backend = DpsBackend::new(DpsConfig::new())?;
  let app = backend.enhance_api(Router::new()).await?;

  let listener = TcpListener::bind("127.0.0.1:3000").await?;
  backend.serve_listener(listener, app).await?;

  Ok(())
}
```

**After** (host owns the shutdown too — drain background work before exiting):

```rust
let (tx, rx) = tokio::sync::oneshot::channel::<()>();
tokio::spawn(async move {
  let _ = dps_backend::services::WaitForShutdownSignalService::run().await;
  let _ = tx.send(());
});

backend
  .serve_with_shutdown(listener, app, async move {
    let _ = rx.await;
    drain_background_tasks().await;
  })
  .await?;
```

## Non-goals

- No changes to `routes()` / `enhance_api()` behavior or signatures.
- No TLS, Unix-socket, or `SO_REUSEPORT` support in the crate; those hosts use `serve_listener` with
  their own listener (or keep owning `axum::serve` via `routes()`).
- No new `dps-config` property (no `get_backend_host()`); bind host is fixed to `0.0.0.0` in `serve`.
- No REST routes/handlers change, so the `README.md` REST endpoint table is untouched.

## Phases

Each phase must compile and pass its tests before the next begins. Run everything via `mise exec --`.

### Phase 1 — `serve*` methods + tests (`src/dps_backend.rs`)

Add `serial_test = "3.0"` to `[dev-dependencies]` in `Cargo.toml` first (required for the `PORT`
env tests; see Design decision 8).

1. Add imports `std::future::Future`, `tokio::net::TcpListener`, `tracing::info`.
2. Add the private `default_shutdown_signal()` free function.
3. Add the private `resolve_port(&self) -> u16` helper (`DPS_BACKEND_PORT` -> `PORT` -> `3000`).
4. Add the three `serve*` methods inside `impl DpsBackend`; `serve` calls `self.resolve_port()`.
5. Update the `DpsBackend` struct doc and the `routes()` doc.
6. Add tests to the existing `#[cfg(test)] mod tests`:

   - `test_resolve_port` (pure, no env): config port wins over `PORT`; with no config port, `PORT`
     applies; with neither, `3000`. Because `create_test_backend` sets the port, build a bare
     `DpsConfig` here. Use `#[serial]` for cases that set/remove the `PORT` env var, and always
     `std::env::remove_var("PORT")` at the end. Do **not** mutate `DPS_BACKEND_PORT` (that is read by
     `DpsConfig::new()`; setting the config directly is enough).
   - `test_serve_with_shutdown_stops_on_signal` — build a backend, `migrate`/`seed`, `routes()`, bind a
     listener on `127.0.0.1:0`, spawn `serve_with_shutdown(listener, app, signal)` with a
     `tokio::sync::oneshot` receiver as the signal; capture `listener.local_addr()` **before** spawning
     if you also connect. Send on the oneshot, `await` the join handle, assert `Ok(())`.
   - `test_serve_listener_serves_requests` — bind on `127.0.0.1:0`, grab `local_addr()`, spawn the
     server with a controllable shutdown (oneshot) so the test can stop it, then issue a raw
     `tokio::net::TcpStream` HTTP/1.1 request for `GET /api/server-timestamp` and assert the response
     line contains `200`. Keep to a single server per test.
   - `test_serve_with_shutdown_custom_future` — signal future that resolves immediately; assert the call
     returns `Ok(())`.

   Note: `serve` (config-port variant) is not unit-tested because it binds a fixed port; it is exercised
   by the example binary/manual run. `serve_listener` and `serve_with_shutdown` cover the shared logic,
   and `test_resolve_port` covers the fallback chain.

   Verify: `RUST_BACKTRACE=1 mise exec -- cargo test --quiet dps_backend`.

### Phase 2 — Simplify the example host (`scripts/run_local_server.rs`)

1. Replace lines 52-68 (router build + listener + `axum::serve` + `shutdown_signal`) with:

   ```rust
   let app = backend.enhance_api(Router::new()).await?;
   backend.serve(app).await?;
   ```

2. Remove now-unused imports (`tokio::net::TcpListener`, the two shutdown services) and the `port`
   variable/`println!`. The backend now logs the bound address via `info!("Server running on http://{addr}")`.
   Keep the existing `DPS_*` env-var dump.
3. Delete the local `shutdown_signal()` helper.

   Verify: `mise exec -- cargo build --bin run_local_server`.

### Phase 3 — Documentation

1. `AGENTS.md` → "API Setup": rewrite the two bullets that state `DpsBackend` never binds a socket and
   the host owns `axum::serve`, to describe `serve*` as an optional convenience while `routes()` /
   `enhance_api()` remain for hosts that own the listener.
2. `README.md`:
   - Overview (lines 9-16): replace "The library **never binds a socket**" with the new optional model.
   - Features (lines 18-27): add a bullet for optional `axum::serve` + graceful shutdown via `serve()`.
   - Quickstart (lines 135-158), Full example (lines 167-192), Accessing the databases (lines 205-230),
     and Mounting host routes (lines 248-278): replace the manual listener + `axum::serve` blocks with
     `backend.serve(app).await?`, and mention `serve_listener` / `serve_with_shutdown` for custom hosts.
   - Local Development (lines 363-365): drop the "owns `axum::serve` and handles graceful shutdown"
     sentence; note the backend now provides it.
   - Configuration (lines 320-338): document the internal port fallback chain for `serve()`:
     `DPS_BACKEND_PORT` -> `PORT` -> `3000`, and note `PORT` is a generic (non-Railway-specific) env var.
   - No changes to the REST endpoint table.

   Verify: `mise exec -- cargo clippy --allow-dirty --fix && mise exec -- cargo fmt`, then
   `mise exec -- cargo test --quiet`.

## Resolved by review

- `serve` binds internally with host hardcoded to `0.0.0.0` (no `DpsConfig` host getter; changing the
  host would require `dps-config` work that is out of scope).
- Port resolution is internal with the chain `DPS_BACKEND_PORT` -> `PORT` -> `3000`; `PORT` is a generic
  environment variable, not Railway-only, and is never surfaced through `DpsConfig`.

## Amendment (2026-10-13): bind port vs public port

The original port chain conflated two different concepts. Superseded by the `dps-config` plan
`2026/10/13-backend-bind-port.md`:

- The **bind** port (internal, what `serve` listens on) is now owned by `DpsConfig` and resolved there as
  `DPS_BACKEND_BIND_PORT` -> `PORT` -> `3000` via `DpsConfig::get_backend_bind_port()`. `DpsBackend`
  reads that getter; its local `resolve_port` and the four `resolve_port` tests were removed, along with
  the `serial_test` dev-dependency.
- The **public** `DPS_BACKEND_PORT` is unchanged and only feeds generated URLs; it no longer influences
  the bind port.
