use serde::Deserialize;

/// Request body for `POST /auth/register`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
  pub username: String,
  pub password: String,
  pub password_confirmation: String,
}

/// Request body for `POST /auth/login`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
  pub username: String,
  pub password: String,
}

/// Request body for `PATCH /auth/password`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
  pub current_password: String,
  pub new_password: String,
  pub new_password_confirmation: String,
}

/// Request body for `POST /sites`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSiteRequest {
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: Option<String>,
  pub metadata_json: Option<String>,
}

/// Request body for `PATCH /sites/{id}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSiteRequest {
  pub slug: Option<String>,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: Option<String>,
  pub metadata_json: Option<String>,
}

/// Request body for `PATCH /users/{id}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
  pub name: Option<String>,
  pub role_id: Option<i64>,
  pub password: Option<String>,
  pub password_confirmation: Option<String>,
  pub metadata_json: Option<String>,
}

/// Request body for `POST /roles`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRoleRequest {
  pub name: String,
  pub permissions: Vec<String>,
}

/// Request body for `PATCH /roles/{id}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
  pub name: Option<String>,
  pub permissions: Option<Vec<String>>,
}
