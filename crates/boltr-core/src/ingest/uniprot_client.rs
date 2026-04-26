//! UniProt HTTP client
//!
//! Fetches protein data from the UniProt REST API.
//! API docs: https://www.uniprot.org/help/api_queries

use crate::error::{Error, Result};
use crate::models::uniprot::*;

/// Client for the UniProt REST API
#[derive(Clone)]
pub struct UniProtClient {
    client: reqwest::Client,
    base_url: String,
}

impl UniProtClient {
    /// Create a new UniProt client
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("boltr-bldr/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: "https://rest.uniprot.org/uniprotkb".to_string(),
        })
    }

    /// Fetch a complete UniProt entry by accession (e.g., "P12345")
    pub async fn fetch_entry(&self, accession: &str) -> Result<UniProtEntry> {
        let accession = accession.to_uppercase();
        tracing::info!("Fetching UniProt entry: {}", accession);

        let url = format!("{}/{}.json", self.base_url, accession);
        let resp = self.client.get(&url).send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::UniProtNotFound(accession));
        }

        let json: serde_json::Value = resp.json().await?;

        self.parse_entry(&accession, &json)
    }

    /// Parse a UniProt JSON response into our model
    fn parse_entry(&self, accession: &str, json: &serde_json::Value) -> Result<UniProtEntry> {
        let entry_name = json["uniProtkbId"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string();

        let protein_name = json["proteinDescription"]["recommendedName"]["fullName"]["value"]
            .as_str()
            .unwrap_or("Unknown protein")
            .to_string();

        let organism = json["organism"]["scientificName"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        let taxonomy_id = json["organism"]["taxonId"].as_u64().map(|v| v as u32);

        let gene_names = json["genes"]
            .as_array()
            .map(|genes| {
                genes
                    .iter()
                    .filter_map(|g| g["geneName"]["value"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let sequence = json["sequence"]["value"].as_str().unwrap_or("").to_string();

        let sequence_length = json["sequence"]["length"]
            .as_u64()
            .unwrap_or(sequence.len() as u64) as u32;

        let molecular_weight = json["sequence"]["molWeight"].as_u64().map(|v| v as u32);

        let ec_numbers = json["proteinDescription"]["ecNumbers"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["value"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let keywords = json["keywords"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let cross_refs = json["uniProtKBCrossReferences"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|xref| {
                        let mut props = std::collections::HashMap::new();
                        if let Some(properties) = xref["properties"].as_array() {
                            for prop in properties {
                                if let (Some(k), Some(v)) =
                                    (prop["key"].as_str(), prop["value"].as_str())
                                {
                                    props.insert(k.to_string(), v.to_string());
                                }
                            }
                        }
                        CrossReference {
                            database: xref["database"].as_str().unwrap_or("").to_string(),
                            id: xref["id"].as_str().unwrap_or("").to_string(),
                            properties: props,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let features = json["features"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|f| UniProtFeature {
                        feature_type: f["type"].as_str().unwrap_or("").to_string(),
                        description: f["description"]["value"].as_str().map(|s| s.to_string()),
                        begin: f["location"]["start"]["value"].as_u64().map(|v| v as u32),
                        end: f["location"]["end"]["value"].as_u64().map(|v| v as u32),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let comments = json["comments"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        let comment_type = c["commentType"].as_str()?;
                        let text = c["texts"]
                            .as_array()
                            .and_then(|texts| texts.first())
                            .and_then(|t| t["value"].as_str())
                            .unwrap_or("");
                        Some(UniProtComment {
                            comment_type: comment_type.to_string(),
                            text: text.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(UniProtEntry {
            accession: accession.to_string(),
            entry_name,
            protein_name,
            organism,
            taxonomy_id,
            gene_names,
            sequence,
            sequence_length,
            molecular_weight,
            ec_numbers,
            keywords,
            cross_refs,
            features,
            comments,
            raw_json: Some(json.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniprot_client_creation() {
        let client = UniProtClient::new();
        assert!(client.is_ok());
    }
}
