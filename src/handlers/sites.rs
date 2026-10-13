use crate::controllers::site::{
  AddSiteController, GetSiteController, GetSitesController, RemoveSiteController,
  UpdateSiteController,
};
use crate::dps_backend::AppState;
use crate::handlers::error::ApiError;
use crate::middleware::session::SessionContext;
use crate::queries::sites::{CreateSiteData, UpdateSiteData};
use crate::types::http::{AddSiteRequest, SiteResponse, SiteSummaryResponse, UpdateSiteRequest};
use crate::types::SiteError;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::instrument;

/// `GET /sites`
#[instrument(skip(state))]
pub async fn list(State(state): State<AppState>) -> Result<Response, ApiError> {
  let sites = GetSitesController::run(&state.databases).await?;
  let body: Vec<SiteSummaryResponse> = sites.into_iter().map(SiteSummaryResponse::from).collect();
  Ok(Json(body).into_response())
}

/// `GET /sites/{id}`
#[instrument(skip(state, session), fields(site_id = %id))]
pub async fn show(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let site = GetSiteController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(SiteResponse::from(site)).into_response())
}

/// `POST /sites`
#[instrument(skip(state, session, request))]
pub async fn create(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Json(request): Json<AddSiteRequest>,
) -> Result<Response, ApiError> {
  let create_data = CreateSiteData {
    slug: request.slug,
    subdomain: request.subdomain,
    port: request.port,
    protocol: request.protocol,
    metadata_json: request.metadata_json,
  };

  let site = AddSiteController::run(
    &state.databases,
    session,
    state.config.as_ref(),
    create_data,
  )
  .await?;

  Ok((StatusCode::CREATED, Json(SiteResponse::from(site))).into_response())
}

/// `PATCH /sites/{id}`
#[instrument(skip(state, session, request), fields(site_id = %id))]
pub async fn update(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
  Json(request): Json<UpdateSiteRequest>,
) -> Result<Response, ApiError> {
  let update_data = UpdateSiteData {
    id,
    slug: request.slug,
    subdomain: request.subdomain.map(Some),
    port: request.port.map(Some),
    protocol: request.protocol,
    metadata_json: request.metadata_json.map(Some),
  };

  match UpdateSiteController::run(
    &state.databases,
    session,
    state.config.as_ref(),
    id,
    update_data,
  )
  .await?
  {
    Some(site) => Ok(Json(SiteResponse::from(site)).into_response()),
    None => Err(ApiError::Site(SiteError::SiteNotFound(id))),
  }
}

/// `DELETE /sites/{id}`
#[instrument(skip(state, session), fields(site_id = %id))]
pub async fn remove(
  State(state): State<AppState>,
  Extension(session): Extension<SessionContext>,
  Path(id): Path<i64>,
) -> Result<Response, ApiError> {
  let site =
    RemoveSiteController::run(&state.databases, session, state.config.as_ref(), id).await?;
  Ok(Json(SiteResponse::from(site)).into_response())
}
