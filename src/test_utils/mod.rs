/*
 * Shared test utilities for creating test data.
 *
 * This module provides centralized functions for creating test users, roles, and other entities.
 * All helpers use existing Query objects to ensure consistency with application behavior.
 *
 * Do NOT create local create_test_* functions in test modules - use these shared utilities instead.
 */

#[cfg(any(test, feature = "test-utils"))]
use crate::database::{DpsBackendDatabases, MainDatabase, SessionDatabase};
use crate::models::{Role, User};
use crate::queries::roles::{CreateRoleData, CreateRoleQuery, GetDefaultRoleQuery};
use crate::queries::users::{CreateUserData, CreateUserQuery};
use crate::services::GeneratePasswordHashService;
use sqlx::SqliteConnection;
use tempfile::NamedTempFile;
use uuid::Uuid;

use dps_config::DpsConfig;

/// Create a test DpsConfig for use in controller and service tests
pub fn create_test_dps_config() -> DpsConfig {
  let mut config = DpsConfig::new();
  config.set_auth_api_port(Some(3000));
  config.set_auth_api_sqlite_main_file_path(":memory:");
  config.set_auth_api_sqlite_session_file_path(":memory:");
  config.set_auth_api_session_secret(Some("a".repeat(32).as_str()));
  config.set_domain("test.com");
  config.set_api_path("api");
  config.set_auth_api_insecure_cookie(true);
  config.set_development_mode(true);
  config.set_auth_api_sqlite_main_pool_size(Some(1));
  config.set_auth_api_sqlite_session_pool_size(Some(1));
  config.set_auth_api_session_ttl_seconds(Some(3600));
  config
}

pub async fn create_test_databases() -> (DpsBackendDatabases, NamedTempFile, NamedTempFile) {
  create_test_databases_with_config(true).await
}

pub async fn create_test_databases_with_pool_size(
  pool_size: u32,
) -> (DpsBackendDatabases, NamedTempFile, NamedTempFile) {
  create_test_databases_with_config_and_pool_size(true, pool_size).await
}

pub async fn create_test_databases_with_config(
  configure_sqlite: bool,
) -> (DpsBackendDatabases, NamedTempFile, NamedTempFile) {
  create_test_databases_with_config_and_pool_size(configure_sqlite, 1).await
}

pub async fn create_test_databases_with_config_and_pool_size(
  configure_sqlite: bool,
  pool_size: u32,
) -> (DpsBackendDatabases, NamedTempFile, NamedTempFile) {
  let main_temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let main_sqlite_file_path = main_temp_file.path().display().to_string();

  let session_temp_file = NamedTempFile::new().expect("Failed to create temp file");
  let session_sqlite_file_path = session_temp_file.path().display().to_string();

  let database = MainDatabase::new_with_pool_size(&main_sqlite_file_path, Some(pool_size))
    .await
    .expect("Failed to create test database");
  let main_pool = database.pool;

  // Optionally configure SQLite settings
  if configure_sqlite {
    MainDatabase::configure_sqlite(&main_pool)
      .await
      .expect("Failed to configure SQLite");
  }

  // Run migrations
  let database = MainDatabase {
    pool: main_pool.clone(),
  };
  database.migrate().await.expect("Failed to run migrations");

  let session_database = SessionDatabase::new_with_pool_size(&session_sqlite_file_path, Some(1))
    .await
    .expect("Failed to create session test database");
  let session_pool = session_database.pool;

  let databases = DpsBackendDatabases::new(main_pool, session_pool);

  (databases, main_temp_file, session_temp_file)
}

/// Create a test user with default password
pub async fn create_test_user_with_databases(
  databases: &DpsBackendDatabases,
  username: &str,
  role_id: i64,
) -> User {
  let main_pool = databases.main();
  let mut main_conn = main_pool.acquire().await.unwrap();
  create_test_user_with_conn(&mut main_conn, username, role_id).await
}

/// Create a test user with default password (takes connection)
pub async fn create_test_user_with_conn(
  main_conn: &mut SqliteConnection,
  username: &str,
  role_id: i64,
) -> User {
  create_test_user_with_conn_and_password(main_conn, username, role_id, "password123").await
}

/// Create a test user with custom password
pub async fn create_test_user_with_databases_and_password(
  databases: &DpsBackendDatabases,
  username: &str,
  role_id: i64,
  password: &str,
) -> User {
  let main_pool = databases.main();
  let mut main_conn = main_pool.acquire().await.unwrap();
  create_test_user_with_conn_and_password(&mut main_conn, username, role_id, password).await
}

/// Create a test user with custom password (takes connection)
pub async fn create_test_user_with_conn_and_password(
  main_conn: &mut SqliteConnection,
  username: &str,
  role_id: i64,
  password: &str,
) -> User {
  create_test_user_full_with_conn(main_conn, username, Some(role_id), password, None).await
}

/// Create a test user with all parameters
pub async fn create_test_user_full_with_databases(
  databases: &DpsBackendDatabases,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  let main_pool = databases.main();
  let mut main_conn = main_pool.acquire().await.unwrap();
  create_test_user_full_with_conn(&mut main_conn, username, role_id, password, metadata_json).await
}

/// Create a test user with all parameters (takes connection)
pub async fn create_test_user_full_with_conn(
  main_conn: &mut SqliteConnection,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  // If no role_id specified, ensure a default role exists
  let final_role_id = if role_id.is_none() {
    // Check if default role exists
    let default_role_id = GetDefaultRoleQuery::run(main_conn)
      .await
      .unwrap()
      .map(|r| r.id);

    match default_role_id {
      Some(id) => Some(id),
      None => {
        // Create a default role if none exists
        Some(
          create_test_role_model_with_conn(main_conn, "user", &["can_view_user_self"], true)
            .await
            .id,
        )
      }
    }
  } else {
    role_id
  };

  let password_hash = GeneratePasswordHashService::run(password).unwrap();
  let metadata_json_str = metadata_json.map(|v| v.to_string());
  let create_data = CreateUserData {
    uuid: Uuid::new_v4().to_string(),
    name: username.to_string(),
    role_id: final_role_id,
    password_hash,
    metadata_json: metadata_json_str,
  };
  CreateUserQuery::run(main_conn, create_data).await.unwrap()
}

/// Create a test user with all parameters including specific UUID
pub async fn create_test_user_with_databases_and_uuid(
  databases: &DpsBackendDatabases,
  uuid: &str,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  let main_pool = databases.main();
  let mut main_conn = main_pool.acquire().await.unwrap();
  create_test_user_with_conn_and_uuid(
    &mut main_conn,
    uuid,
    username,
    role_id,
    password,
    metadata_json,
  )
  .await
}

/// Create a test user with all parameters including specific UUID (takes connection)
pub async fn create_test_user_with_conn_and_uuid(
  main_conn: &mut SqliteConnection,
  uuid: &str,
  username: &str,
  role_id: Option<i64>,
  password: &str,
  metadata_json: Option<serde_json::Value>,
) -> User {
  // If no role_id specified, ensure a default role exists
  let final_role_id = if role_id.is_none() {
    // Check if default role exists
    let default_role_id = GetDefaultRoleQuery::run(main_conn)
      .await
      .unwrap()
      .map(|r| r.id);

    match default_role_id {
      Some(id) => Some(id),
      None => {
        // Create a default role if none exists
        Some(
          create_test_role_model_with_conn(main_conn, "user", &["can_view_user_self"], true)
            .await
            .id,
        )
      }
    }
  } else {
    role_id
  };

  let password_hash = GeneratePasswordHashService::run(password).unwrap();
  let metadata_json_str = metadata_json.map(|v| v.to_string());
  let create_data = CreateUserData {
    uuid: uuid.to_string(),
    name: username.to_string(),
    role_id: final_role_id,
    password_hash,
    metadata_json: metadata_json_str,
  };
  CreateUserQuery::run(main_conn, create_data).await.unwrap()
}

/// Create a test role and return ID
pub async fn create_test_role_with_databases(
  databases: &DpsBackendDatabases,
  name: &str,
  permissions: &[&str],
) -> i64 {
  let role = create_test_role_model_with_databases(databases, name, permissions, false).await;
  role.id
}

/// Create a test role and return ID (takes connection)
pub async fn create_test_role_with_conn(
  main_conn: &mut SqliteConnection,
  name: &str,
  permissions: &[&str],
) -> i64 {
  let role = create_test_role_model_with_conn(main_conn, name, permissions, false).await;
  role.id
}

/// Create a test role and return full Role model
pub async fn create_test_role_model_with_databases(
  databases: &DpsBackendDatabases,
  name: &str,
  permissions: &[&str],
  is_default: bool,
) -> Role {
  let main_pool = databases.main();
  let mut main_conn = main_pool.acquire().await.unwrap();
  create_test_role_model_with_conn(&mut main_conn, name, permissions, is_default).await
}

/// Create a test role and return full Role model (takes connection)
pub async fn create_test_role_model_with_conn(
  main_conn: &mut SqliteConnection,
  name: &str,
  permissions: &[&str],
  is_default: bool,
) -> Role {
  let create_data = CreateRoleData {
    name: name.to_string(),
    permissions: permissions.iter().map(|&p| p.to_string()).collect(),
    is_default,
  };
  CreateRoleQuery::run(main_conn, create_data).await.unwrap()
}

/// Build a fully-migrated, seeded backend router merged into a minimal host
/// router. No TCP listener is opened.
pub async fn create_test_app() -> axum::Router {
  use crate::dps_backend::DpsBackend;
  use axum::routing::get;

  let temp_file = tempfile::Builder::new()
    .prefix("dps_backend_test_")
    .suffix(".db")
    .tempfile()
    .expect("Failed to create temp file");
  let db_path = temp_file
    .path()
    .to_str()
    .expect("Failed to get temp file path")
    .to_string();
  // Keep the temp file alive for the lifetime of the process; the backend
  // opens its own connection pool against this path.
  std::mem::forget(temp_file);

  let mut config = create_test_dps_config();
  config.set_auth_api_sqlite_main_file_path(&db_path);
  config.set_auth_api_sqlite_session_file_path(":memory:");
  config.set_domain("dps.localhost");

  let backend = DpsBackend::new(config).expect("Failed to create test backend");
  backend
    .migrate_database()
    .await
    .expect("Failed to run migrations");
  backend.seed_database().await.expect("Failed to run seeds");

  let host = axum::Router::new()
    .route("/", get(|| async { "OK" }))
    .route("/health", get(|| async { "OK" }))
    .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "NOT FOUND") });

  backend
    .enhance(host)
    .await
    .expect("Failed to build test app")
}

/// Create a test user through the REST registration endpoint.
pub async fn create_test_user_via_api(
  app: &axum::Router,
  username: &str,
  password: &str,
) -> String {
  use axum::{
    body::Body,
    http::{Request, StatusCode},
  };
  use tower::ServiceExt;

  let body = serde_json::json!({
    "username": username,
    "password": password,
    "passwordConfirmation": password,
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

  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let data: serde_json::Value = serde_json::from_slice(&body).unwrap();
  data["user"]["uuid"]
    .as_str()
    .expect("Registration response missing uuid")
    .to_string()
}

#[cfg(test)]
mod tests {
  use dps_backend_test_macros::dps_backend_db_test;

  use super::*;
  use crate::models::ROLE_PERMISSIONS;

  #[dps_backend_db_test]
  async fn test_create_test_user_no_role() {
    // Create user without role (will create and use default role automatically)
    let user =
      create_test_user_full_with_databases(&databases, "testuser", None, "password123", None).await;

    assert_eq!(user.name, "testuser");
    // Should have the default role ID (created automatically)
    assert!(user.role_id > 0);

    // Verify that default role is created automatically
    let mut main_conn = main_pool.acquire().await.unwrap();
    let default_role = GetDefaultRoleQuery::run(&mut main_conn)
      .await
      .unwrap()
      .unwrap();
    assert_eq!(user.role_id, default_role.id);
    assert_eq!(default_role.name, "user");
    assert!(default_role.is_default);
  }

  #[dps_backend_db_test]
  async fn test_create_test_user_custom_password() {
    let role_id =
      create_test_role_with_databases(&databases, "test_role", &["can_view_user_self"]).await;

    // Create user with custom password
    let user =
      create_test_user_with_databases_and_password(&databases, "testuser", role_id, "custompass")
        .await;

    assert_eq!(user.name, "testuser");
    assert_eq!(user.role_id, role_id);
  }

  #[dps_backend_db_test]
  async fn test_create_test_user_full() {
    let role_id =
      create_test_role_with_databases(&databases, "test_role", &["can_view_user_self"]).await;
    let metadata = serde_json::json!({"key": "value"});

    // Create user with all parameters
    let user = create_test_user_full_with_databases(
      &databases,
      "testuser",
      Some(role_id),
      "password123",
      Some(metadata.clone()),
    )
    .await;

    assert_eq!(user.name, "testuser");
    assert_eq!(user.role_id, role_id);
    assert_eq!(user.metadata_json, Some(metadata.to_string()));
  }

  #[dps_backend_db_test]
  async fn test_create_test_role_id() {
    // Create role and get ID
    let role_id = create_test_role_with_databases(
      &databases,
      "test_role",
      &["can_view_user_self", "can_list_users"],
    )
    .await;

    assert!(role_id > 0);
  }

  #[dps_backend_db_test]
  async fn test_create_test_role_model() {
    // Create role and get full model
    let role = create_test_role_model_with_databases(
      &databases,
      "test_role",
      &["can_view_user_self", "can_list_users"],
      true,
    )
    .await;

    assert_eq!(role.name, "test_role");
    assert!(role.is_default);
    assert_eq!(role.permissions.len(), 2);
    assert!(role.permissions.contains(&"can_view_user_self".to_string()));
    assert!(role.permissions.contains(&"can_list_users".to_string()));
  }

  #[dps_backend_db_test]
  async fn test_create_test_role_empty_permissions() {
    // Create role with no permissions
    let role = create_test_role_model_with_databases(&databases, "empty_role", &[], false).await;

    assert_eq!(role.name, "empty_role");
    assert!(!role.is_default);
    assert_eq!(role.permissions.len(), 0);
  }

  #[dps_backend_db_test]
  async fn test_create_test_role_all_permissions() {
    // Create role with all available permissions
    let role =
      create_test_role_model_with_databases(&databases, "admin_role", ROLE_PERMISSIONS, false)
        .await;

    assert_eq!(role.name, "admin_role");
    assert_eq!(role.permissions.len(), ROLE_PERMISSIONS.len());
  }

  #[dps_backend_db_test]
  async fn test_create_test_user_with_uuid_success() {
    use crate::queries::users::GetUserByUuidQuery;

    // Test data
    let test_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let test_username = "testuser";
    let test_password = "password123";
    let test_metadata = serde_json::json!({"key": "value"});

    // Create user with specific UUID
    let user = create_test_user_with_databases_and_uuid(
      &databases,
      test_uuid,
      test_username,
      None, // Use default role
      test_password,
      Some(test_metadata.clone()),
    )
    .await;

    // Verify user was created with correct data
    assert_eq!(user.uuid, test_uuid);
    assert_eq!(user.name, test_username);
    assert!(user.role_id > 0); // Should have default role
    assert!(user.password_hash.starts_with("$argon2")); // Password should be hashed
    assert_eq!(user.metadata_json, Some(test_metadata.to_string()));

    // Verify user can be retrieved by UUID
    let mut main_conn = main_pool.acquire().await.unwrap();
    let retrieved_user = GetUserByUuidQuery::run(&mut main_conn, test_uuid)
      .await
      .unwrap();
    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.uuid, test_uuid);
    assert_eq!(retrieved_user.name, test_username);
  }
}
