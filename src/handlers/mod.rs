pub mod auth;
pub mod error;
pub mod pages;
pub mod roles;
pub mod sites;
pub mod system;
pub mod users;

pub use error::ApiError;

use axum::http::{header, HeaderValue};
use axum::response::Response;

/// Append a `Set-Cookie` header to an already-built response.
pub(crate) fn attach_cookie(mut response: Response, cookie: &str) -> Response {
  if let Ok(value) = HeaderValue::from_str(cookie) {
    response.headers_mut().append(header::SET_COOKIE, value);
  } else {
    tracing::error!("Failed to build Set-Cookie header value");
  }
  response
}
