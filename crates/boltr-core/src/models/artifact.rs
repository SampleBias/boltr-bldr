//! Artifact models — manifest.json and NPZ result files

use serde::{Deserialize, Serialize};

/// manifest.json structure for packaging metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest schema version
    pub version: String,
    /// Unique package ID
    pub package_id: String,
    /// Timestamp of package creation
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Source IDs that contributed to this package
    pub sources: Vec<ManifestSource>,
    /// Files included in this package
    pub files: Vec<ManifestFile>,
    /// Total size of all files in bytes
    pub total_size_bytes: u64,
    /// Pipeline version that produced this
    pub pipeline_version: String,
    /// Optional tags for categorization
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Source reference in a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSource {
    pub database: String,
    pub id: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

/// File entry in a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Relative path within the package
    pub path: String,
    /// File type ("yaml", "npz", "pdb", "fasta", "json")
    pub file_type: String,
    /// SHA-256 hash
    pub sha256: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// NPZ file header information (metadata, not full array data)
/// NPZ is a NumPy compressed archive format. We store metadata
/// about the arrays within.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpzMetadata {
    /// Name of the .npz file
    pub filename: String,
    /// SHA-256 hash of the file
    pub sha256: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Array names within the NPZ archive
    pub arrays: Vec<NpzArrayInfo>,
}

/// Information about a single array within an NPZ file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpzArrayInfo {
    /// Array name (key in the NPZ archive)
    pub name: String,
    /// Shape of the array (e.g., "(100, 3)")
    pub shape: String,
    /// Data type (e.g., "float64", "int32")
    pub dtype: String,
}

/// An indexed artifact in the local store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedArtifact {
    /// Database row ID
    pub row_id: i64,
    /// Package ID
    pub package_id: String,
    /// File path
    pub file_path: String,
    /// File type
    pub file_type: String,
    /// SHA-256 hash
    pub sha256: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// When it was indexed
    pub indexed_at: chrono::DateTime<chrono::Utc>,
    /// Source database
    pub source_db: String,
    /// Source ID
    pub source_id: String,
}
