//! Boltr Bldr WebUI — local browser interface
//!
//! Exposes ingest, normalize, emit, status, and artifact indexing via a local HTTP server.
//! Full pipeline and package management remain available in the CLI.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::Router;
use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod handlers;
mod routes;

#[derive(Parser)]
#[command(name = "boltr-web")]
#[command(version, about = "Boltr Bldr WebUI — local browser interface")]
struct WebCli {
    /// Port to listen on
    #[arg(long, default_value = "8081", env = "BOLTR_WEB_PORT")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1", env = "BOLTR_WEB_HOST")]
    host: String,

    /// Base data directory
    #[arg(long, default_value = "data", env = "BOLTR_DATA_DIR")]
    data_dir: PathBuf,

    /// Log level
    #[arg(long, default_value = "info", env = "BOLTR_LOG_LEVEL")]
    log_level: String,
}

/// Shared application state
pub struct AppState {
    pub data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = WebCli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .init();

    let state = Arc::new(AppState {
        data_dir: cli.data_dir,
    });

    // Build the static files dir relative to the binary
    let static_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("crates/boltr-web/static");

    let app = Router::new()
        .merge(routes::api_router(state.clone()))
        .fallback_service(ServeDir::new(static_dir))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(CorsLayer::permissive());

    let addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🌐 Boltr Bldr WebUI starting at http://{}", addr);
    println!("🌐 Boltr Bldr WebUI");
    println!("   URL: http://{}", addr);
    println!("   Data dir: {}", state.data_dir.display());
    println!("   Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}
