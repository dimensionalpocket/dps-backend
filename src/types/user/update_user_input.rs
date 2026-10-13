/// High-level input type for the update user workflow
///
/// This type represents application-level input data that is:
/// - Created by the REST handler from the request body and path parameter
/// - Passed to the controller for validation and business logic
/// - Not exposed directly as an HTTP DTO
///
/// This establishes a clean pattern: the handler creates app-level types,
/// the controller accepts app-level types, and the service layer performs
/// the business logic.
#[derive(Debug)]
pub struct UpdateUserInput {
  pub id: i64,
  pub name: Option<String>,
  pub role_id: Option<i64>,
  pub password: Option<String>,
  pub password_confirmation: Option<String>,
  pub metadata_json: Option<String>,
}
