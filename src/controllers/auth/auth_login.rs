use crate::database::DpsBackendDatabases;
use crate::services::{AuthLoginService, AuthResult, GenerateSessionCookieService};
use crate::types::SessionError;
use dps_config::DpsConfig;

pub struct AuthLoginController;

impl AuthLoginController {
  pub async fn run(
    databases: &DpsBackendDatabases,
    username: &str,
    password: &str,
    config: &DpsConfig,
  ) -> Result<(AuthResult, String), SessionError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(|e| SessionError::DatabaseError(e.to_string()))?;

    let auth_result = AuthLoginService::run(&mut main_conn, username, password, config).await?;

    let cookie_value = GenerateSessionCookieService::run(config, &auth_result.session_token);

    Ok((auth_result, cookie_value))
  }
}
