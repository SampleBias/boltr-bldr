//! PDB (Protein Data Bank) data models

use serde::{Deserialize, Serialize};

/// Raw PDB entry as returned by the RCSB PDB REST API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbEntry {
    /// PDB ID (e.g., "1ABC")
    pub id: String,
    /// Structure title
    pub title: String,
    /// Deposition date
    pub deposition_date: Option<String>,
    /// Resolution in Angstroms
    pub resolution: Option<f64>,
    /// Experimental method (e.g., "X-RAY DIFFRACTION")
    pub method: Option<String>,
    /// List of polymer entities (proteins, nucleic acids)
    pub entities: Vec<PdbEntity>,
    /// List of resolved atoms
    pub atoms: Vec<PdbAtom>,
    /// Chain identifiers
    pub chains: Vec<PdbChain>,
    /// Associated UniProt cross-references
    pub uniprot_refs: Vec<UniprotCrossRef>,
    /// Raw JSON from RCSB API (for passthrough)
    pub raw_json: Option<serde_json::Value>,
}

/// A polymer entity within a PDB structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbEntity {
    /// Entity ID
    pub entity_id: String,
    /// Entity type (e.g., "polymer")
    pub entity_type: String,
    /// Description / name
    pub description: Option<String>,
    /// Polymer description (sequence)
    pub sequence: Option<String>,
    /// Length of the polymer
    pub length: Option<u32>,
}

/// A single atom from a PDB structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbAtom {
    /// Atom serial number
    pub serial: u32,
    /// Atom name (e.g., "CA" for C-alpha)
    pub name: String,
    /// Residue name (3-letter code)
    pub residue_name: String,
    /// Residue sequence number
    pub residue_seq: i32,
    /// Chain identifier
    pub chain_id: String,
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Z coordinate
    pub z: f64,
    /// Occupancy
    pub occupancy: Option<f64>,
    /// Temperature factor (B-factor)
    pub b_factor: Option<f64>,
    /// Element symbol
    pub element: Option<String>,
}

/// A chain within a PDB structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbChain {
    /// Chain identifier (single letter)
    pub chain_id: String,
    /// Entity ID this chain belongs to
    pub entity_id: String,
    /// Number of residues
    pub residue_count: u32,
}

/// Cross-reference from PDB to UniProt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniprotCrossRef {
    /// UniProt accession (e.g., "P12345")
    pub accession: String,
    /// UniProt entry name (e.g., "TPIS_HUMAN")
    pub name: Option<String>,
    /// Chain ID in the PDB structure
    pub chain_id: Option<String>,
}

/// Summary info for a PDB entry (for listings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbSummary {
    pub id: String,
    pub title: String,
    pub resolution: Option<f64>,
    pub method: Option<String>,
    pub num_chains: usize,
    pub num_atoms: usize,
    pub deposition_date: Option<String>,
}

impl From<&PdbEntry> for PdbSummary {
    fn from(entry: &PdbEntry) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.title.clone(),
            resolution: entry.resolution,
            method: entry.method.clone(),
            num_chains: entry.chains.len(),
            num_atoms: entry.atoms.len(),
            deposition_date: entry.deposition_date.clone(),
        }
    }
}
