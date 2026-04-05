//! API route definitions

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers;
use crate::AppState;

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Status / dashboard
        .route("/api/status", get(handlers::get_status))
        .route("/api/stats", get(handlers::get_stats))
        // Ingest
        .route("/api/ingest", post(handlers::ingest))
        // Normalize
        .route("/api/normalize", post(handlers::normalize))
        // Emit
        .route("/api/emit", post(handlers::emit))
        // Package
        .route("/api/package", post(handlers::package))
        // Index
        .route("/api/index", post(handlers::index_artifacts))
        // Full pipeline
        .route("/api/pipeline", post(handlers::run_pipeline))
        // List artifacts
        .route("/api/artifacts", get(handlers::list_artifacts))
        // List packages
        .route("/api/packages", get(handlers::list_packages))
        // AlphaFold-style job YAML + structure upload
        .route("/api/job-yaml", post(handlers::post_job_yaml))
        .route("/api/upload-structure", post(handlers::upload_structure))
        .with_state(state)
}
