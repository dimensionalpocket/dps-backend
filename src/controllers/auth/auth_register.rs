use crate::database::DpsBackendDatabases;
use crate::services::{AuthRegisterService, GenerateSessionCookieService, RegisterResult};
use crate::types::SessionError;
use dps_config::DpsConfig;

pub struct AuthRegisterController;

impl AuthRegisterController {
  pub async fn run(
    databases: &DpsBackendDatabases,
    username: &str,
    password: &str,
    password_confirmation: &str,
    config: &DpsConfig,
  ) -> Result<(RegisterResult, String), SessionError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result = AuthRegisterService::run(
      &mut main_conn,
      username,
      password,
      password_confirmation,
      config,
    )
    .await
    .map_err(|e| SessionError::AuthenticationError(e.to_string()))?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
