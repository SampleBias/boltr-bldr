//! API request handlers — each mirrors a CLI subcommand

use std::sync::Arc;

use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

// ── Response type ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

impl ApiResponse {
    fn ok(data: serde_json::Value) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    fn err(msg: impl ToString) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()) }
    }
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngestRequest {
    pub pdb: Vec<String>,
    pub uniprot: Vec<String>,
}

#[derive(Deserialize)]
pub struct NormalizeRequest {
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Deserialize)]
pub struct EmitRequest {
    pub input: Option<String>,
    pub output: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize)]
pub struct PackageRequest {
    pub input: Option<String>,
    pub output: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PipelineRequest {
    pub pdb: Vec<String>,
    pub uniprot: Vec<String>,
    pub output: Option<String>,
}

// ── Handlers ────────────────────────────────────��───────────────────

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let store = match boltr_core::store::Store::open(&state.data_dir) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Store error: {}", e))),
    };

    let stats = match store.stats() {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Stats error: {}", e))),
    };

    Json(ApiResponse::ok(serde_json::json!({
        "status": "running",
        "data_dir": state.data_dir.to_string_lossy(),
        "stats": stats,
    })))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let store = match boltr_core::store::Store::open(&state.data_dir) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Store error: {}", e))),
    };

    let stats = match store.stats() {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Stats error: {}", e))),
    };

    Json(ApiResponse::ok(serde_json::json!({ "stats": stats })))
}

pub async fn ingest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Json<ApiResponse> {
    if req.pdb.is_empty() && req.uniprot.is_empty() {
        return Json(ApiResponse::err("No sources specified"));
    }

    match boltr_core::ingest::ingest_sources(&req.pdb, &req.uniprot).await {
        Ok(result) => {
            let output_dir = state.data_dir.join("raw");
            let _ = std::fs::create_dir_all(&output_dir);

            for entry in &result.pdb_entries {
                if let Some(ref raw) = entry.raw_json {
                    let path = output_dir.join(format!("pdb_{}.json", entry.id.to_lowercase()));
                    let _ = std::fs::write(&path, serde_json::to_string_pretty(raw).unwrap_or_default());
                }
                let path = output_dir.join(format!("parsed_pdb_{}.json", entry.id.to_lowercase()));
                let _ = std::fs::write(&path, serde_json::to_string_pretty(entry).unwrap_or_default());
            }
            for entry in &result.uniprot_entries {
                if let Some(ref raw) = entry.raw_json {
                    let path = output_dir.join(format!("uniprot_{}.json", entry.accession.to_lowercase()));
                    let _ = std::fs::write(&path, serde_json::to_string_pretty(raw).unwrap_or_default());
                }
                let path = output_dir.join(format!("parsed_uniprot_{}.json", entry.accession.to_lowercase()));
                let _ = std::fs::write(&path, serde_json::to_string_pretty(entry).unwrap_or_default());
            }

            let pdb_count = result.pdb_entries.len();
            let uniprot_count = result.uniprot_entries.len();

            Json(ApiResponse::ok(serde_json::json!({
                "ingested": true,
                "pdb_count": pdb_count,
                "uniprot_count": uniprot_count,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Ingest failed: {}", e))),
    }
}

pub async fn normalize(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NormalizeRequest>,
) -> Json<ApiResponse> {
    let input_dir = state.data_dir.join(req.input.as_deref().unwrap_or("raw"));
    let output_dir = state.data_dir.join(req.output.as_deref().unwrap_or("normalized"));
    let _ = std::fs::create_dir_all(&output_dir);

    let mut pdb_entries = Vec::new();
    let mut uniprot_entries = Vec::new();

    match std::fs::read_dir(&input_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if name.starts_with("parsed_pdb_") {
                            if let Ok(pdb) = serde_json::from_value(json) {
                                pdb_entries.push(pdb);
                            }
                        } else if name.starts_with("parsed_uniprot_") {
                            if let Ok(up) = serde_json::from_value(json) {
                                uniprot_entries.push(up);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => return Json(ApiResponse::err(format!("Cannot read input dir: {}", e))),
    }

    match boltr_core::normalize::normalize_batch(pdb_entries, uniprot_entries) {
        Ok(records) => {
            let count = records.len();
            if let Ok(encoded) = serde_json::to_vec(&records) {
                let full_path = output_dir.join("full_normalized.bincode");
                let _ = std::fs::write(&full_path, encoded);
            }

            Json(ApiResponse::ok(serde_json::json!({
                "normalized": true,
                "record_count": count,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Normalize failed: {}", e))),
    }
}

pub async fn emit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmitRequest>,
) -> Json<ApiResponse> {
    let input_dir = state.data_dir.join(req.input.as_deref().unwrap_or("normalized"));
    let output_dir = state.data_dir.join(req.output.as_deref().unwrap_or("boltr"));

    let full_path = input_dir.join("full_normalized.bincode");
    let encoded = match std::fs::read(&full_path) {
        Ok(e) => e,
        Err(e) => return Json(ApiResponse::err(format!("No normalized data: {}. Run normalize first.", e))),
    };

    let records: Vec<boltr_core::normalize::NormalizedRecord> = match serde_json::from_slice(&encoded) {
        Ok(r) => r,
        Err(e) => return Json(ApiResponse::err(format!("Parse error: {}", e))),
    };

    let opts = boltr_core::emit::EmitOptions {
        output_dir,
        version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
        include_raw: false,
    };

    match boltr_core::emit::emit_batch(&records, &opts) {
        Ok(emitted) => {
            let files: Vec<String> = emitted.iter().map(|f| f.path.display().to_string()).collect();
            Json(ApiResponse::ok(serde_json::json!({
                "emitted": true,
                "file_count": files.len(),
                "files": files,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Emit failed: {}", e))),
    }
}

pub async fn package(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PackageRequest>,
) -> Json<ApiResponse> {
    let input_dir = state.data_dir.join(req.input.as_deref().unwrap_or("boltr"));
    let output_dir = state.data_dir.join(req.output.as_deref().unwrap_or("packages"));

    let package_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let manager = boltr_core::artifact::ArtifactManager::new(&output_dir);

    match manager.package(
        &package_id,
        &input_dir,
        req.description,
        req.tags.unwrap_or_default(),
    ) {
        Ok((pkg_path, manifest)) => {
            Json(ApiResponse::ok(serde_json::json!({
                "packaged": true,
                "package_id": package_id,
                "path": pkg_path.display().to_string(),
                "file_count": manifest.files.len(),
                "total_size": manifest.total_size_bytes,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Package failed: {}", e))),
    }
}

pub async fn index_artifacts(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse> {
    let store = match boltr_core::store::Store::open(&state.data_dir) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Store error: {}", e))),
    };

    let packages_dir = state.data_dir.join("packages");
    let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);

    match manager.list_packages() {
        Ok(packages) => {
            let mut total_indexed = 0;
            for manifest in &packages {
                let pkg_path = packages_dir.join(&manifest.package_id);
                if let Ok(row_ids) = store.index_manifest(manifest, &pkg_path) {
                    total_indexed += row_ids.len();
                }
            }

            Json(ApiResponse::ok(serde_json::json!({
                "indexed": true,
                "packages": packages.len(),
                "artifacts": total_indexed,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Index failed: {}", e))),
    }
}

pub async fn run_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineRequest>,
) -> Json<ApiResponse> {
    if req.pdb.is_empty() && req.uniprot.is_empty() {
        return Json(ApiResponse::err("No sources specified"));
    }

    // Step 1: Ingest
    let ingest_result = match boltr_core::ingest::ingest_sources(&req.pdb, &req.uniprot).await {
        Ok(r) => r,
        Err(e) => return Json(ApiResponse::err(format!("Ingest failed: {}", e))),
    };

    // Step 2: Normalize
    let records = match boltr_core::normalize::normalize_batch(
        ingest_result.pdb_entries,
        ingest_result.uniprot_entries,
    ) {
        Ok(r) => r,
        Err(e) => return Json(ApiResponse::err(format!("Normalize failed: {}", e))),
    };

    // Step 3: Emit
    let output_dir = state.data_dir.join(req.output.as_deref().unwrap_or("output"));
    let emit_opts = boltr_core::emit::EmitOptions {
        output_dir: output_dir.clone(),
        version: "1.0.0".to_string(),
        include_raw: false,
    };

    let emitted = match boltr_core::emit::emit_batch(&records, &emit_opts) {
        Ok(e) => e,
        Err(e) => return Json(ApiResponse::err(format!("Emit failed: {}", e))),
    };

    // Step 4: Package
    let packages_dir = state.data_dir.join("packages");
    let package_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);

    let (pkg_path, manifest) = match manager.package(
        &package_id,
        &output_dir,
        Some("Pipeline run via WebUI".to_string()),
        vec!["pipeline".to_string()],
    ) {
        Ok(p) => p,
        Err(e) => return Json(ApiResponse::err(format!("Package failed: {}", e))),
    };

    // Step 5: Index
    let indexed = if let Ok(store) = boltr_core::store::Store::open(&state.data_dir) {
        store.index_manifest(&manifest, &pkg_path).is_ok()
    } else {
        false
    };

    let files: Vec<String> = emitted.iter().map(|f| f.path.display().to_string()).collect();

    Json(ApiResponse::ok(serde_json::json!({
        "pipeline_complete": true,
        "records_normalized": records.len(),
        "yaml_files_emitted": files.len(),
        "files": files,
        "package_id": package_id,
        "package_files": manifest.files.len(),
        "indexed": indexed,
    })))
}

pub async fn list_artifacts(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse> {
    let store = match boltr_core::store::Store::open(&state.data_dir) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(format!("Store error: {}", e))),
    };

    match store.list_all() {
        Ok(artifacts) => {
            let items: Vec<serde_json::Value> = artifacts
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.row_id,
                        "package_id": a.package_id,
                        "file_path": a.file_path,
                        "file_type": a.file_type,
                        "sha256": a.sha256,
                        "size_bytes": a.size_bytes,
                        "source_db": a.source_db,
                        "source_id": a.source_id,
                    })
                })
                .collect();

            Json(ApiResponse::ok(serde_json::json!({ "artifacts": items })))
        }
        Err(e) => Json(ApiResponse::err(format!("Query failed: {}", e))),
    }
}

pub async fn list_packages(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse> {
    let packages_dir = state.data_dir.join("packages");
    let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);

    match manager.list_packages() {
        Ok(packages) => {
            let items: Vec<serde_json::Value> = packages
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "package_id": p.package_id,
                        "file_count": p.files.len(),
                        "total_size": p.total_size_bytes,
                        "created_at": p.created_at.to_rfc3339(),
                        "description": p.description,
                        "tags": p.tags,
                    })
                })
                .collect();

            Json(ApiResponse::ok(serde_json::json!({ "packages": items })))
        }
        Err(e) => Json(ApiResponse::err(format!("List failed: {}", e))),
    }
}
