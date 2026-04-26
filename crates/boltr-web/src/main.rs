//! Boltr Bldr WebUI — local browser interface
//!
//! Exposes ingest, normalize, emit, package, pipeline, and artifact indexing via a local HTTP server.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method};
use axum::Router;
use clap::Parser;
use tower_http::cors::{AllowOrigin, CorsLayer};
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
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .init();

    let state = Arc::new(AppState {
        data_dir: cli.data_dir,
    });

    if !is_loopback_host(&cli.host)
        && std::env::var("BOLTR_WEB_ALLOW_REMOTE").as_deref() != Ok("true")
    {
        anyhow::bail!(
            "Refusing to bind Boltr WebUI to non-loopback host '{}'. Set BOLTR_WEB_ALLOW_REMOTE=true to opt in.",
            cli.host
        );
    }

    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    let allowed_origins = [
        format!("http://127.0.0.1:{}", cli.port),
        format!("http://localhost:{}", cli.port),
    ]
    .into_iter()
    .filter_map(|origin| origin.parse::<HeaderValue>().ok())
    .collect::<Vec<_>>();

    let app = Router::new()
        .merge(routes::api_router(state.clone()))
        .fallback_service(ServeDir::new(static_dir))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        );

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

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}
