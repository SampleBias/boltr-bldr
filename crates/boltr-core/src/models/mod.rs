//! Data models for protein data ingestion, normalization, and Boltr YAML output

pub mod pdb;
pub mod uniprot;
pub mod af3_input;
pub mod boltr;
pub mod artifact;

pub use pdb::*;
pub use uniprot::*;
pub use boltr::*;
pub use af3_input::{
    Af3Dna, Af3Id, Af3Ligand, Af3Protein, Af3Rna, Af3Template, Alphafold3Job, BuilderEntity,
    BuilderEntityKind, SequenceEntry,
};
pub use artifact::*;

use serde::{Deserialize, Serialize};

/// Universal identifier that can reference a PDB entry or UniProt accession
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SourceId {
    /// The database source (e.g., "pdb", "uniprot")
    pub source: String,
    /// The entry identifier (e.g., "1ABC", "P12345")
    pub id: String,
}

impl SourceId {
    pub fn pdb(id: impl Into<String>) -> Self {
        Self { source: "pdb".into(), id: id.into() }
    }

    pub fn uniprot(id: impl Into<String>) -> Self {
        Self { source: "uniprot".into(), id: id.into() }
    }
}

/// Status of a data record in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Raw,
    Ingested,
    Normalized,
    Emitted,
    Packaged,
    Indexed,
}

/// A timestamped record of pipeline progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRecord {
    pub id: SourceId,
    pub status: RecordStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: Option<String>,
}
