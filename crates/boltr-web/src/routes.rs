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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        let dir = std::env::temp_dir().join(format!("boltr-web-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Arc::new(AppState { data_dir: dir })
    }

    #[tokio::test]
    async fn artifacts_route_returns_empty_page() {
        let app = api_router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/artifacts?limit=5&offset=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("\"success\":true"));
        assert!(body.contains("\"artifacts\":[]"));
    }

    #[tokio::test]
    async fn normalize_rejects_parent_dir_paths() {
        let app = api_router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/normalize")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"input":"../outside","output":"normalized"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("\"success\":false"));
        assert!(body.contains("Path cannot contain"));
    }
}
