use axum::Router;
use dps_backend::services::{LogShutdownStartService, WaitForShutdownSignalService};
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Environment variable name fragments whose values must never be logged.
const SENSITIVE_KEY_FRAGMENTS: &[&str] = &["SECRET", "PASSWORD", "TOKEN", "API_KEY"];

/// Returns `true` when an environment variable's value is sensitive and must be
/// redacted from logs.
fn is_sensitive_key(key: &str) -> bool {
  let upper = key.to_ascii_uppercase();
  SENSITIVE_KEY_FRAGMENTS
    .iter()
    .any(|fragment| upper.contains(fragment))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Print DPS_ environment variables for debugging (sensitive values redacted)
  println!("=== DPS Configuration Variables ===");
  for (key, value) in std::env::vars() {
    if key.starts_with("DPS_") {
      if is_sensitive_key(&key) {
        println!("{key}: [redacted]");
      } else {
        println!("{key}: {value}");
      }
    }
  }
  println!("====================================\n");

  // Initialize logging
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dps_backend=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // DpsConfig::new() automatically loads from environment variables
  let config = DpsConfig::new();
  let port = config.get_backend_port().unwrap_or(3000);
  let backend = DpsBackend::new(config)?;

  // The host contributes only API routes; the backend owns the root.
  let host_api = Router::new();
  let app = backend.enhance_api(host_api).await?;

  let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
  println!("Server running on http://0.0.0.0:{port}");
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  Ok(())
}

async fn shutdown_signal() {
  let signal = WaitForShutdownSignalService::run().await;
  LogShutdownStartService::run(signal);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_sensitive_key_redacts_secrets() {
    assert!(is_sensitive_key("DPS_BACKEND_SESSION_SECRET"));
    assert!(is_sensitive_key("DPS_BACKEND_SESSION_PASSWORD"));
    assert!(is_sensitive_key("SOME_TOKEN"));
    assert!(is_sensitive_key("MY_API_KEY"));
  }

  #[test]
  fn test_is_sensitive_key_allows_config() {
    assert!(!is_sensitive_key("DPS_BACKEND_PORT"));
    assert!(!is_sensitive_key("DPS_API_PATH"));
    assert!(!is_sensitive_key("DPS_BACKEND_SQLITE_MAIN_FILE_PATH"));
  }
}
