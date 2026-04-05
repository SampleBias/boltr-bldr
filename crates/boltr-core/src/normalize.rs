//! Normalization pipeline
//!
//! Converts raw PDB and UniProt entries into a unified intermediate representation
//! that can be emitted as Boltr-compatible YAML.

use crate::error::{Error, Result};
use crate::models::pdb::PdbEntry;
use crate::models::uniprot::UniProtEntry;

/// A normalized protein record — the unified intermediate representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NormalizedRecord {
    /// Unique record ID (generated)
    pub id: String,
    /// Source PDB entry (if this record came from PDB)
    pub pdb: Option<PdbEntry>,
    /// Source UniProt entry (if this record came from UniProt)
    pub uniprot: Option<UniProtEntry>,
}

impl NormalizedRecord {
    /// Create a new normalized record with a UUID
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pdb: None,
            uniprot: None,
        }
    }
}

/// Normalize a PDB entry into a NormalizedRecord
pub fn normalize_pdb(entry: PdbEntry) -> Result<NormalizedRecord> {
    tracing::info!(pdb_id = %entry.id, "Normalizing PDB entry");

    if entry.id.is_empty() {
        return Err(Error::Normalize("PDB entry has empty ID".into()));
    }

    let mut record = NormalizedRecord::new();
    record.pdb = Some(entry);

    tracing::info!(record_id = %record.id, "PDB entry normalized");
    Ok(record)
}

/// Normalize a UniProt entry into a NormalizedRecord
pub fn normalize_uniprot(entry: UniProtEntry) -> Result<NormalizedRecord> {
    tracing::info!(accession = %entry.accession, "Normalizing UniProt entry");

    if entry.accession.is_empty() {
        return Err(Error::Normalize("UniProt entry has empty accession".into()));
    }

    if entry.sequence.is_empty() {
        tracing::warn!(accession = %entry.accession, "UniProt entry has empty sequence");
    }

    let mut record = NormalizedRecord::new();
    record.uniprot = Some(entry);

    tracing::info!(record_id = %record.id, "UniProt entry normalized");
    Ok(record)
}

/// Merge a PDB-derived and UniProt-derived record into a single normalized record
pub fn normalize_merged(pdb_entry: PdbEntry, uniprot_entry: UniProtEntry) -> Result<NormalizedRecord> {
    tracing::info!(
        pdb_id = %pdb_entry.id,
        accession = %uniprot_entry.accession,
        "Normalizing merged PDB+UniProt entries"
    );

    let mut record = NormalizedRecord::new();
    record.pdb = Some(pdb_entry);
    record.uniprot = Some(uniprot_entry);

    tracing::info!(record_id = %record.id, "Merged entry normalized");
    Ok(record)
}

/// Normalize a batch of records from ingest results
pub fn normalize_batch(
    pdb_entries: Vec<PdbEntry>,
    uniprot_entries: Vec<UniProtEntry>,
) -> Result<Vec<NormalizedRecord>> {
    let mut records = Vec::new();

    // Try to match PDB entries with UniProt entries via cross-references
    let mut matched_uniprot: std::collections::HashSet<String> = std::collections::HashSet::new();

    for pdb_entry in &pdb_entries {
        // Find matching UniProt entry via cross-references
        let matching_uniprot = pdb_entry
            .uniprot_refs
            .iter()
            .find_map(|xref| {
                uniprot_entries
                    .iter()
                    .find(|u| u.accession == xref.accession)
            })
            .cloned();

        if let Some(uniprot) = matching_uniprot {
            matched_uniprot.insert(uniprot.accession.clone());
            records.push(normalize_merged(pdb_entry.clone(), uniprot)?);
        } else {
            records.push(normalize_pdb(pdb_entry.clone())?);
        }
    }

    // Add any unmatched UniProt entries
    for uniprot_entry in uniprot_entries {
        if !matched_uniprot.contains(&uniprot_entry.accession) {
            records.push(normalize_uniprot(uniprot_entry)?);
        }
    }

    tracing::info!(count = records.len(), "Batch normalization complete");
    Ok(records)
}

/// Validate a normalized record
pub fn validate_record(record: &NormalizedRecord) -> Result<()> {
    if record.pdb.is_none() && record.uniprot.is_none() {
        return Err(Error::Validation(
            "Normalized record has neither PDB nor UniProt data".into(),
        ));
    }

    if let Some(ref pdb) = record.pdb {
        if pdb.id.is_empty() {
            return Err(Error::Validation("PDB entry has empty ID".into()));
        }
    }

    if let Some(ref uniprot) = record.uniprot {
        if uniprot.accession.is_empty() {
            return Err(Error::Validation("UniProt entry has empty accession".into()));
        }
    }

    Ok(())
}
