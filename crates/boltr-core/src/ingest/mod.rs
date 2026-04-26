//! Data ingestion module — PDB and UniProt clients, plus orchestration

pub mod pdb_client;
pub mod uniprot_client;

pub use pdb_client::PdbClient;
pub use uniprot_client::UniProtClient;

use crate::error::Result;
use crate::models::pdb::PdbEntry;
use crate::models::uniprot::UniProtEntry;

const MAX_CONCURRENT_FETCHES: usize = 6;

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

impl Default for IngestResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Ingest data from specified PDB IDs and UniProt accessions
pub async fn ingest_sources(
    pdb_ids: &[String],
    uniprot_accessions: &[String],
) -> Result<IngestResult> {
    let mut result = IngestResult::new();

    if !pdb_ids.is_empty() {
        let pdb_client = PdbClient::new()?;
        result.pdb_entries = fetch_pdb_entries(&pdb_client, pdb_ids).await?;
    }

    if !uniprot_accessions.is_empty() {
        let uniprot_client = UniProtClient::new()?;
        result.uniprot_entries = fetch_uniprot_entries(&uniprot_client, uniprot_accessions).await?;
    }

    tracing::info!(
        pdb = result.pdb_entries.len(),
        uniprot = result.uniprot_entries.len(),
        "Ingest complete"
    );

    Ok(result)
}

async fn fetch_pdb_entries(client: &PdbClient, pdb_ids: &[String]) -> Result<Vec<PdbEntry>> {
    let mut entries = Vec::with_capacity(pdb_ids.len());

    for chunk in pdb_ids.chunks(MAX_CONCURRENT_FETCHES) {
        let mut tasks = tokio::task::JoinSet::new();

        for (idx, pdb_id) in chunk.iter().enumerate() {
            let client = client.clone();
            let pdb_id = pdb_id.clone();
            tasks.spawn(async move {
                tracing::info!("Ingesting PDB: {}", pdb_id);
                let entry = client
                    .fetch_entry(&pdb_id)
                    .await
                    .map_err(|e| (pdb_id.clone(), e))?;
                Ok::<_, (String, crate::Error)>((idx, entry))
            });
        }

        let mut chunk_entries = Vec::with_capacity(chunk.len());
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(Ok((idx, entry))) => {
                    tracing::info!(id = %entry.id, atoms = entry.atoms.len(), "PDB entry fetched");
                    chunk_entries.push((idx, entry));
                }
                Ok(Err((pdb_id, e))) => {
                    tracing::error!(id = %pdb_id, error = %e, "Failed to fetch PDB entry");
                    return Err(crate::Error::Ingest(format!(
                        "PDB {} failed: {}",
                        pdb_id, e
                    )));
                }
                Err(e) => return Err(crate::Error::Ingest(format!("PDB worker failed: {}", e))),
            }
        }

        chunk_entries.sort_by_key(|(idx, _)| *idx);
        entries.extend(chunk_entries.into_iter().map(|(_, entry)| entry));
    }

    Ok(entries)
}

async fn fetch_uniprot_entries(
    client: &UniProtClient,
    accessions: &[String],
) -> Result<Vec<UniProtEntry>> {
    let mut entries = Vec::with_capacity(accessions.len());

    for chunk in accessions.chunks(MAX_CONCURRENT_FETCHES) {
        let mut tasks = tokio::task::JoinSet::new();

        for (idx, accession) in chunk.iter().enumerate() {
            let client = client.clone();
            let accession = accession.clone();
            tasks.spawn(async move {
                tracing::info!("Ingesting UniProt: {}", accession);
                let entry = client
                    .fetch_entry(&accession)
                    .await
                    .map_err(|e| (accession.clone(), e))?;
                Ok::<_, (String, crate::Error)>((idx, entry))
            });
        }

        let mut chunk_entries = Vec::with_capacity(chunk.len());
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(Ok((idx, entry))) => {
                    tracing::info!(
                        accession = %entry.accession,
                        length = entry.sequence_length,
                        "UniProt entry fetched"
                    );
                    chunk_entries.push((idx, entry));
                }
                Ok(Err((accession, e))) => {
                    tracing::error!(accession = %accession, error = %e, "Failed to fetch UniProt entry");
                    return Err(crate::Error::Ingest(format!(
                        "UniProt {} failed: {}",
                        accession, e
                    )));
                }
                Err(e) => {
                    return Err(crate::Error::Ingest(format!(
                        "UniProt worker failed: {}",
                        e
                    )))
                }
            }
        }

        chunk_entries.sort_by_key(|(idx, _)| *idx);
        entries.extend(chunk_entries.into_iter().map(|(_, entry)| entry));
    }

    Ok(entries)
}
