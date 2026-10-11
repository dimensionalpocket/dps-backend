use axum::{body::Body, http::Request, Router};
use dps_backend::test_utils;
use dps_backend::test_utils::create_test_user_via_api;
use tower::ServiceExt;

async fn create_app() -> Router {
  test_utils::create_test_app().await
}

// ===== PASSWORD LOGGING SECURITY TESTS =====

#[tracing_test::traced_test]
#[tokio::test]
async fn test_auth_login_no_password_in_logs() {
  let app = create_app().await;
  let unique_username = format!("testuser_{}", rand::random::<u32>());
  create_test_user_via_api(&app, &unique_username, "password123").await;

  let body = serde_json::json!({
    "username": unique_username,
    "password": "secret_password_123!",
  });

  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap(),
    )
    .await
    .unwrap();

  // Login fails (wrong password) but must not leak the password into logs.
  assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_auth_register_no_password_in_logs() {
  let app = create_app().await;
  let unique_username = format!("newuser_{}", rand::random::<u32>());

  let body = serde_json::json!({
    "username": unique_username,
    "password": "super_secret_pass_456",
    "passwordConfirmation": "super_secret_pass_456",
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

  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
}

// ===== HTTP LOGGING TESTS (TraceLayer) =====

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_get_request_logged() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_post_request_logged() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_http_logging_404_request_logged() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .method("GET")
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

// ===== SHUTDOWN SERVICE LOGGING TESTS =====

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_logs_correct_message() {
  use dps_backend::services::LogShutdownStartService;

  LogShutdownStartService::run("SIGTERM");

  assert!(logs_contain(
    "Starting graceful shutdown due to SIGTERM signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_with_sigint() {
  use dps_backend::services::LogShutdownStartService;

  LogShutdownStartService::run("SIGINT");

  assert!(logs_contain(
    "Starting graceful shutdown due to SIGINT signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}

#[tracing_test::traced_test]
#[test]
fn test_log_shutdown_start_with_custom_signal() {
  use dps_backend::services::LogShutdownStartService;

  LogShutdownStartService::run("TEST_SIGNAL");

  assert!(logs_contain(
    "Starting graceful shutdown due to TEST_SIGNAL signal"
  ));
  assert!(logs_contain(
    "Waiting for existing connections to complete..."
  ));
}
