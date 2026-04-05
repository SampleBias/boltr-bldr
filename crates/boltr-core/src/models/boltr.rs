//! Boltr-compatible YAML schema models
//!
//! YAML is the canonical input format for the Boltr pipeline.
//! These models represent the structure of Boltr-compatible YAML files.

use serde::{Deserialize, Serialize};

/// The top-level Boltr YAML document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrDocument {
    /// Schema version
    pub version: String,
    /// Unique identifier for this document
    pub id: String,
    /// Timestamp when this document was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Source identifiers
    pub sources: Vec<BoltrSource>,
    /// Protein information
    pub protein: BoltrProtein,
    /// Structure information (from PDB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure: Option<BoltrStructure>,
    /// Sequence and features
    pub sequence: BoltrSequence,
    /// Annotations and metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<BoltrAnnotations>,
    /// Processing parameters / pipeline config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_yaml::Mapping>,
    /// Associated artifacts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<BoltrArtifactRef>>,
}

/// Source database reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrSource {
    /// Database name ("pdb" or "uniprot")
    pub database: String,
    /// Entry identifier
    pub id: String,
    /// URL to the original data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Protein information in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrProtein {
    /// Protein name
    pub name: String,
    /// Organism
    pub organism: String,
    /// Gene names
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gene_names: Vec<String>,
    /// EC numbers
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ec_numbers: Vec<String>,
}

/// Structure information derived from PDB data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrStructure {
    /// PDB ID
    pub pdb_id: String,
    /// Title of the structure
    pub title: String,
    /// Experimental method
    pub method: String,
    /// Resolution in Angstroms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<f64>,
    /// Number of chains
    pub num_chains: u32,
    /// Number of atoms
    pub num_atoms: u32,
    /// Chain summaries
    pub chains: Vec<BoltrChain>,
    /// Entity summaries
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<BoltrEntity>,
}

/// Chain summary in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrChain {
    pub chain_id: String,
    pub entity_id: String,
    pub residue_count: u32,
}

/// Entity summary in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrEntity {
    pub entity_id: String,
    pub description: Option<String>,
    pub length: Option<u32>,
}

/// Sequence data in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrSequence {
    /// Amino acid sequence
    pub sequence: String,
    /// Sequence length
    pub length: u32,
    /// Molecular weight in Daltons
    #[serde(skip_serializing_if = "Option::is_none")]
    pub molecular_weight: Option<u32>,
    /// Domain features
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<BoltrFeature>,
}

/// A feature annotation in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrFeature {
    pub feature_type: String,
    pub description: Option<String>,
    pub begin: Option<u32>,
    pub end: Option<u32>,
}

/// Annotations in the Boltr schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrAnnotations {
    /// Functional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// Pathway information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pathway: Option<String>,
    /// Keywords
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Additional comments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

/// Reference to an associated artifact file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoltrArtifactRef {
    /// Artifact type ("manifest", "npz", "pdb_file", "fasta")
    pub artifact_type: String,
    /// Relative file path
    pub path: String,
    /// SHA-256 hash of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}
