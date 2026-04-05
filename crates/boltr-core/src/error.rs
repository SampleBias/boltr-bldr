//! Error types for Boltr Bldr

use thiserror::Error;

/// Core error type for all Boltr Bldr operations
#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    YamlSerialize(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Ingest error: {0}")]
    Ingest(String),

    #[error("Normalization error: {0}")]
    Normalize(String),

    #[error("Emission error: {0}")]
    Emit(String),

    #[error("Artifact error: {0}")]
    Artifact(String),

    #[error("PDB entry not found: {0}")]
    PdbNotFound(String),

    #[error("UniProt entry not found: {0}")]
    UniProtNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Store error: {0}")]
    Store(String),
}

/// Convenience Result alias
pub type Result<T> = std::result::Result<T, Error>;
