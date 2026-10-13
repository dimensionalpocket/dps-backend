use crate::database::{DpsBackendDatabases, MainDatabase, SessionDatabase};
use crate::frontend::{FrontendAssets, FrontendConfig};
use axum::{
  middleware::from_fn,
  routing::{get, patch, post, put},
  Router,
};
use inertia_rs::axum::VersionLayer;
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tower::ServiceBuilder;
use tower_http::{
  cors::CorsLayer,
  services::ServeDir,
  trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};

/// Shared state passed to every backend route.
#[derive(Clone)]
pub struct AppState {
  pub databases: DpsBackendDatabases,
  pub config: Arc<dps_config::DpsConfig>,
  pub frontend: Arc<FrontendAssets>,
}

/// Embeddable backend that enhances a host-provided `axum::Router` with the
/// DPS backend routes.
///
/// The backend can optionally own `axum::serve` and graceful shutdown via the
/// `serve*` methods. When the host needs full control over the listener (TLS,
/// Unix sockets, socket options), use `routes()` / `enhance_api()` to obtain the
/// root router and serve it directly.
pub struct DpsBackend {
  pub(crate) config: Arc<dps_config::DpsConfig>,
  databases: OnceCell<DpsBackendDatabases>,
  frontend_config: FrontendConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DpsBackendError {
  InvalidSecretLength { actual: usize, expected: usize },
  MissingRequiredConfig { field: String },
  DatabaseError(String),
  NetworkError(String),
  ConfigurationError(String),
}

impl std::fmt::Display for DpsBackendError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DpsBackendError::InvalidSecretLength { actual, expected } => {
        write!(
          f,
          "Invalid secret length: got {actual} bytes, expected {expected}"
        )
      }
      DpsBackendError::MissingRequiredConfig { field } => {
        write!(f, "Missing required configuration: {field}")
      }
      DpsBackendError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
      DpsBackendError::NetworkError(msg) => write!(f, "Network error: {msg}"),
      DpsBackendError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
    }
  }
}

impl std::error::Error for DpsBackendError {}

impl DpsBackend {
  /// Create a new `DpsBackend` instance from `DpsConfig`.
  ///
  /// Validates the configuration and returns an error if required fields are
  /// missing or invalid.
  ///
  /// # Errors
  ///
  /// Returns `DpsBackendError::MissingRequiredConfig` if session_secret is not set.
  /// Returns `DpsBackendError::InvalidSecretLength` if session_secret is not exactly 32 bytes.
  ///
  /// # Example
  ///
  /// ```rust
  /// use dps_config::DpsConfig;
  /// use dps_backend::DpsBackend;
  ///
  /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut config = DpsConfig::new();
  /// config.set_backend_session_secret(Some("a".repeat(32).as_str()));
  /// let backend = DpsBackend::new(config)?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn new(dps_config: dps_config::DpsConfig) -> Result<Self, DpsBackendError> {
    let session_secret = dps_config.get_backend_session_secret_bytes().ok_or(
      DpsBackendError::MissingRequiredConfig {
        field: "backend_session_secret".to_string(),
      },
    )?;

    if session_secret.len() != 32 {
      return Err(DpsBackendError::InvalidSecretLength {
        actual: session_secret.len(),
        expected: 32,
      });
    }

    let frontend_config = FrontendConfig::main(dps_config.get_development_mode());

    Ok(DpsBackend {
      config: Arc::new(dps_config),
      databases: OnceCell::new(),
      frontend_config,
    })
  }

  /// Override the frontend asset configuration.
  ///
  /// Useful for tests and API-only deployments that do not ship a frontend
  /// build.
  pub fn with_frontend_config(mut self, config: FrontendConfig) -> Self {
    self.frontend_config = config;
    self
  }

  /// Returns the shared [`DpsBackendDatabases`] instance used by the backend.
  ///
  /// The instance is created lazily on first use and cached, so the host and
  /// the embedded router always share the exact same connection pools. Hosts
  /// can borrow the returned handle to run their own queries (including data
  /// modifications) against the main and session databases.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use dps_backend::DpsBackend;
  /// use dps_config::DpsConfig;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let config = DpsConfig::new();
  /// let backend = DpsBackend::new(config)?;
  /// let databases = backend.databases().await?;
  ///
  /// let mut conn = databases.main().acquire().await?;
  /// sqlx::query("SELECT 1").execute(&mut *conn).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn databases(&self) -> Result<&DpsBackendDatabases, DpsBackendError> {
    self
      .databases
      .get_or_try_init(|| async {
        let main_db = MainDatabase::new_with_pool_size(
          &self.config.get_backend_sqlite_main_file_path(),
          Some(self.config.get_backend_sqlite_main_pool_size() as u32),
        )
        .await
        .map_err(|e| DpsBackendError::DatabaseError(e.to_string()))?;

        let session_db = SessionDatabase::new_with_pool_size(
          &self.config.get_backend_sqlite_session_file_path(),
          Some(self.config.get_backend_sqlite_session_pool_size() as u32),
        )
        .await
        .map_err(|e| DpsBackendError::DatabaseError(e.to_string()))?;

        Ok(DpsBackendDatabases::new(main_db.pool, session_db.pool))
      })
      .await
  }

  /// Build the full root router.
  ///
  /// The backend owns `/health`, the Inertia page routes at `/`, the REST API
  /// under `/{api_path}`, and the root fallback. The returned router is not
  /// bound to a port; pass it to a `serve*` method or serve it yourself.
  pub async fn routes(&self) -> Result<Router, DpsBackendError> {
    self.build_root(Router::new()).await
  }

  /// Merge host routes (paths relative to the API root, e.g. `/reports`) into
  /// the backend's `{api_path}` router, then return the full root router.
  ///
  /// Host routes inherit the backend's session, request-id, tracing, and CORS
  /// layers. The host must not redefine an exact backend method + path, which
  /// panics on merge.
  pub async fn enhance_api(&self, host_api: Router<AppState>) -> Result<Router, DpsBackendError> {
    self.build_root(host_api).await
  }

  /// Bind the router to `0.0.0.0:<bind port>` and serve it with graceful
  /// shutdown on SIGINT/SIGTERM.
  ///
  /// The bind port comes from [`dps_config::DpsConfig::get_backend_bind_port`]
  /// (`DPS_BACKEND_BIND_PORT` -> `PORT` -> `3000`) and is the *internal* port the
  /// process listens on. It is distinct from the *public* `DPS_BACKEND_PORT`,
  /// which is only appended to generated URLs.
  ///
  /// Convenience over [`DpsBackend::serve_listener`] for the common case. Use
  /// `routes()` / `enhance_api()` and `serve_listener` when you need to control
  /// the bind address, TLS termination, or the socket type.
  ///
  /// # Errors
  ///
  /// Returns [`DpsBackendError::NetworkError`] if the bind or the server loop fails.
  pub async fn serve(&self, app: Router) -> Result<(), DpsBackendError> {
    let port = self.config.get_backend_bind_port();
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
    self
      .serve_with_shutdown(listener, app, default_shutdown_signal())
      .await
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

  async fn build_root(&self, host_api: Router<AppState>) -> Result<Router, DpsBackendError> {
    let state = self.initialize_state().await?;
    let api_path = format!("/{}", self.config.get_api_path());

    let session_secret = self.config.get_backend_session_secret_bytes().ok_or(
      DpsBackendError::MissingRequiredConfig {
        field: "backend_session_secret".to_string(),
      },
    )?;

    // REST + host routes, then shared layers.
    let api = backend_api_router()
      .merge(host_api)
      .layer(from_fn(
        crate::middleware::session::create_session_middleware(session_secret),
      ))
      .layer(
        ServiceBuilder::new()
          .layer(from_fn(
            crate::middleware::request_id::request_id_middleware,
          ))
          .layer(
            TraceLayer::new_for_http()
              .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
              .on_request(DefaultOnRequest::new().level(Level::INFO))
              .on_response(DefaultOnResponse::new().level(Level::INFO)),
          )
          .layer(CorsLayer::very_permissive()),
      );

    let api = Router::new().nest(&api_path, api);

    // Inertia pages. Auth-agnostic shells: no session middleware, no shared props.
    let frontend = state.frontend.clone();
    let pages = Router::new()
      .route("/", get(crate::handlers::pages::home))
      .route("/login", get(crate::handlers::pages::login))
      .route("/register", get(crate::handlers::pages::register))
      .route("/login-success", get(crate::handlers::pages::login_success))
      .route(
        "/password-change",
        get(crate::handlers::pages::password_change),
      )
      .route("/admin", get(crate::handlers::pages::admin_home))
      .route("/admin/users", get(crate::handlers::pages::admin_users))
      .route("/admin/sites", get(crate::handlers::pages::admin_sites))
      .route(
        "/admin/sites/new",
        get(crate::handlers::pages::admin_sites_new),
      )
      .route(
        "/admin/sites/{id}/edit",
        get(crate::handlers::pages::admin_sites_edit),
      )
      .layer(VersionLayer::dynamic({
        let frontend = frontend.clone();
        move || frontend.version()
      }));

    let root = Router::new()
      .route("/health", get(crate::handlers::system::health))
      .merge(pages)
      .merge(api)
      .fallback(crate::handlers::pages::not_found);

    let root = if self.frontend_config.dev_mode {
      root
    } else {
      root.nest_service("/build", ServeDir::new(&self.frontend_config.dist_path))
    };

    Ok(root.with_state(state))
  }

  /// Returns a [`MainDatabase`] handle backed by the shared main connection
  /// pool used by the backend (no migrations or seeds are run).
  ///
  /// This method is public for test usage.
  pub async fn initialize_database(&self) -> Result<MainDatabase, DpsBackendError> {
    let databases = self.databases().await?;
    Ok(MainDatabase {
      pool: databases.main().clone(),
    })
  }

  /// Run database migrations only.
  pub async fn migrate_database(&self) -> Result<(), DpsBackendError> {
    let database = self.initialize_database().await?;
    database
      .migrate()
      .await
      .map_err(|e| DpsBackendError::DatabaseError(e.to_string()))
  }

  /// Run database seeds only.
  pub async fn seed_database(&self) -> Result<(), DpsBackendError> {
    let database = self.initialize_database().await?;
    database
      .seed()
      .await
      .map_err(|e| DpsBackendError::DatabaseError(e.to_string()))
  }

  async fn initialize_state(&self) -> Result<AppState, DpsBackendError> {
    Ok(AppState {
      databases: self.databases().await?.clone(),
      config: self.config.clone(),
      frontend: Arc::new(FrontendAssets::load(self.frontend_config.clone())),
    })
  }
}

/// Default graceful-shutdown future: resolves on SIGINT/SIGTERM and logs the
/// reason via the backend's own shutdown services.
async fn default_shutdown_signal() {
  let signal = crate::services::WaitForShutdownSignalService::run().await;
  crate::services::LogShutdownStartService::run(signal);
}

/// Backend REST routes, relative to the `{api_path}` root (declarations only;
/// layers are applied by [`DpsBackend::build_root`]).
fn backend_api_router() -> Router<AppState> {
  Router::new()
    .route(
      "/server-timestamp",
      get(crate::handlers::system::server_timestamp),
    )
    .route("/auth/register", post(crate::handlers::auth::register))
    .route("/auth/login", post(crate::handlers::auth::login))
    .route("/auth/logout", post(crate::handlers::auth::logout))
    .route(
      "/auth/password",
      patch(crate::handlers::auth::change_password),
    )
    .route("/auth/me", get(crate::handlers::auth::me))
    .route(
      "/sites",
      get(crate::handlers::sites::list).post(crate::handlers::sites::create),
    )
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
    .route(
      "/roles",
      get(crate::handlers::roles::list).post(crate::handlers::roles::create),
    )
    .route(
      "/roles/{id}",
      get(crate::handlers::roles::show)
        .patch(crate::handlers::roles::update)
        .delete(crate::handlers::roles::remove),
    )
    .route(
      "/roles/{id}/default",
      put(crate::handlers::roles::set_default),
    )
    .route(
      "/role-permissions",
      get(crate::handlers::roles::permissions),
    )
}

#[cfg(test)]
mod tests {
  use dps_backend_test_macros::dps_backend_db_test;

  use super::*;
  use crate::middleware::session::SessionContext;
  use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
  };
  use std::fs;
  use std::path::PathBuf;
  use tempfile::{NamedTempFile, TempDir};
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::sync::oneshot;
  use tower::ServiceExt;

  fn dev_frontend_config() -> FrontendConfig {
    FrontendConfig {
      dev_mode: true,
      base: "/build/".to_string(),
      dist_path: PathBuf::from("frontend/dist"),
      hot_file_path: PathBuf::from("frontend/does-not-exist-hot"),
      entry: "src/main.ts".to_string(),
      dev_server_url: "http://localhost:5000".to_string(),
    }
  }

  fn create_test_backend(db_path: &str) -> DpsBackend {
    let mut config = dps_config::DpsConfig::new();
    config.set_backend_session_secret(Some("a".repeat(32).as_str()));
    config.set_backend_sqlite_main_file_path(db_path);
    config.set_api_path("api");
    config.set_development_mode(true);
    DpsBackend::new(config)
      .unwrap()
      .with_frontend_config(dev_frontend_config())
  }

  #[test]
  fn test_new_missing_secret() {
    let config = dps_config::DpsConfig::new();
    let result = DpsBackend::new(config);
    assert!(matches!(
      result,
      Err(DpsBackendError::MissingRequiredConfig { .. })
    ));
  }

  #[test]
  fn test_new_invalid_secret_length() {
    let mut config = dps_config::DpsConfig::new();
    config.set_backend_session_secret(Some("short"));
    let result = DpsBackend::new(config);
    assert!(matches!(
      result,
      Err(DpsBackendError::InvalidSecretLength { .. })
    ));
  }

  #[dps_backend_db_test]
  async fn test_initialize_database_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);

    assert!(backend.initialize_database().await.is_ok());
  }

  #[dps_backend_db_test]
  async fn test_initialize_database_invalid_path() {
    let backend = create_test_backend("/invalid/path/that/does/not/exist/test.db");

    assert!(matches!(
      backend.initialize_database().await,
      Err(DpsBackendError::DatabaseError(_))
    ));
  }

  #[dps_backend_db_test]
  async fn test_migrate_and_seed_database() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);

    assert!(backend.migrate_database().await.is_ok());
    assert!(backend.seed_database().await.is_ok());
  }

  #[dps_backend_db_test]
  async fn test_databases_returns_cached_instance() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);

    let first = backend.databases().await.unwrap();
    let second = backend.databases().await.unwrap();

    assert!(std::ptr::eq(first, second));
  }

  #[dps_backend_db_test]
  async fn test_databases_shared_with_backend() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);

    // A write through the host-facing handle...
    let databases = backend.databases().await.unwrap();
    sqlx::query("CREATE TABLE host_writes (id INTEGER PRIMARY KEY)")
      .execute(databases.main())
      .await
      .unwrap();

    // ...is visible through the backend's own main pool.
    let main_db = backend.initialize_database().await.unwrap();
    let table_count: i64 = sqlx::query_scalar(
      "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'host_writes'",
    )
    .fetch_one(&main_db.pool)
    .await
    .unwrap();

    assert_eq!(table_count, 1);
  }

  #[dps_backend_db_test]
  async fn test_routes_server_timestamp() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app: Router = backend.routes().await.unwrap();

    let response = app
      .oneshot(
        Request::builder()
          .uri("/api/server-timestamp")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("timestamp"));
  }

  async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
  }

  #[dps_backend_db_test]
  async fn test_routes_serves_pages_api_health_and_fallback() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app: Router = backend.routes().await.unwrap();

    // Inertia page shell.
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .uri("/login")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_string(response).await.contains("data-page=\"app\""));

    // REST JSON.
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .uri("/api/server-timestamp")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_string(response).await.contains("timestamp"));

    // Health check.
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .uri("/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_string(response).await, "OK");

    // API fallback keeps plain text 404.
    let response = app
      .oneshot(
        Request::builder()
          .uri("/api/does-not-exist")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_string(response).await, "NOT FOUND");
  }

  async fn host_reports(Extension(session): Extension<SessionContext>) -> &'static str {
    if session.authenticated() {
      "AUTHED"
    } else {
      "ANON"
    }
  }

  #[dps_backend_db_test]
  async fn test_enhance_api_mounts_host_routes_and_inherits_session() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    backend.migrate_database().await.unwrap();
    backend.seed_database().await.unwrap();

    let host_api = Router::new().route("/reports", get(host_reports));
    let app = backend.enhance_api(host_api).await.unwrap();

    // Anonymous request: session context present but not authenticated.
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .uri("/api/reports")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_string(response).await, "ANON");

    // Register to obtain a valid session cookie.
    let body = serde_json::json!({
      "username": "host_route_user",
      "password": "password123",
      "passwordConfirmation": "password123",
    });
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .method("POST")
          .uri("/api/auth/register")
          .header("content-type", "application/json")
          .body(Body::from(body.to_string()))
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let cookie = response
      .headers()
      .get("set-cookie")
      .unwrap()
      .to_str()
      .unwrap()
      .to_string();

    // Authenticated request: host route sees the session cookie.
    let response = app
      .clone()
      .oneshot(
        Request::builder()
          .uri("/api/reports")
          .header("cookie", cookie)
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_string(response).await, "AUTHED");

    // Host routes do not leak to the root.
    let response = app
      .oneshot(
        Request::builder()
          .uri("/reports")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[dps_backend_db_test]
  #[should_panic]
  async fn test_enhance_api_duplicate_path_panics() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let host_api = Router::new().route("/auth/me", get(|| async { "duplicate" }));

    let _ = backend.enhance_api(host_api).await;
  }

  #[dps_backend_db_test]
  async fn test_static_assets_served_in_prod_mode() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let dist = TempDir::new().unwrap();
    fs::write(dist.path().join("app.js"), "console.log('hi')").unwrap();

    let frontend_config = FrontendConfig {
      dev_mode: false,
      base: "/build/".to_string(),
      dist_path: dist.path().to_path_buf(),
      hot_file_path: PathBuf::from("frontend/does-not-exist-hot"),
      entry: "src/main.ts".to_string(),
      dev_server_url: "http://localhost:5000".to_string(),
    };

    let backend = create_test_backend(db_path).with_frontend_config(frontend_config);
    let app = backend.routes().await.unwrap();

    let response = app
      .oneshot(
        Request::builder()
          .uri("/build/app.js")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_string(response).await, "console.log('hi')");
  }

  #[dps_backend_db_test]
  async fn test_serve_with_shutdown_stops_on_signal() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app = backend.routes().await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (tx, rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
      backend
        .serve_with_shutdown(listener, app, async move {
          let _ = rx.await;
        })
        .await
    });

    tx.send(()).unwrap();
    let result = server.await.unwrap();
    assert!(result.is_ok());
  }

  #[dps_backend_db_test]
  async fn test_serve_with_shutdown_custom_future() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app = backend.routes().await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let result = backend
      .serve_with_shutdown(listener, app, std::future::ready(()))
      .await;

    assert!(result.is_ok());
  }

  #[dps_backend_db_test]
  async fn test_serve_listener_serves_requests() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app = backend.routes().await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move { backend.serve_listener(listener, app).await });

    let mut response = String::new();
    for _ in 0..50 {
      match tokio::net::TcpStream::connect(addr).await {
        Ok(mut stream) => {
          let request =
            "GET /api/server-timestamp HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
          stream.write_all(request.as_bytes()).await.unwrap();
          stream.read_to_string(&mut response).await.unwrap();
          break;
        }
        Err(_) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
      }
    }

    assert!(response.contains("200"), "response was: {response}");

    server.abort();
  }
}
