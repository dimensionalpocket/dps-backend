use crate::types::{ErrorResponse, RoleError, SessionError, SiteError, UserError};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Unified error type for REST handlers.
///
/// Converts each domain error into an appropriate HTTP status code and a
/// uniform `{"error": "..."}` JSON body. Internal errors are logged and never
/// surfaced to clients.
#[derive(Debug)]
pub enum ApiError {
  Session(SessionError),
  User(UserError),
  Role(RoleError),
  Site(SiteError),
  InvalidCredentials,
  Unauthorized,
}

impl From<SessionError> for ApiError {
  fn from(error: SessionError) -> Self {
    Self::Session(error)
  }
}

impl From<UserError> for ApiError {
  fn from(error: UserError) -> Self {
    Self::User(error)
  }
}

impl From<RoleError> for ApiError {
  fn from(error: RoleError) -> Self {
    Self::Role(error)
  }
}

impl From<SiteError> for ApiError {
  fn from(error: SiteError) -> Self {
    Self::Site(error)
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let (status, message) = match &self {
      ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()),
      ApiError::Unauthorized => (
        StatusCode::UNAUTHORIZED,
        "Authentication required".to_string(),
      ),
      ApiError::Session(error) => match error {
        SessionError::AuthenticationError(_) => (
          StatusCode::UNAUTHORIZED,
          "Authentication required".to_string(),
        ),
        _ => (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Internal server error".to_string(),
        ),
      },
      ApiError::User(error) => match error {
        UserError::UsernameAlreadyExists(_) | UserError::SelfDeletion => {
          (StatusCode::CONFLICT, error.to_string())
        }
        UserError::UserNotFound(_) => (StatusCode::NOT_FOUND, error.to_string()),
        UserError::ValidationError(_) => (StatusCode::BAD_REQUEST, error.to_string()),
        UserError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, error.to_string()),
        UserError::AuthorizationError(_) => (StatusCode::FORBIDDEN, error.to_string()),
        _ => (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Internal server error".to_string(),
        ),
      },
      ApiError::Role(error) => match error {
        RoleError::RoleNotFound(_) => (StatusCode::NOT_FOUND, error.to_string()),
        RoleError::RoleNameAlreadyExists(_) | RoleError::RoleInUse(_) => {
          (StatusCode::CONFLICT, error.to_string())
        }
        RoleError::ValidationError(_) | RoleError::InvalidPermission(_) => {
          (StatusCode::BAD_REQUEST, error.to_string())
        }
        RoleError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, error.to_string()),
        RoleError::AuthorizationError(_) => (StatusCode::FORBIDDEN, error.to_string()),
        _ => (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Internal server error".to_string(),
        ),
      },
      ApiError::Site(error) => match error {
        SiteError::SiteNotFound(_) => (StatusCode::NOT_FOUND, error.to_string()),
        SiteError::SlugAlreadyExists(_) => (StatusCode::CONFLICT, error.to_string()),
        SiteError::ValidationError(_) => (StatusCode::BAD_REQUEST, error.to_string()),
        SiteError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, error.to_string()),
        SiteError::AuthorizationError(_) => (StatusCode::FORBIDDEN, error.to_string()),
        _ => (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Internal server error".to_string(),
        ),
      },
    };

    if status.is_server_error() {
      tracing::error!(error = ?self, "API error");
    }

    (status, Json(ErrorResponse { error: message })).into_response()
  }
}
