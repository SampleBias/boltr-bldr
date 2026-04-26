//! YAML emission — converts normalized records into Boltr-compatible YAML
//!
//! YAML is the canonical input format for the Boltr pipeline.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::models::af3_input::{
    first_protein_sequence, sequences_from_builder_entities, Alphafold3Job, BuilderEntity,
};
use crate::models::boltr::*;
use crate::normalize::NormalizedRecord;

/// Options for YAML emission
#[derive(Debug, Clone)]
pub struct EmitOptions {
    /// Output directory for YAML files
    pub output_dir: PathBuf,
    /// Schema version to use in emitted documents
    pub version: String,
    /// Whether to include raw JSON as comments
    pub include_raw: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("data/boltr"),
            version: "1.0.0".to_string(),
            include_raw: false,
        }
    }
}

/// Result of emitting a single record
#[derive(Debug)]
pub struct EmittedFile {
    /// Path to the emitted YAML file
    pub path: PathBuf,
    /// SHA-256 hash of the file contents
    pub sha256: String,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Emit a single normalized record as a Boltr-compatible YAML file
pub fn emit_record(record: &NormalizedRecord, opts: &EmitOptions) -> Result<EmittedFile> {
    let doc = build_boltr_document(record, &opts.version)?;
    validate_boltr_document(&doc)?;

    let yaml_str = serde_yaml::to_string(&doc)?;

    // Determine filename
    let filename = build_filename(record);
    let output_path = opts.output_dir.join(&filename);

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write file
    std::fs::write(&output_path, &yaml_str)?;

    let sha256 = compute_sha256(&yaml_str);
    let size_bytes = yaml_str.len() as u64;

    tracing::info!(
        path = %output_path.display(),
        sha256 = %sha256[..16],
        size = size_bytes,
        "Emitted Boltr YAML"
    );

    Ok(EmittedFile {
        path: output_path,
        sha256,
        size_bytes,
    })
}

/// Emit a batch of normalized records
pub fn emit_batch(records: &[NormalizedRecord], opts: &EmitOptions) -> Result<Vec<EmittedFile>> {
    std::fs::create_dir_all(&opts.output_dir)?;

    let mut emitted = Vec::with_capacity(records.len());
    for record in records {
        emitted.push(emit_record(record, opts)?);
    }

    tracing::info!(count = emitted.len(), "Batch emission complete");
    Ok(emitted)
}

/// Build a BoltrDocument from a NormalizedRecord
fn build_boltr_document(record: &NormalizedRecord, version: &str) -> Result<BoltrDocument> {
    let mut sources = Vec::new();
    let mut protein = BoltrProtein {
        name: "Unknown".to_string(),
        organism: "Unknown".to_string(),
        gene_names: Vec::new(),
        ec_numbers: Vec::new(),
    };
    let mut structure: Option<BoltrStructure> = None;
    let mut sequence = BoltrSequence {
        sequence: String::new(),
        length: 0,
        molecular_weight: None,
        features: Vec::new(),
    };
    let mut annotations: Option<BoltrAnnotations> = None;

    // Populate from UniProt data (primary source for sequence/annotations)
    if let Some(ref up) = record.uniprot {
        sources.push(BoltrSource {
            database: "uniprot".to_string(),
            id: up.accession.clone(),
            url: Some(format!(
                "https://www.uniprot.org/uniprotkb/{}",
                up.accession
            )),
        });

        protein.name = up.protein_name.clone();
        protein.organism = up.organism.clone();
        protein.gene_names = up.gene_names.clone();
        protein.ec_numbers = up.ec_numbers.clone();

        sequence.sequence = up.sequence.clone();
        sequence.length = up.sequence_length;
        sequence.molecular_weight = up.molecular_weight;

        sequence.features = up
            .features
            .iter()
            .map(|f| BoltrFeature {
                feature_type: f.feature_type.clone(),
                description: f.description.clone(),
                begin: f.begin,
                end: f.end,
            })
            .collect();

        let function_comment = up
            .comments
            .iter()
            .find(|c| c.comment_type == "FUNCTION")
            .map(|c| c.text.clone());

        let pathway_comment = up
            .comments
            .iter()
            .find(|c| c.comment_type == "PATHWAY")
            .map(|c| c.text.clone());

        let other_comments: Vec<String> = up
            .comments
            .iter()
            .filter(|c| c.comment_type != "FUNCTION" && c.comment_type != "PATHWAY")
            .map(|c| format!("{}: {}", c.comment_type, c.text))
            .collect();

        annotations = Some(BoltrAnnotations {
            function: function_comment,
            pathway: pathway_comment,
            keywords: up.keywords.clone(),
            comments: other_comments,
        });
    }

    // Populate from PDB data (primary source for structure)
    if let Some(ref pdb) = record.pdb {
        sources.push(BoltrSource {
            database: "pdb".to_string(),
            id: pdb.id.clone(),
            url: Some(format!("https://www.rcsb.org/structure/{}", pdb.id)),
        });

        // If no UniProt, use PDB title as protein name
        if record.uniprot.is_none() {
            protein.name = pdb.title.clone();
        }

        structure = Some(BoltrStructure {
            pdb_id: pdb.id.clone(),
            title: pdb.title.clone(),
            method: pdb.method.clone().unwrap_or_default(),
            resolution: pdb.resolution,
            num_chains: pdb.chains.len() as u32,
            num_atoms: pdb.atoms.len() as u32,
            chains: pdb
                .chains
                .iter()
                .map(|c| BoltrChain {
                    chain_id: c.chain_id.clone(),
                    entity_id: c.entity_id.clone(),
                    residue_count: c.residue_count,
                })
                .collect(),
            entities: pdb
                .entities
                .iter()
                .map(|e| BoltrEntity {
                    entity_id: e.entity_id.clone(),
                    description: e.description.clone(),
                    length: e.length,
                })
                .collect(),
        });

        // If no UniProt sequence, try to get it from PDB entities
        if sequence.sequence.is_empty() {
            if let Some(entity) = pdb.entities.first() {
                if let Some(ref seq) = entity.sequence {
                    sequence.sequence = seq.clone();
                    sequence.length = seq.len() as u32;
                }
            }
        }
    }

    Ok(BoltrDocument {
        version: version.to_string(),
        id: record.id.clone(),
        generated_at: chrono::Utc::now(),
        sources,
        protein,
        structure,
        sequence,
        annotations,
        parameters: None,
        artifacts: None,
        af3_input: None,
    })
}

/// Validate a BoltrDocument before emission
fn validate_boltr_document(doc: &BoltrDocument) -> Result<()> {
    if doc.version.is_empty() {
        return Err(Error::Emit("Document version is empty".into()));
    }
    if doc.id.is_empty() {
        return Err(Error::Emit("Document ID is empty".into()));
    }
    if doc.sources.is_empty() {
        return Err(Error::Emit("Document has no sources".into()));
    }
    if doc.protein.name.is_empty() {
        return Err(Error::Emit("Protein name is empty".into()));
    }
    Ok(())
}

fn sanitize_manual_job_id(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if t.is_empty() {
        "job".to_string()
    } else {
        t
    }
}

/// Build a [`BoltrDocument`] from manual job builder entities and write YAML (AlphaFold-style job).
pub fn build_boltr_document_with_af3(
    schema_version: &str,
    job_name: &str,
    model_seeds: Vec<i64>,
    entities: &[BuilderEntity],
) -> std::result::Result<BoltrDocument, String> {
    let sequences = sequences_from_builder_entities(entities)?;
    let af3 = Alphafold3Job::new(job_name.to_string(), model_seeds, sequences.clone());

    let display_name = if job_name.trim().is_empty() {
        "manual-job".to_string()
    } else {
        job_name.trim().to_string()
    };

    let first_aa = first_protein_sequence(&sequences);
    let seq_str = first_aa.clone().unwrap_or_default();
    let seq_len = seq_str.len() as u32;

    Ok(BoltrDocument {
        version: schema_version.to_string(),
        id: uuid::Uuid::new_v4().to_string(),
        generated_at: chrono::Utc::now(),
        sources: vec![BoltrSource {
            database: "manual".to_string(),
            id: sanitize_manual_job_id(&display_name),
            url: None,
        }],
        protein: BoltrProtein {
            name: display_name.clone(),
            organism: "Unknown".to_string(),
            gene_names: Vec::new(),
            ec_numbers: Vec::new(),
        },
        structure: None,
        sequence: BoltrSequence {
            sequence: seq_str,
            length: seq_len,
            molecular_weight: None,
            features: Vec::new(),
        },
        annotations: None,
        parameters: None,
        artifacts: None,
        af3_input: Some(af3),
    })
}

/// Emit a manual AF3-style job to `.boltr.yaml` under `output_dir`.
pub fn emit_af3_job(
    schema_version: &str,
    job_name: &str,
    model_seeds: Vec<i64>,
    entities: &[BuilderEntity],
    output_dir: &std::path::Path,
) -> Result<EmittedFile> {
    let doc = build_boltr_document_with_af3(schema_version, job_name, model_seeds, entities)
        .map_err(Error::Emit)?;
    validate_boltr_document(&doc)?;

    let yaml_str = serde_yaml::to_string(&doc)?;

    let safe = sanitize_manual_job_id(job_name);
    let short = &uuid::Uuid::new_v4().to_string()[..8];
    let filename = format!("{}_{}.boltr.yaml", safe, short);
    let output_path = output_dir.join(&filename);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&output_path, &yaml_str)?;

    let sha256 = compute_sha256(&yaml_str);
    let size_bytes = yaml_str.len() as u64;

    tracing::info!(
        path = %output_path.display(),
        sha256 = %sha256[..16],
        size = size_bytes,
        "Emitted AF3 Boltr YAML"
    );

    Ok(EmittedFile {
        path: output_path,
        sha256,
        size_bytes,
    })
}

/// Build a descriptive filename for the YAML output
fn build_filename(record: &NormalizedRecord) -> String {
    let mut parts = Vec::new();

    if let Some(ref pdb) = record.pdb {
        parts.push(pdb.id.to_lowercase());
    }
    if let Some(ref up) = record.uniprot {
        parts.push(up.accession.to_lowercase());
    }

    if parts.is_empty() {
        parts.push(record.id.clone());
    }

    format!("{}.boltr.yaml", parts.join("_"))
}

/// Compute SHA-256 hash of a string
fn compute_sha256(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

/// Parse a Boltr YAML file back into a BoltrDocument (for round-trip validation)
pub fn parse_yaml_file(path: &Path) -> Result<BoltrDocument> {
    let content = std::fs::read_to_string(path)?;
    let doc: BoltrDocument = serde_yaml::from_str(&content)?;
    validate_boltr_document(&doc)?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::af3_input::{BuilderEntity, BuilderEntityKind};

    #[test]
    fn test_emit_options_default() {
        let opts = EmitOptions::default();
        assert_eq!(opts.version, "1.0.0");
        assert!(!opts.include_raw);
    }

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256("hello");
        assert_eq!(hash.len(), 64); // SHA-256 is 64 hex chars
    }

    #[test]
    fn test_build_boltr_document_with_af3_roundtrip() {
        let entities = vec![
            BuilderEntity {
                kind: BuilderEntityKind::Protein,
                copies: 1,
                sequence: "ACDEFGHIK".into(),
                smiles: String::new(),
                ccd_codes: vec![],
                mmcif_path: None,
                pdb_path: None,
                description: None,
            },
            BuilderEntity {
                kind: BuilderEntityKind::Ligand,
                copies: 1,
                sequence: String::new(),
                smiles: String::new(),
                ccd_codes: vec!["ATP".into()],
                mmcif_path: None,
                pdb_path: None,
                description: None,
            },
        ];
        let doc = build_boltr_document_with_af3("1.0.0", "test-job", vec![1], &entities).unwrap();
        assert!(doc.af3_input.is_some());
        let yaml = serde_yaml::to_string(&doc).unwrap();
        let parsed: BoltrDocument = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.version, doc.version);
        assert_eq!(parsed.af3_input.as_ref().unwrap().name, "test-job");
    }

    #[test]
    fn test_emit_af3_job_writes_file() {
        let tmp = std::env::temp_dir().join(format!("boltr_af3_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let entities = vec![BuilderEntity {
            kind: BuilderEntityKind::Protein,
            copies: 1,
            sequence: "MVLSP".into(),
            smiles: String::new(),
            ccd_codes: vec![],
            mmcif_path: None,
            pdb_path: None,
            description: None,
        }];
        let emitted = emit_af3_job("1.0.0", "emit_test", vec![42], &entities, &tmp).unwrap();
        assert!(emitted.path.exists());
        let content = std::fs::read_to_string(&emitted.path).unwrap();
        assert!(content.contains("alphafold3"));
        assert!(content.contains("modelSeeds"));
        let _doc: BoltrDocument = serde_yaml::from_str(&content).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
