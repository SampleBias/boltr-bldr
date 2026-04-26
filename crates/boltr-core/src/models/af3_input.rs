//! AlphaFold 3–compatible job input (subset) for embedding in Boltr YAML.
//!
//! Field names follow AF3 JSON conventions (camelCase). See AlphaFold 3 `docs/input.md`.

use serde::{Deserialize, Serialize};

/// Top-level AF3 job block embedded under `af3_input` in [`BoltrDocument`](super::boltr::BoltrDocument).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alphafold3Job {
    pub name: String,
    #[serde(rename = "modelSeeds")]
    pub model_seeds: Vec<i64>,
    pub sequences: Vec<SequenceEntry>,
    pub dialect: String,
    pub version: i32,
}

impl Alphafold3Job {
    pub fn new(
        name: impl Into<String>,
        model_seeds: Vec<i64>,
        sequences: Vec<SequenceEntry>,
    ) -> Self {
        Self {
            name: name.into(),
            model_seeds,
            sequences,
            dialect: "alphafold3".to_string(),
            version: 4,
        }
    }
}

/// One entry in `sequences`: exactly one of protein, dna, rna, ligand.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SequenceEntry {
    Protein { protein: Af3Protein },
    Dna { dna: Af3Dna },
    Rna { rna: Af3Rna },
    Ligand { ligand: Af3Ligand },
}

/// Chain identifier: single chain or homomer copies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Af3Id {
    Single(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Af3Protein {
    pub id: Af3Id,
    pub sequence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<Af3Template>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Af3Template {
    #[serde(rename = "mmcifPath", skip_serializing_if = "Option::is_none")]
    pub mmcif_path: Option<String>,
    #[serde(rename = "pdbPath", skip_serializing_if = "Option::is_none")]
    pub pdb_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Af3Dna {
    pub id: Af3Id,
    pub sequence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Af3Rna {
    pub id: Af3Id,
    pub sequence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Af3Ligand {
    pub id: Af3Id,
    #[serde(rename = "ccdCodes", skip_serializing_if = "Option::is_none")]
    pub ccd_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smiles: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Entity row from the job builder (WebUI / API).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderEntityKind {
    Protein,
    Dna,
    Rna,
    Ligand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuilderEntity {
    pub kind: BuilderEntityKind,
    /// Homomer copies (>=1). Chain IDs are allocated sequentially.
    #[serde(default = "default_copies")]
    pub copies: u32,
    /// Raw sequence, FASTA, or empty for ligands.
    #[serde(default)]
    pub sequence: String,
    #[serde(default)]
    pub smiles: String,
    #[serde(default)]
    pub ccd_codes: Vec<String>,
    /// Relative path from job working dir (e.g. after upload) for a custom structure template.
    #[serde(default)]
    pub mmcif_path: Option<String>,
    #[serde(default)]
    pub pdb_path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_copies() -> u32 {
    1
}

/// Convert 0-based index to Excel-style chain IDs: A, B, …, Z, AA, AB, …
pub fn chain_id_for_index(index: usize) -> String {
    let mut n = index + 1;
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

/// Strip FASTA headers and whitespace; uppercase letters for polymer.
pub fn parse_fasta_or_plain_sequence(text: &str) -> String {
    let mut seq = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('>') {
            continue;
        }
        seq.push_str(line);
    }
    seq.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

/// Build AF3 sequences and assign unique chain IDs across entities.
pub fn sequences_from_builder_entities(
    entities: &[BuilderEntity],
) -> Result<Vec<SequenceEntry>, String> {
    let mut out = Vec::new();
    let mut chain_index: usize = 0;

    for ent in entities {
        let copies = ent.copies.max(1) as usize;
        let ids: Vec<String> = (0..copies)
            .map(|i| chain_id_for_index(chain_index + i))
            .collect();
        chain_index += copies;

        let id = if ids.len() == 1 {
            Af3Id::Single(ids[0].clone())
        } else {
            Af3Id::Many(ids)
        };

        match ent.kind {
            BuilderEntityKind::Protein => {
                let sequence = parse_fasta_or_plain_sequence(&ent.sequence);
                if sequence.is_empty() && ent.mmcif_path.is_none() && ent.pdb_path.is_none() {
                    return Err("Protein entity requires sequence or structure path".into());
                }
                let mut templates = None;
                if ent.mmcif_path.is_some() || ent.pdb_path.is_some() {
                    templates = Some(vec![Af3Template {
                        mmcif_path: ent.mmcif_path.clone(),
                        pdb_path: ent.pdb_path.clone(),
                    }]);
                }
                out.push(SequenceEntry::Protein {
                    protein: Af3Protein {
                        id,
                        sequence,
                        description: ent.description.clone(),
                        templates,
                    },
                });
            }
            BuilderEntityKind::Dna => {
                let sequence = parse_fasta_or_plain_sequence(&ent.sequence);
                if sequence.is_empty() {
                    return Err("DNA entity requires sequence".into());
                }
                out.push(SequenceEntry::Dna {
                    dna: Af3Dna {
                        id,
                        sequence,
                        description: ent.description.clone(),
                    },
                });
            }
            BuilderEntityKind::Rna => {
                let sequence = parse_fasta_or_plain_sequence(&ent.sequence);
                if sequence.is_empty() {
                    return Err("RNA entity requires sequence".into());
                }
                out.push(SequenceEntry::Rna {
                    rna: Af3Rna {
                        id,
                        sequence,
                        description: ent.description.clone(),
                    },
                });
            }
            BuilderEntityKind::Ligand => {
                let smiles = ent.smiles.trim();
                let has_smiles = !smiles.is_empty();
                let has_ccd = !ent.ccd_codes.is_empty();
                if !has_smiles && !has_ccd {
                    return Err("Ligand entity requires SMILES and/or CCD codes".into());
                }
                out.push(SequenceEntry::Ligand {
                    ligand: Af3Ligand {
                        id,
                        ccd_codes: if has_ccd {
                            Some(ent.ccd_codes.clone())
                        } else {
                            None
                        },
                        smiles: if has_smiles {
                            Some(smiles.to_string())
                        } else {
                            None
                        },
                        description: ent.description.clone(),
                    },
                });
            }
        }
    }

    if out.is_empty() {
        return Err("At least one entity is required".into());
    }

    Ok(out)
}

/// First protein amino-acid sequence in the job, if any (for legacy Boltr fields).
pub fn first_protein_sequence(sequences: &[SequenceEntry]) -> Option<String> {
    for s in sequences {
        if let SequenceEntry::Protein { protein } = s {
            if !protein.sequence.is_empty() {
                return Some(protein.sequence.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_ids_excel_style() {
        assert_eq!(chain_id_for_index(0), "A");
        assert_eq!(chain_id_for_index(25), "Z");
        assert_eq!(chain_id_for_index(26), "AA");
    }

    #[test]
    fn parse_fasta_strips_header() {
        let s = parse_fasta_or_plain_sequence(">sp|P123|x\nACGT\nacgt");
        assert_eq!(s, "ACGTACGT");
    }

    #[test]
    fn serde_sequence_entry_protein() {
        let e = SequenceEntry::Protein {
            protein: Af3Protein {
                id: Af3Id::Single("A".into()),
                sequence: "AC".into(),
                description: None,
                templates: None,
            },
        };
        let yaml = serde_yaml::to_string(&e).unwrap();
        let back: SequenceEntry = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(e, back);
    }
}
