use crate::database::DpsBackendDatabases;
use crate::models::Site;
use crate::services::GetAllSitesService;
use crate::types::SiteError;

pub struct GetSitesController;

impl GetSitesController {
  pub async fn run(databases: &DpsBackendDatabases) -> Result<Vec<Site>, SiteError> {
    let main_pool = databases.main();
    let mut main_conn = main_pool
      .acquire()
      .await
      .map_err(SiteError::DatabaseError)?;

    GetAllSitesService::run(&mut main_conn).await
  }
}
