use crate::services::GetServerTimestampService;
use crate::types::http::TimestampResponse;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::instrument;

/// `GET /server-timestamp`
#[instrument]
pub async fn server_timestamp() -> Response {
  let timestamp = GetServerTimestampService::run().to_string();
  Json(TimestampResponse { timestamp }).into_response()
}

/// `GET /health`
#[instrument]
pub async fn health() -> &'static str {
  "OK"
}
