use axum::{http::StatusCode, routing::get, Router};
use dps_backend::services::{LogShutdownStartService, WaitForShutdownSignalService};
use dps_backend::DpsBackend;
use dps_config::DpsConfig;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Load environment variables from .env file for local development
  dotenvy::dotenv().ok();

  // Print DPS_ environment variables for debugging
  println!("=== DPS Configuration Variables ===");
  for (key, value) in std::env::vars() {
    if key.starts_with("DPS_") {
      println!("{key}: {value}");
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

  // The host owns the root, health check, and fallback routes.
  let host = Router::new()
    .route("/", get(|| async { "OK" }))
    .route("/health", get(|| async { "OK" }))
    .fallback(not_found);

  // Enhance the host router with the backend's `{api_path}` routes.
  let app = backend.enhance(host).await?;

  let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
  println!("Server running on http://0.0.0.0:{port}");
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  Ok(())
}

async fn not_found() -> (StatusCode, &'static str) {
  (StatusCode::NOT_FOUND, "NOT FOUND")
}

async fn shutdown_signal() {
  let signal = WaitForShutdownSignalService::run().await;
  LogShutdownStartService::run(signal);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_not_found_handler_returns_404() {
    let (status, message) = not_found().await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(message, "NOT FOUND");
  }
}
