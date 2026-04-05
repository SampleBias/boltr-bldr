//! UniProt data models

use serde::{Deserialize, Serialize};

/// Raw UniProt entry as returned by the UniProt REST API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtEntry {
    /// UniProt accession (e.g., "P12345")
    pub accession: String,
    /// Entry name / mnemonic (e.g., "TPIS_HUMAN")
    pub entry_name: String,
    /// Protein name
    pub protein_name: String,
    /// Organism name
    pub organism: String,
    /// NCBI taxonomy ID
    pub taxonomy_id: Option<u32>,
    /// Gene names
    pub gene_names: Vec<String>,
    /// Protein sequence (amino acids)
    pub sequence: String,
    /// Sequence length
    pub sequence_length: u32,
    /// Molecular weight
    pub molecular_weight: Option<u32>,
    /// EC (Enzyme Commission) numbers
    pub ec_numbers: Vec<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Database cross-references
    pub cross_refs: Vec<CrossReference>,
    /// Features (domains, binding sites, etc.)
    pub features: Vec<UniProtFeature>,
    /// Comments (function, pathway, etc.)
    pub comments: Vec<UniProtComment>,
    /// Raw JSON from UniProt API
    pub raw_json: Option<serde_json::Value>,
}

/// A database cross-reference from UniProt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    /// Database name (e.g., "PDB", "Pfam")
    pub database: String,
    /// ID in the external database
    pub id: String,
    /// Optional properties
    pub properties: std::collections::HashMap<String, String>,
}

/// A feature annotation from UniProt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtFeature {
    /// Feature type (e.g., "DOMAIN", "BINDING", "ACT_SITE")
    pub feature_type: String,
    /// Description
    pub description: Option<String>,
    /// Start position in sequence
    pub begin: Option<u32>,
    /// End position in sequence
    pub end: Option<u32>,
}

/// A comment/annotation from UniProt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtComment {
    /// Comment type (e.g., "FUNCTION", "PATHWAY", "SUBUNIT")
    pub comment_type: String,
    /// Text content
    pub text: String,
}

/// Summary info for a UniProt entry (for listings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniProtSummary {
    pub accession: String,
    pub entry_name: String,
    pub protein_name: String,
    pub organism: String,
    pub sequence_length: u32,
    pub gene_names: Vec<String>,
}

impl From<&UniProtEntry> for UniProtSummary {
    fn from(entry: &UniProtEntry) -> Self {
        Self {
            accession: entry.accession.clone(),
            entry_name: entry.entry_name.clone(),
            protein_name: entry.protein_name.clone(),
            organism: entry.organism.clone(),
            sequence_length: entry.sequence_length,
            gene_names: entry.gene_names.clone(),
        }
    }
}
