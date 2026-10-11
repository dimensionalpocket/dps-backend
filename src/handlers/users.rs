use crate::controllers::user::{
  DeleteUserController, GetUserController, GetUsersController, UpdateUserController,
};
use crate::dps_backend::AppState;
use crate::handlers::error::ApiError;
use crate::middleware::session::SessionContext;
use crate::types::http::{
  DeleteUserResponse, UpdateUserRequest, UserResponse, UserSummaryResponse,
};
use crate::types::UpdateUserInput;
use axum::extract::{Extension, Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::instrument;

/// `GET /users`
#[instrument(skip(state, session))]
pub async fn list(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
) -> Result<Response, ApiError> {
  let users = GetUsersController::run(&state.databases, session, state.config.as_ref()).await?;
  let body: Vec<UserSummaryResponse> = users.into_iter().map(UserSummaryResponse::from).collect();
  Ok(Json(body).into_response())
}

/// `GET /users/{id}`
#[instrument(skip(state, session), fields(user_id = %id))]
pub async fn show(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let user = GetUserController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(UserResponse::from(user)).into_response())
}

/// `PATCH /users/{id}`
#[instrument(skip(state, session, request), fields(user_id = %id))]
pub async fn update(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
  Json(request): Json<UpdateUserRequest>,
) -> Result<Response, ApiError> {
  let input = UpdateUserInput {
    id,
    name: request.name,
    role_id: request.role_id,
    password: request.password,
    password_confirmation: request.password_confirmation,
    metadata_json: request.metadata_json,
  };

  let user =
    UpdateUserController::run(&state.databases, session, state.config.as_ref(), input).await?;

  Ok(Json(UserResponse::from(user)).into_response())
}

/// `DELETE /users/{id}`
#[instrument(skip(state, session), fields(user_id = %id))]
pub async fn remove(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  DeleteUserController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(DeleteUserResponse { success: true }).into_response())
}
