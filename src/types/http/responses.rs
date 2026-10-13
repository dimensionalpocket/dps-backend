use crate::models::user::UserWithRole;
use crate::models::{Role, Site};
use crate::services::{AuthMeResult, AuthResult, RegisterResult};
use serde::Serialize;

/// Standard error envelope returned by the REST API.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
  pub error: String,
}

/// Role information embedded in user responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRoleResponse {
  pub id: i64,
  pub name: String,
  pub permissions: Vec<String>,
}

impl From<Role> for UserRoleResponse {
  fn from(role: Role) -> Self {
    Self {
      id: role.id,
      name: role.name,
      permissions: role.permissions,
    }
  }
}

/// Full user representation with role information.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
  pub id: i64,
  pub uuid: Option<String>,
  pub name: String,
  pub role: UserRoleResponse,
  pub metadata_json: Option<String>,
  pub created_ts: Option<i64>,
  pub updated_ts: Option<i64>,
}

impl From<UserWithRole> for UserResponse {
  fn from(user_with_role: UserWithRole) -> Self {
    Self {
      id: user_with_role.user.id,
      uuid: Some(user_with_role.user.uuid),
      name: user_with_role.user.name,
      role: UserRoleResponse::from(user_with_role.role),
      metadata_json: user_with_role.user.metadata_json,
      created_ts: Some(user_with_role.user.created_ts),
      updated_ts: Some(user_with_role.user.updated_ts),
    }
  }
}

/// User representation used for listings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummaryResponse {
  pub id: i64,
  pub uuid: String,
  pub name: String,
  pub role: UserRoleResponse,
  pub created_ts: i64,
  pub updated_ts: i64,
}

impl From<UserWithRole> for UserSummaryResponse {
  fn from(user_with_role: UserWithRole) -> Self {
    Self {
      id: user_with_role.user.id,
      uuid: user_with_role.user.uuid,
      name: user_with_role.user.name,
      role: UserRoleResponse::from(user_with_role.role),
      created_ts: user_with_role.user.created_ts,
      updated_ts: user_with_role.user.updated_ts,
    }
  }
}

/// Full site representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteResponse {
  pub id: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
  pub metadata_json: Option<String>,
  pub created_ts: i64,
  pub updated_ts: i64,
}

impl From<Site> for SiteResponse {
  fn from(site: Site) -> Self {
    Self {
      id: site.id,
      slug: site.slug,
      subdomain: site.subdomain,
      port: site.port,
      protocol: site.protocol,
      metadata_json: site.metadata_json,
      created_ts: site.created_ts,
      updated_ts: site.updated_ts,
    }
  }
}

/// Site representation used for listings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSummaryResponse {
  pub id: i64,
  pub slug: String,
  pub subdomain: Option<String>,
  pub port: Option<i64>,
  pub protocol: String,
}

impl From<Site> for SiteSummaryResponse {
  fn from(site: Site) -> Self {
    Self {
      id: site.id,
      slug: site.slug,
      subdomain: site.subdomain,
      port: site.port,
      protocol: site.protocol,
    }
  }
}

/// Full role representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleResponse {
  pub id: i64,
  pub name: String,
  pub is_default: bool,
  pub permissions: Vec<String>,
  pub created_ts: i64,
  pub updated_ts: i64,
}

impl From<Role> for RoleResponse {
  fn from(role: Role) -> Self {
    Self {
      id: role.id,
      name: role.name,
      is_default: role.is_default,
      permissions: role.permissions,
      created_ts: role.created_ts,
      updated_ts: role.updated_ts,
    }
  }
}

/// Role representation used for listings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleSummaryResponse {
  pub id: i64,
  pub name: String,
  pub is_default: bool,
  pub permissions: Vec<String>,
  pub created_ts: i64,
  pub updated_ts: i64,
}

impl From<Role> for RoleSummaryResponse {
  fn from(role: Role) -> Self {
    Self {
      id: role.id,
      name: role.name,
      is_default: role.is_default,
      permissions: role.permissions,
      created_ts: role.created_ts,
      updated_ts: role.updated_ts,
    }
  }
}

/// Response for `POST /auth/register`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
  pub user: UserResponse,
  pub message: String,
}

impl RegisterResponse {
  pub fn from_register_result(result: RegisterResult) -> Self {
    Self {
      user: UserResponse {
        id: result.user_id,
        uuid: Some(result.uuid),
        name: result.username,
        role: UserRoleResponse::from(result.role),
        metadata_json: None,
        created_ts: Some(result.created_ts),
        updated_ts: Some(result.updated_ts),
      },
      message: "User registration successful".to_string(),
    }
  }
}

/// User representation returned by login (no uuid/timestamps available).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserResponse {
  pub id: i64,
  pub name: String,
  pub role: UserRoleResponse,
}

/// Response for `POST /auth/login`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
  pub token: String,
  pub user: LoginUserResponse,
  pub message: String,
}

impl LoginResponse {
  pub fn from_auth_result(result: AuthResult) -> Self {
    Self {
      token: result.session_token,
      user: LoginUserResponse {
        id: result.user_id,
        name: result.username,
        role: UserRoleResponse::from(result.role),
      },
      message: "Authentication successful".to_string(),
    }
  }
}

/// Response for `POST /auth/logout`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {
  pub message: String,
}

/// Response for `GET /auth/me`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
  pub user: UserResponse,
  pub session_iat: i64,
  pub session_exp: i64,
}

impl MeResponse {
  pub fn from_auth_me_result(result: AuthMeResult) -> Self {
    Self {
      user: UserResponse {
        id: result.user_id,
        uuid: Some(result.uuid),
        name: result.username,
        role: UserRoleResponse::from(result.role),
        metadata_json: None,
        created_ts: Some(result.created_ts),
        updated_ts: Some(result.updated_ts),
      },
      session_iat: result.session_iat,
      session_exp: result.session_exp,
    }
  }
}

/// Response for `PATCH /auth/password`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordResponse {
  pub message: String,
  pub updated_ts: i64,
}

/// Response for `DELETE /roles/{id}`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRoleResponse {
  pub success: bool,
  pub id: i64,
  pub name: String,
}

/// Response for `DELETE /users/{id}`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserResponse {
  pub success: bool,
}

/// Response for `GET /server-timestamp`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampResponse {
  pub timestamp: String,
}

/// Response for `GET /role-permissions` (serializes as a plain JSON array).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct RolePermissionsResponse(pub Vec<String>);
