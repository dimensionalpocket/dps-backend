use crate::database::{DpsBackendDatabases, MainDatabase, SessionDatabase};
use axum::{
  middleware::from_fn,
  routing::{get, patch, post, put},
  Router,
};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tower::ServiceBuilder;
use tower_http::{
  cors::CorsLayer,
  trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

/// Shared state passed to every backend route.
#[derive(Clone)]
pub struct AppState {
  pub databases: DpsBackendDatabases,
  pub config: Arc<dps_config::DpsConfig>,
}

/// Embeddable backend that enhances a host-provided `axum::Router` with the
/// DPS backend routes.
///
/// The library never binds a socket or starts a server; the host owns
/// `axum::serve` and graceful shutdown. The backend only contributes the
/// `{api_path}`-prefixed routes and their middleware.
pub struct DpsBackend {
  pub(crate) config: Arc<dps_config::DpsConfig>,
  databases: OnceCell<DpsBackendDatabases>,
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

    Ok(DpsBackend {
      config: Arc::new(dps_config),
      databases: OnceCell::new(),
    })
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

  /// Build the backend's own routes, generic over the host router's state type.
  ///
  /// The returned router is nested under `/{api_path}` and owns its session,
  /// request-id, tracing, and CORS middleware. It does not bind a port.
  pub async fn routes<S>(&self) -> Result<Router<S>, DpsBackendError>
  where
    S: Clone + Send + Sync + 'static,
  {
    let state = self.initialize_state().await?;
    let api_path = format!("/{}", self.config.get_api_path());

    let session_secret = self.config.get_backend_session_secret_bytes().ok_or(
      DpsBackendError::MissingRequiredConfig {
        field: "backend_session_secret".to_string(),
      },
    )?;

    let api = Router::new()
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
      // Session middleware applies to every backend route.
      .layer(from_fn(
        crate::middleware::session::create_session_middleware(session_secret),
      ))
      // Backend-only observability; intentionally not applied to host routes.
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

    Ok(Router::new().nest(&api_path, api).with_state(state))
  }

  /// Enhance a host-provided router with the backend routes.
  ///
  /// The host is responsible for binding and serving the returned router. It
  /// must not define routes under the backend's `{api_path}` namespace, since
  /// merging overlapping routes panics.
  pub async fn enhance<S>(&self, router: Router<S>) -> Result<Router<S>, DpsBackendError>
  where
    S: Clone + Send + Sync + 'static,
  {
    let routes = self.routes::<S>().await?;
    Ok(router.merge(routes))
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
    })
  }
}

#[cfg(test)]
mod tests {
  use dps_backend_test_macros::dps_backend_db_test;

  use super::*;
  use axum::{
    body::Body,
    http::{Request, StatusCode},
  };
  use tempfile::NamedTempFile;
  use tower::ServiceExt;

  fn create_test_backend(db_path: &str) -> DpsBackend {
    let mut config = dps_config::DpsConfig::new();
    config.set_backend_session_secret(Some("a".repeat(32).as_str()));
    config.set_backend_sqlite_main_file_path(db_path);
    config.set_api_path("api");
    DpsBackend::new(config).unwrap()
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

  #[dps_backend_db_test]
  async fn test_enhance_preserves_host_routes() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let host = Router::new()
      .route("/", get(|| async { "OK" }))
      .route("/health", get(|| async { "OK" }));
    let app = backend.enhance(host).await.unwrap();

    let response = app
      .clone()
      .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
      .oneshot(
        Request::builder()
          .uri("/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
  }

  #[dps_backend_db_test]
  async fn test_enhance_adds_backend_routes() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let backend = create_test_backend(db_path);
    let app = backend.enhance(Router::new()).await.unwrap();

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
  }
}
