//! Data ingestion module — PDB and UniProt clients, plus orchestration

pub mod pdb_client;
pub mod uniprot_client;

pub use pdb_client::PdbClient;
pub use uniprot_client::UniProtClient;

use crate::error::Result;
use crate::models::pdb::PdbEntry;
use crate::models::uniprot::UniProtEntry;

/// Result of ingesting data from all configured sources
#[derive(Debug)]
pub struct IngestResult {
    /// PDB entries fetched (if any PDB IDs were requested)
    pub pdb_entries: Vec<PdbEntry>,
    /// UniProt entries fetched (if any accessions were requested)
    pub uniprot_entries: Vec<UniProtEntry>,
}

impl IngestResult {
    pub fn new() -> Self {
        Self {
            pdb_entries: Vec::new(),
            uniprot_entries: Vec::new(),
        }
    }

    pub fn total_count(&self) -> usize {
        self.pdb_entries.len() + self.uniprot_entries.len()
    }
}

/// Ingest data from specified PDB IDs and UniProt accessions
pub async fn ingest_sources(
    pdb_ids: &[String],
    uniprot_accessions: &[String],
) -> Result<IngestResult> {
    let mut result = IngestResult::new();

    // Fetch PDB entries
    if !pdb_ids.is_empty() {
        let pdb_client = PdbClient::new()?;
        for pdb_id in pdb_ids {
            tracing::info!("Ingesting PDB: {}", pdb_id);
            match pdb_client.fetch_entry(pdb_id).await {
                Ok(entry) => {
                    tracing::info!(id = %entry.id, atoms = entry.atoms.len(), "PDB entry fetched");
                    result.pdb_entries.push(entry);
                }
                Err(e) => {
                    tracing::error!(id = %pdb_id, error = %e, "Failed to fetch PDB entry");
                    return Err(e);
                }
            }
        }
    }

    // Fetch UniProt entries
    if !uniprot_accessions.is_empty() {
        let uniprot_client = UniProtClient::new()?;
        for accession in uniprot_accessions {
            tracing::info!("Ingesting UniProt: {}", accession);
            match uniprot_client.fetch_entry(accession).await {
                Ok(entry) => {
                    tracing::info!(
                        accession = %entry.accession,
                        length = entry.sequence_length,
                        "UniProt entry fetched"
                    );
                    result.uniprot_entries.push(entry);
                }
                Err(e) => {
                    tracing::error!(accession = %accession, error = %e, "Failed to fetch UniProt entry");
                    return Err(e);
                }
            }
        }
    }

    tracing::info!(
        pdb = result.pdb_entries.len(),
        uniprot = result.uniprot_entries.len(),
        "Ingest complete"
    );

    Ok(result)
}
