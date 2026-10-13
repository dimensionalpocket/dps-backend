use axum::{
  body::Body,
  http::{Request, StatusCode},
  response::Response,
  Router,
};
use dps_backend::test_utils;
use dps_backend::test_utils::create_test_user_via_api;
use tower::ServiceExt;

async fn create_app() -> Router {
  test_utils::create_test_app().await
}

async fn to_json(response: Response<Body>) -> serde_json::Value {
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  serde_json::from_slice(&body).unwrap()
}

async fn login(app: &Router, username: &str, password: &str) -> Response<Body> {
  let body = serde_json::json!({ "username": username, "password": password });
  app
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
    .unwrap()
}

#[tokio::test]
async fn test_root_endpoint() {
  let app = create_app().await;

  let response = app
    .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body = String::from_utf8(body.to_vec()).unwrap();
  assert!(body.contains("data-page=\"app\""));
}

#[tokio::test]
async fn test_health_endpoint() {
  let app = create_app().await;

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
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn test_server_timestamp_endpoint() {
  let app = create_app().await;

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
  let data = to_json(response).await;
  assert!(data["timestamp"].as_str().unwrap().parse::<u64>().unwrap() > 0);
}

#[tokio::test]
async fn test_register_returns_created_with_cookie() {
  let app = create_app().await;

  let body = serde_json::json!({
    "username": "register_user",
    "password": "password123",
    "passwordConfirmation": "password123",
  });

  let response = app
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
    .expect("Set-Cookie header should be present")
    .to_str()
    .unwrap()
    .to_string();
  assert!(cookie.contains("DpsSession="));
  assert!(cookie.contains("Domain=.dps.localhost"));
  assert!(cookie.contains("Path=/api"));
  assert!(cookie.contains("HttpOnly"));
  assert!(!cookie.contains("Secure"));

  let data = to_json(response).await;
  assert!(!data["user"]["uuid"].as_str().unwrap().is_empty());
  assert_eq!(
    data["message"].as_str().unwrap(),
    "User registration successful"
  );
}

#[tokio::test]
async fn test_login_success_sets_cookie() {
  let app = create_app().await;
  create_test_user_via_api(&app, "login_user", "password123").await;

  let response = login(&app, "login_user", "password123").await;
  assert_eq!(response.status(), StatusCode::OK);
  assert!(response.headers().get("set-cookie").is_some());

  let data = to_json(response).await;
  assert!(!data["token"].as_str().unwrap().is_empty());
  assert_eq!(data["user"]["name"].as_str().unwrap(), "login_user");
  assert_eq!(
    data["message"].as_str().unwrap(),
    "Authentication successful"
  );
}

#[tokio::test]
async fn test_login_invalid_credentials_returns_401() {
  let app = create_app().await;
  create_test_user_via_api(&app, "bad_login", "password123").await;

  let response = login(&app, "bad_login", "wrong_password").await;
  assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
  assert!(response.headers().get("set-cookie").is_none());

  let data = to_json(response).await;
  assert_eq!(data["error"].as_str().unwrap(), "Invalid credentials");
}

#[tokio::test]
async fn test_login_missing_user_returns_401() {
  let app = create_app().await;

  let response = login(&app, "nonexistent", "password123").await;
  assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

  let data = to_json(response).await;
  assert_eq!(data["error"].as_str().unwrap(), "Invalid credentials");
}

#[tokio::test]
async fn test_auth_me_requires_session() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .uri("/api/auth/me")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_me_authenticated_with_bearer() {
  let app = create_app().await;
  create_test_user_via_api(&app, "me_user", "password123").await;

  let login_data = to_json(login(&app, "me_user", "password123").await).await;
  let token = login_data["token"].as_str().unwrap().to_string();

  let response = app
    .oneshot(
      Request::builder()
        .uri("/api/auth/me")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let data = to_json(response).await;
  assert_eq!(data["user"]["name"].as_str().unwrap(), "me_user");
  assert!(data["sessionIat"].as_i64().unwrap() > 0);
  assert!(data["sessionExp"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_auth_me_authenticated_with_cookie() {
  let app = create_app().await;
  create_test_user_via_api(&app, "cookie_user", "password123").await;

  let login_response = login(&app, "cookie_user", "password123").await;
  let cookie = login_response
    .headers()
    .get("set-cookie")
    .unwrap()
    .to_str()
    .unwrap()
    .to_string();

  let response = app
    .oneshot(
      Request::builder()
        .uri("/api/auth/me")
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);
  let data = to_json(response).await;
  assert_eq!(data["user"]["name"].as_str().unwrap(), "cookie_user");
}

#[tokio::test]
async fn test_auth_me_invalid_token_is_unauthorized() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .uri("/api/auth/me")
        .header("authorization", "Bearer invalid_token")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_404_handler_renders_inertia_error_page() {
  let app = create_app().await;

  let response = app
    .oneshot(
      Request::builder()
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  let body = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
  let body = String::from_utf8(body.to_vec()).unwrap();
  assert!(body.contains("\"component\":\"Error\"") || body.contains("\"component\": \"Error\""));
}

#[test]
fn test_session_context_utility_methods() {
  use dps_auth_session::DpsAuthSessionPayload;
  use dps_backend::middleware::session::SessionContext;
  use dps_backend::test_utils::create_test_dps_config;
  use dps_backend::utils::session_context_sub_to_user_id;

  let config = create_test_dps_config();

  let empty_context = SessionContext::new(None);
  assert!(!empty_context.authenticated());
  assert!(session_context_sub_to_user_id(&empty_context, &config).is_err());

  let payload = DpsAuthSessionPayload {
    sub: "123".to_string(),
    iat: 1000,
    exp: 2000,
  };
  let auth_context = SessionContext::new(Some(payload));
  assert!(auth_context.authenticated());
  assert_eq!(
    session_context_sub_to_user_id(&auth_context, &config),
    Ok(123)
  );
}

#[test]
fn test_session_cookie_name_constant() {
  use dps_backend::middleware::session::SESSION_COOKIE_NAME;
  assert_eq!(SESSION_COOKIE_NAME, "DpsSession");
}
