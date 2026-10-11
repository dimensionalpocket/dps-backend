use crate::controllers::auth::{
  AuthChangePasswordController, AuthLoginController, AuthMeController, AuthRegisterController,
};
use crate::dps_backend::AppState;
use crate::handlers::attach_cookie;
use crate::handlers::error::ApiError;
use crate::middleware::session::SessionContext;
use crate::services::GenerateLogoutCookieService;
use crate::types::http::{
  ChangePasswordRequest, ChangePasswordResponse, LoginRequest, LoginResponse, LogoutResponse,
  MeResponse, RegisterRequest, RegisterResponse,
};
use crate::types::SessionError;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::instrument;

/// `POST /auth/register`
#[instrument(skip(state, request), fields(username = %request.username))]
pub async fn register(
  State(state): State<AppState>,
  Json(request): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
  let (register_result, cookie_value) = AuthRegisterController::run(
    &state.databases,
    &request.username,
    &request.password,
    &request.password_confirmation,
    state.config.as_ref(),
  )
  .await?;

  let response = (
    StatusCode::CREATED,
    Json(RegisterResponse::from_register_result(register_result)),
  )
    .into_response();

  Ok(attach_cookie(response, &cookie_value))
}

/// `POST /auth/login`
#[instrument(skip(state, request), fields(username = %request.username))]
pub async fn login(
  State(state): State<AppState>,
  Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
  let result = AuthLoginController::run(
    &state.databases,
    &request.username,
    &request.password,
    state.config.as_ref(),
  )
  .await;

  match result {
    Ok((auth_result, cookie_value)) => {
      let response = (
        StatusCode::OK,
        Json(LoginResponse::from_auth_result(auth_result)),
      )
        .into_response();
      Ok(attach_cookie(response, &cookie_value))
    }
    Err(SessionError::AuthenticationError(_)) => Err(ApiError::InvalidCredentials),
    Err(error) => Err(ApiError::Session(error)),
  }
}

/// `POST /auth/logout`
#[instrument(skip(state))]
pub async fn logout(State(state): State<AppState>) -> Result<Response, ApiError> {
  let cookie_value = GenerateLogoutCookieService::run(state.config.as_ref());
  let response = Json(LogoutResponse {
    message: "Successfully logged out".to_string(),
  })
  .into_response();

  Ok(attach_cookie(response, &cookie_value))
}

/// `PATCH /auth/password`
#[instrument(skip(state, session, request))]
pub async fn change_password(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Json(request): Json<ChangePasswordRequest>,
) -> Result<Response, ApiError> {
  let updated_user = AuthChangePasswordController::run(
    &state.databases,
    session,
    state.config.as_ref(),
    &request.current_password,
    &request.new_password,
    &request.new_password_confirmation,
  )
  .await?;

  Ok(
    Json(ChangePasswordResponse {
      message: "Password changed successfully".to_string(),
      updated_ts: updated_user.updated_ts,
    })
    .into_response(),
  )
}

/// `GET /auth/me`
#[instrument(skip(state, session))]
pub async fn me(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
) -> Result<Response, ApiError> {
  match AuthMeController::run(&state.databases, session, state.config.as_ref()).await {
    Ok(Some(result)) => Ok(Json(MeResponse::from_auth_me_result(result)).into_response()),
    Ok(None) => Err(ApiError::Unauthorized),
    Err(error) => Err(ApiError::Session(error)),
  }
}
