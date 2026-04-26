//! Shared full-pipeline orchestration for CLI and WebUI callers.

use std::path::PathBuf;

use crate::artifact::ArtifactManager;
use crate::emit::{emit_batch, EmitOptions, EmittedFile};
use crate::error::Result;
use crate::ingest::ingest_sources;
use crate::models::artifact::Manifest;
use crate::normalize::normalize_batch;
use crate::store::{Store, StoreStats};

#[derive(Debug, Clone)]
pub struct PipelineOptions {
    pub data_dir: PathBuf,
    pub output_dir: PathBuf,
    pub package_dir: PathBuf,
    pub pdb_ids: Vec<String>,
    pub uniprot_accessions: Vec<String>,
    pub version: String,
    pub package_description: Option<String>,
    pub package_tags: Vec<String>,
    pub index: bool,
}

#[derive(Debug)]
pub struct PipelineOutput {
    pub records_normalized: usize,
    pub emitted: Vec<EmittedFile>,
    pub package_id: String,
    pub package_path: PathBuf,
    pub manifest: Manifest,
    pub indexed: bool,
    pub stats: Option<StoreStats>,
}

pub async fn run(options: PipelineOptions) -> Result<PipelineOutput> {
    let ingest_result = ingest_sources(&options.pdb_ids, &options.uniprot_accessions).await?;

    let records = normalize_batch(ingest_result.pdb_entries, ingest_result.uniprot_entries)?;
    let records_normalized = records.len();

    let emit_opts = EmitOptions {
        output_dir: options.output_dir.clone(),
        version: options.version,
        include_raw: false,
    };
    let emitted = emit_batch(&records, &emit_opts)?;

    let package_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let manager = ArtifactManager::new(&options.package_dir);
    let (package_path, manifest) = manager.package(
        &package_id,
        &options.output_dir,
        options.package_description,
        options.package_tags,
    )?;

    let stats = if options.index {
        let store = Store::open(&options.data_dir)?;
        store.index_manifest(&manifest, &package_path)?;
        Some(store.stats()?)
    } else {
        None
    };

    Ok(PipelineOutput {
        records_normalized,
        emitted,
        package_id,
        package_path,
        manifest,
        indexed: options.index,
        stats,
    })
}
