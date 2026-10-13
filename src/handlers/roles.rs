use crate::controllers::role::{
  AddRoleController, GetRoleController, GetRolePermissionsController, GetRolesController,
  RemoveRoleController, SetDefaultRoleController, UpdateRoleController,
};
use crate::dps_backend::AppState;
use crate::handlers::error::ApiError;
use crate::middleware::session::SessionContext;
use crate::queries::roles::{CreateRoleData, UpdateRoleData};
use crate::types::http::{
  AddRoleRequest, RemoveRoleResponse, RolePermissionsResponse, RoleResponse, RoleSummaryResponse,
  UpdateRoleRequest,
};
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::instrument;

/// `GET /roles`
#[instrument(skip(state, session))]
pub async fn list(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
) -> Result<Response, ApiError> {
  let roles = GetRolesController::run(&state.databases, session, state.config.as_ref()).await?;
  let body: Vec<RoleSummaryResponse> = roles.into_iter().map(RoleSummaryResponse::from).collect();
  Ok(Json(body).into_response())
}

/// `GET /roles/{id}`
#[instrument(skip(state, session), fields(role_id = %id))]
pub async fn show(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let role = GetRoleController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(RoleResponse::from(role)).into_response())
}

/// `GET /role-permissions`
#[instrument(skip(state, session))]
pub async fn permissions(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
) -> Result<Response, ApiError> {
  let permissions =
    GetRolePermissionsController::run(&state.databases, session, state.config.as_ref()).await?;
  Ok(Json(RolePermissionsResponse(permissions)).into_response())
}

/// `POST /roles`
#[instrument(skip(state, session, request))]
pub async fn create(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Json(request): Json<AddRoleRequest>,
) -> Result<Response, ApiError> {
  let create_data = CreateRoleData {
    name: request.name,
    permissions: request.permissions,
    is_default: false,
  };

  let role = AddRoleController::run(
    &state.databases,
    session,
    state.config.as_ref(),
    create_data,
  )
  .await?;

  Ok((StatusCode::CREATED, Json(RoleResponse::from(role))).into_response())
}

/// `PATCH /roles/{id}`
#[instrument(skip(state, session, request), fields(role_id = %id))]
pub async fn update(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
  Json(request): Json<UpdateRoleRequest>,
) -> Result<Response, ApiError> {
  let update_data = UpdateRoleData {
    id,
    name: request.name,
    permissions: request.permissions,
  };

  let role = UpdateRoleController::run(
    &state.databases,
    session,
    state.config.as_ref(),
    id,
    update_data,
  )
  .await?;

  Ok(Json(RoleResponse::from(role)).into_response())
}

/// `PUT /roles/{id}/default`
#[instrument(skip(state, session), fields(role_id = %id))]
pub async fn set_default(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let role =
    SetDefaultRoleController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(RoleResponse::from(role)).into_response())
}

/// `DELETE /roles/{id}`
#[instrument(skip(state, session), fields(role_id = %id))]
pub async fn remove(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let role =
    RemoveRoleController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(
    Json(RemoveRoleResponse {
      success: true,
      id: role.id,
      name: role.name,
    })
    .into_response(),
  )
}
