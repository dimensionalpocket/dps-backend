use crate::dps_backend::AppState;
use axum::extract::{Path, State};
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use inertia_rs::axum::{InertiaError, InertiaRequest};
use inertia_rs::Inertia;
use serde::Serialize;
use serde_json::json;
use tracing::instrument;

/// Render an Inertia page using the frontend root document.
fn render_page<T: Serialize>(
  state: &AppState,
  inertia: &InertiaRequest,
  component: &str,
  props: T,
) -> Response {
  let frontend = state.frontend.clone();
  inertia
    .render(Inertia::response(component, props), |ctx| {
      Html(frontend.root_html(ctx.data_page())).into_response()
    })
    .unwrap_or_else(|error: InertiaError| {
      tracing::error!(?error, "failed to render Inertia page");
      (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    })
}

/// `GET /`
#[instrument(skip(state, inertia))]
pub async fn home(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Home", json!({}))
}

/// `GET /login`
#[instrument(skip(state, inertia))]
pub async fn login(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Auth/Login", json!({}))
}

/// `GET /register`
#[instrument(skip(state, inertia))]
pub async fn register(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Auth/Register", json!({}))
}

/// `GET /login-success`
#[instrument(skip(state, inertia))]
pub async fn login_success(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Auth/LoginSuccess", json!({}))
}

/// `GET /password-change`
#[instrument(skip(state, inertia))]
pub async fn password_change(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Auth/PasswordChange", json!({}))
}

/// `GET /admin`
#[instrument(skip(state, inertia))]
pub async fn admin_home(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Admin/Home", json!({}))
}

/// `GET /admin/users`
#[instrument(skip(state, inertia))]
pub async fn admin_users(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Admin/Users", json!({}))
}

/// `GET /admin/sites`
#[instrument(skip(state, inertia))]
pub async fn admin_sites(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Admin/Sites", json!({}))
}

/// `GET /admin/sites/new`
#[instrument(skip(state, inertia))]
pub async fn admin_sites_new(State(state): State<AppState>, inertia: InertiaRequest) -> Response {
  render_page(&state, &inertia, "Admin/SitesNew", json!({}))
}

/// `GET /admin/sites/{id}/edit`
#[instrument(skip(state, inertia))]
pub async fn admin_sites_edit(
  State(state): State<AppState>,
  Path(id): Path<i64>,
  inertia: InertiaRequest,
) -> Response {
  render_page(&state, &inertia, "Admin/SitesEdit", json!({ "id": id }))
}

/// Root fallback: API paths keep the plain-text 404; everything else renders the
/// Inertia `Error` page.
#[instrument(skip(state, inertia))]
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

#[cfg(test)]
mod tests {
  use crate::test_utils::create_test_app;
  use axum::body::Body;
  use axum::http::{Request, StatusCode};
  use tower::ServiceExt;

  async fn get(uri: &str) -> axum::response::Response {
    create_test_app()
      .await
      .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
      .await
      .unwrap()
  }

  async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
      .await
      .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
  }

  #[tokio::test]
  async fn test_pages_render_html_with_data_page_and_component() {
    let cases = [
      ("/", "Home"),
      ("/login", "Auth/Login"),
      ("/register", "Auth/Register"),
      ("/login-success", "Auth/LoginSuccess"),
      ("/password-change", "Auth/PasswordChange"),
      ("/admin", "Admin/Home"),
      ("/admin/users", "Admin/Users"),
      ("/admin/sites", "Admin/Sites"),
      ("/admin/sites/new", "Admin/SitesNew"),
    ];

    for (uri, component) in cases {
      let response = get(uri).await;
      assert_eq!(response.status(), StatusCode::OK, "GET {uri}");
      let body = body_string(response).await;
      assert!(
        body.contains("data-page=\"app\""),
        "GET {uri} missing data-page"
      );
      assert!(
        body.contains(component),
        "GET {uri} missing component {component}"
      );
    }
  }

  #[tokio::test]
  async fn test_login_inertia_json_response() {
    let response = create_test_app()
      .await
      .oneshot(
        Request::builder()
          .uri("/login")
          .header("X-Inertia", "true")
          .header("X-Inertia-Version", "dev")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

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
    assert!(body_string(response)
      .await
      .contains("\"component\":\"Auth/Login\""));
  }

  #[tokio::test]
  async fn test_admin_sites_edit_passes_id_prop() {
    let response = get("/admin/sites/7/edit").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert!(body.contains("Admin/SitesEdit"));
    assert!(body.contains("\"id\":7"));
  }

  #[tokio::test]
  async fn test_not_found_api_path_keeps_plain_text() {
    let response = get("/api/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_string(response).await, "NOT FOUND");
  }

  #[tokio::test]
  async fn test_not_found_non_api_path_renders_error_page() {
    let response = get("/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(body_string(response).await.contains("Error"));
  }
}
