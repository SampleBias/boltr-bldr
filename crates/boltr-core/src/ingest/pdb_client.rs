//! RCSB PDB HTTP client
//!
//! Fetches protein structure data from the RCSB PDB Data API.
//! API docs: https://data.rcsb.org/

use crate::error::{Error, Result};
use crate::models::pdb::*;

/// Client for the RCSB PDB REST API
pub struct PdbClient {
    client: reqwest::Client,
    data_api_url: String,
}

impl PdbClient {
    /// Create a new PDB client
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("boltr-bldr/0.1.0")
            .build()?;

        Ok(Self {
            client,
            data_api_url: "https://data.rcsb.org/rest/v1/core".to_string(),
        })
    }

    /// Fetch a complete PDB entry by ID (e.g., "1ABC", "7BV2")
    pub async fn fetch_entry(&self, pdb_id: &str) -> Result<PdbEntry> {
        let pdb_id = pdb_id.to_uppercase();
        tracing::info!(pdb_id = %pdb_id, "Fetching PDB entry");

        // Fetch the entry-level info
        let entry_url = format!("{}/entry/{}", self.data_api_url, pdb_id);
        let entry_resp = self.client.get(&entry_url).send().await?;

        if entry_resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::PdbNotFound(pdb_id));
        }

        let entry_json: serde_json::Value = entry_resp.json().await?;

        // Extract basic entry fields
        let title = entry_json["struct"]["title"]
            .as_str()
            .unwrap_or("Untitled")
            .to_string();

        let deposition_date = entry_json["rcsb_accession_info"]
            .get("deposit_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let method = entry_json["exptl"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v["method"].as_str())
            .map(|s| s.to_string());

        let resolution = entry_json["rcsb_entry_info"]
            .get("resolution_combined")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_f64());

        // Fetch entity info
        let entities = self.fetch_entities(&pdb_id).await.unwrap_or_default();

        // Fetch polymer entity sequences for chain info
        let chains = self.fetch_chains(&pdb_id).await.unwrap_or_default();

        // Fetch cross-references to UniProt
        let uniprot_refs = self.fetch_uniprot_refs(&pdb_id).await.unwrap_or_default();

        Ok(PdbEntry {
            id: pdb_id.clone(),
            title,
            deposition_date,
            resolution,
            method,
            entities,
            atoms: Vec::new(), // Atoms would come from PDB file parsing; not in REST API
            chains,
            uniprot_refs,
            raw_json: Some(entry_json),
        })
    }

    /// Fetch entity information for a PDB entry
    async fn fetch_entities(&self, pdb_id: &str) -> Result<Vec<PdbEntity>> {
        let url = format!("{}/polymer_entity/{}/1", self.data_api_url, pdb_id);
        let resp = self.client.get(&url).send().await?;

        // The API returns individual entity endpoints; we'll parse the first one
        // and also try to get entity count from entry info
        let json: serde_json::Value = resp.json().await?;

        let entity = PdbEntity {
            entity_id: "1".to_string(),
            entity_type: "polymer".to_string(),
            description: json["rcsb_polymer_entity"]
                .get("pdbx_description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            sequence: json["entity_poly"]
                .get("pdbx_seq_one_letter_code_can")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            length: json["entity_poly"]
                .get("pdbx_strand_id")
                .and_then(|v| v.as_str())
                .map(|_| json["rcsb_polymer_entity"]["rcsb_entity_source_organism"].as_array().map(|a| a.len() as u32).unwrap_or(0)),
        };

        Ok(vec![entity])
    }

    /// Fetch chain information
    async fn fetch_chains(&self, pdb_id: &str) -> Result<Vec<PdbChain>> {
        let url = format!("{}/assembly/{}/1", self.data_api_url, pdb_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            // Assembly endpoint might not exist; try entity
            return Ok(Vec::new());
        }

        let json: serde_json::Value = resp.json().await?;

        let chains = json["rcsb_assembly_container_identifiers"]
            .get("polymer_entity_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(i, id)| PdbChain {
                        chain_id: char::from(b'A' + (i as u8 % 26)).to_string(),
                        entity_id: id.as_str().unwrap_or("1").to_string(),
                        residue_count: 0,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(chains)
    }

    /// Fetch UniProt cross-references
    async fn fetch_uniprot_refs(&self, pdb_id: &str) -> Result<Vec<UniprotCrossRef>> {
        let url = format!("{}/uniprot/{}", self.data_api_url, pdb_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = resp.json().await?;

        let refs = json
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|xref| UniprotCrossRef {
                        accession: xref["rcsb_id"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        name: None,
                        chain_id: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(refs)
    }

    /// Fetch summary info for a PDB entry (lighter weight)
    pub async fn fetch_summary(&self, pdb_id: &str) -> Result<PdbSummary> {
        let entry = self.fetch_entry(pdb_id).await?;
        Ok(PdbSummary::from(&entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb_client_creation() {
        let client = PdbClient::new();
        assert!(client.is_ok());
    }
}
