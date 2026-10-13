use axum::{
  body::{Body, Bytes},
  http::{Request, StatusCode},
  response::Response,
  Router,
};
use dps_backend::test_utils;
use tower::ServiceExt;

async fn create_app() -> Router {
  test_utils::create_test_app().await
}

async fn send(app: &Router, method: &str, uri: &str, json_body: Option<&str>) -> Response<Body> {
  let mut builder = Request::builder().method(method).uri(uri);
  let body = match json_body {
    Some(payload) => {
      builder = builder.header("content-type", "application/json");
      Body::from(payload.to_string())
    }
    None => Body::empty(),
  };
  app
    .clone()
    .oneshot(builder.body(body).unwrap())
    .await
    .unwrap()
}

async fn body_bytes(response: Response<Body>) -> Bytes {
  axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap()
}

#[tokio::test]
async fn test_all_routes_are_mounted() {
  let app = create_app().await;

  let empty_json = Some("{}");
  let cases: &[(&str, &str, Option<&str>)] = &[
    ("GET", "/api/server-timestamp", None),
    ("POST", "/api/auth/register", empty_json),
    ("POST", "/api/auth/login", empty_json),
    ("POST", "/api/auth/logout", None),
    ("PATCH", "/api/auth/password", empty_json),
    ("GET", "/api/auth/me", None),
    ("GET", "/api/sites", None),
    ("GET", "/api/sites/1", None),
    ("POST", "/api/sites", empty_json),
    ("PATCH", "/api/sites/1", empty_json),
    ("DELETE", "/api/sites/1", None),
    ("GET", "/api/users", None),
    ("GET", "/api/users/1", None),
    ("PATCH", "/api/users/1", empty_json),
    ("DELETE", "/api/users/1", None),
    ("GET", "/api/roles", None),
    ("GET", "/api/roles/1", None),
    ("GET", "/api/role-permissions", None),
    ("POST", "/api/roles", empty_json),
    ("PATCH", "/api/roles/1", empty_json),
    ("PUT", "/api/roles/1/default", None),
    ("DELETE", "/api/roles/1", None),
  ];

  for (method, uri, body) in cases {
    let response = send(&app, method, uri, *body).await;
    let status = response.status();
    let bytes = body_bytes(response).await;

    assert_ne!(
      status,
      StatusCode::METHOD_NOT_ALLOWED,
      "{method} {uri} returned 405"
    );
    let is_fallback = status == StatusCode::NOT_FOUND && &bytes[..] == b"NOT FOUND";
    assert!(!is_fallback, "{method} {uri} is not mounted");
  }
}

#[tokio::test]
async fn test_json_content_type_contract() {
  let app = create_app().await;

  let response = send(&app, "GET", "/api/server-timestamp", None).await;
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    response
      .headers()
      .get("content-type")
      .unwrap()
      .to_str()
      .unwrap(),
    "application/json"
  );

  let response = send(&app, "POST", "/api/auth/logout", None).await;
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    response
      .headers()
      .get("content-type")
      .unwrap()
      .to_str()
      .unwrap(),
    "application/json"
  );
}

#[tokio::test]
async fn test_wrong_method_returns_405() {
  let app = create_app().await;

  let response = send(&app, "GET", "/api/auth/logout", None).await;
  assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_unmounted_route_returns_fallback() {
  let app = create_app().await;

  let response = send(&app, "GET", "/api/does-not-exist", None).await;
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(&body_bytes(response).await[..], b"NOT FOUND");
}
