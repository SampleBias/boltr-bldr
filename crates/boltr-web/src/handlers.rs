//! API request handlers — each mirrors a CLI subcommand

use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use axum::{
    extract::{Multipart, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

use boltr_core::models::BuilderEntity;

use crate::AppState;

const NORMALIZED_RECORDS_FILE: &str = "full_normalized.json";
const LEGACY_NORMALIZED_RECORDS_FILE: &str = "full_normalized.bincode";

// ── Response type ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

impl ApiResponse {
    fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(msg: impl ToString) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
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

#[derive(Deserialize)]
pub struct JobYamlRequest {
    pub name: String,
    #[serde(default)]
    pub model_seeds: Vec<i64>,
    #[serde(default = "default_schema_version")]
    pub version: String,
    pub entities: Vec<BuilderEntity>,
}

#[derive(Deserialize)]
pub struct ArtifactListQuery {
    limit: Option<u32>,
    offset: Option<u32>,
}

fn default_schema_version() -> String {
    "1.0.0".to_string()
}

async fn blocking_response<F>(work: F) -> ApiResponse
where
    F: FnOnce() -> ApiResponse + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(response) => response,
        Err(e) => ApiResponse::err(format!("Worker task failed: {}", e)),
    }
}

fn data_path(
    data_dir: &Path,
    requested: Option<&str>,
    default: &str,
) -> std::result::Result<PathBuf, String> {
    let raw = requested.unwrap_or(default).trim();
    if raw.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let rel = Path::new(raw);
    if rel.is_absolute() {
        return Err(format!("Path must be relative to data_dir: {}", raw));
    }

    for component in rel.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(format!("Path cannot contain '..': {}", raw)),
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Path must stay inside data_dir: {}", raw));
            }
        }
    }

    Ok(data_dir.join(rel))
}

fn read_normalized_records(
    input_dir: &Path,
) -> std::result::Result<Vec<boltr_core::normalize::NormalizedRecord>, String> {
    let json_path = input_dir.join(NORMALIZED_RECORDS_FILE);
    let legacy_path = input_dir.join(LEGACY_NORMALIZED_RECORDS_FILE);
    let selected = if json_path.exists() {
        json_path
    } else {
        legacy_path
    };

    let encoded = std::fs::read(&selected)
        .map_err(|e| format!("No normalized data: {}. Run normalize first.", e))?;
    serde_json::from_slice(&encoded).map_err(|e| format!("Parse error: {}", e))
}

// ── Handlers ──────────────────────────────────────────────────────────

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let data_dir = state.data_dir.clone();
    Json(
        blocking_response(move || {
            let store = match boltr_core::store::Store::open(&data_dir) {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Store error: {}", e)),
            };

            let stats = match store.stats() {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Stats error: {}", e)),
            };

            ApiResponse::ok(serde_json::json!({
                "status": "running",
                "data_dir": data_dir.to_string_lossy(),
                "stats": stats,
            }))
        })
        .await,
    )
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let data_dir = state.data_dir.clone();
    Json(
        blocking_response(move || {
            let store = match boltr_core::store::Store::open(&data_dir) {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Store error: {}", e)),
            };

            let stats = match store.stats() {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Stats error: {}", e)),
            };

            ApiResponse::ok(serde_json::json!({ "stats": stats }))
        })
        .await,
    )
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
            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                return Json(ApiResponse::err(format!(
                    "Cannot create raw output dir: {}",
                    e
                )));
            }

            for entry in &result.pdb_entries {
                if let Some(ref raw) = entry.raw_json {
                    let path = output_dir.join(format!("pdb_{}.json", entry.id.to_lowercase()));
                    let json = match serde_json::to_vec(raw) {
                        Ok(j) => j,
                        Err(e) => {
                            return Json(ApiResponse::err(format!(
                                "Serialize PDB raw failed: {}",
                                e
                            )))
                        }
                    };
                    if let Err(e) = std::fs::write(&path, json) {
                        return Json(ApiResponse::err(format!(
                            "Write {} failed: {}",
                            path.display(),
                            e
                        )));
                    }
                }
                let path = output_dir.join(format!("parsed_pdb_{}.json", entry.id.to_lowercase()));
                let json = match serde_json::to_vec(entry) {
                    Ok(j) => j,
                    Err(e) => {
                        return Json(ApiResponse::err(format!(
                            "Serialize PDB entry failed: {}",
                            e
                        )))
                    }
                };
                if let Err(e) = std::fs::write(&path, json) {
                    return Json(ApiResponse::err(format!(
                        "Write {} failed: {}",
                        path.display(),
                        e
                    )));
                }
            }
            for entry in &result.uniprot_entries {
                if let Some(ref raw) = entry.raw_json {
                    let path =
                        output_dir.join(format!("uniprot_{}.json", entry.accession.to_lowercase()));
                    let json = match serde_json::to_vec(raw) {
                        Ok(j) => j,
                        Err(e) => {
                            return Json(ApiResponse::err(format!(
                                "Serialize UniProt raw failed: {}",
                                e
                            )))
                        }
                    };
                    if let Err(e) = std::fs::write(&path, json) {
                        return Json(ApiResponse::err(format!(
                            "Write {} failed: {}",
                            path.display(),
                            e
                        )));
                    }
                }
                let path = output_dir.join(format!(
                    "parsed_uniprot_{}.json",
                    entry.accession.to_lowercase()
                ));
                let json = match serde_json::to_vec(entry) {
                    Ok(j) => j,
                    Err(e) => {
                        return Json(ApiResponse::err(format!(
                            "Serialize UniProt entry failed: {}",
                            e
                        )))
                    }
                };
                if let Err(e) = std::fs::write(&path, json) {
                    return Json(ApiResponse::err(format!(
                        "Write {} failed: {}",
                        path.display(),
                        e
                    )));
                }
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
    let input_dir = match data_path(&state.data_dir, req.input.as_deref(), "raw") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let output_dir = match data_path(&state.data_dir, req.output.as_deref(), "normalized") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    Json(
        blocking_response(move || {
            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                return ApiResponse::err(format!("Cannot create output dir: {}", e));
            }

            let mut pdb_entries = Vec::new();
            let mut uniprot_entries = Vec::new();

            let entries = match std::fs::read_dir(&input_dir) {
                Ok(entries) => entries,
                Err(e) => return ApiResponse::err(format!("Cannot read input dir: {}", e)),
            };

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => return ApiResponse::err(format!("Cannot read input entry: {}", e)),
                };
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if name.starts_with("parsed_pdb_") {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            return ApiResponse::err(format!(
                                "Read {} failed: {}",
                                path.display(),
                                e
                            ))
                        }
                    };
                    match serde_json::from_str(&content) {
                        Ok(pdb) => pdb_entries.push(pdb),
                        Err(e) => {
                            return ApiResponse::err(format!(
                                "Parse {} failed: {}",
                                path.display(),
                                e
                            ))
                        }
                    }
                } else if name.starts_with("parsed_uniprot_") {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            return ApiResponse::err(format!(
                                "Read {} failed: {}",
                                path.display(),
                                e
                            ))
                        }
                    };
                    match serde_json::from_str(&content) {
                        Ok(up) => uniprot_entries.push(up),
                        Err(e) => {
                            return ApiResponse::err(format!(
                                "Parse {} failed: {}",
                                path.display(),
                                e
                            ))
                        }
                    }
                }
            }

            match boltr_core::normalize::normalize_batch(pdb_entries, uniprot_entries) {
                Ok(records) => {
                    let count = records.len();
                    let encoded = match serde_json::to_vec(&records) {
                        Ok(encoded) => encoded,
                        Err(e) => {
                            return ApiResponse::err(format!(
                                "Serialize normalized records failed: {}",
                                e
                            ))
                        }
                    };
                    let full_path = output_dir.join(NORMALIZED_RECORDS_FILE);
                    if let Err(e) = std::fs::write(&full_path, encoded) {
                        return ApiResponse::err(format!(
                            "Write {} failed: {}",
                            full_path.display(),
                            e
                        ));
                    }

                    ApiResponse::ok(serde_json::json!({
                        "normalized": true,
                        "record_count": count,
                        "file": full_path.display().to_string(),
                    }))
                }
                Err(e) => ApiResponse::err(format!("Normalize failed: {}", e)),
            }
        })
        .await,
    )
}

pub async fn emit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmitRequest>,
) -> Json<ApiResponse> {
    let input_dir = match data_path(&state.data_dir, req.input.as_deref(), "normalized") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let output_dir = match data_path(&state.data_dir, req.output.as_deref(), "boltr") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    let records = match read_normalized_records(&input_dir) {
        Ok(r) => r,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    let opts = boltr_core::emit::EmitOptions {
        output_dir,
        version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
        include_raw: false,
    };

    match boltr_core::emit::emit_batch(&records, &opts) {
        Ok(emitted) => {
            let files: Vec<String> = emitted
                .iter()
                .map(|f| f.path.display().to_string())
                .collect();
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
    let input_dir = match data_path(&state.data_dir, req.input.as_deref(), "boltr") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let output_dir = match data_path(&state.data_dir, req.output.as_deref(), "packages") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    let package_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let package_id_for_response = package_id.clone();

    Json(
        blocking_response(move || {
            let manager = boltr_core::artifact::ArtifactManager::new(&output_dir);

            match manager.package(
                &package_id,
                &input_dir,
                req.description,
                req.tags.unwrap_or_default(),
            ) {
                Ok((pkg_path, manifest)) => ApiResponse::ok(serde_json::json!({
                    "packaged": true,
                    "package_id": package_id_for_response,
                    "path": pkg_path.display().to_string(),
                    "file_count": manifest.files.len(),
                    "total_size": manifest.total_size_bytes,
                })),
                Err(e) => ApiResponse::err(format!("Package failed: {}", e)),
            }
        })
        .await,
    )
}

pub async fn index_artifacts(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let data_dir = state.data_dir.clone();
    Json(
        blocking_response(move || {
            let store = match boltr_core::store::Store::open(&data_dir) {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Store error: {}", e)),
            };

            let packages_dir = data_dir.join("packages");
            let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);

            match manager.list_packages() {
                Ok(packages) => {
                    let mut total_indexed = 0;
                    for manifest in &packages {
                        let pkg_path = packages_dir.join(&manifest.package_id);
                        match store.index_manifest(manifest, &pkg_path) {
                            Ok(row_ids) => total_indexed += row_ids.len(),
                            Err(e) => {
                                return ApiResponse::err(format!(
                                    "Index package {} failed: {}",
                                    manifest.package_id, e
                                ));
                            }
                        }
                    }

                    ApiResponse::ok(serde_json::json!({
                        "indexed": true,
                        "packages": packages.len(),
                        "artifacts": total_indexed,
                    }))
                }
                Err(e) => ApiResponse::err(format!("Index failed: {}", e)),
            }
        })
        .await,
    )
}

pub async fn run_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineRequest>,
) -> Json<ApiResponse> {
    if req.pdb.is_empty() && req.uniprot.is_empty() {
        return Json(ApiResponse::err("No sources specified"));
    }

    let output_dir = match data_path(&state.data_dir, req.output.as_deref(), "output") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let packages_dir = state.data_dir.join("packages");

    let result = match boltr_core::pipeline::run(boltr_core::pipeline::PipelineOptions {
        data_dir: state.data_dir.clone(),
        output_dir,
        package_dir: packages_dir,
        pdb_ids: req.pdb,
        uniprot_accessions: req.uniprot,
        version: "1.0.0".to_string(),
        package_description: Some("Pipeline run via WebUI".to_string()),
        package_tags: vec!["pipeline".to_string()],
        index: true,
    })
    .await
    {
        Ok(result) => result,
        Err(e) => return Json(ApiResponse::err(format!("Pipeline failed: {}", e))),
    };

    let files: Vec<String> = result
        .emitted
        .iter()
        .map(|f| f.path.display().to_string())
        .collect();

    Json(ApiResponse::ok(serde_json::json!({
        "pipeline_complete": true,
        "steps": ["ingest", "normalize", "emit", "package", "index"],
        "records_normalized": result.records_normalized,
        "yaml_files_emitted": files.len(),
        "files": files,
        "package_id": result.package_id,
        "package_path": result.package_path.display().to_string(),
        "package_files": result.manifest.files.len(),
        "indexed": result.indexed,
    })))
}

pub async fn list_artifacts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ArtifactListQuery>,
) -> Json<ApiResponse> {
    let data_dir = state.data_dir.clone();
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0);

    Json(
        blocking_response(move || {
            let store = match boltr_core::store::Store::open(&data_dir) {
                Ok(s) => s,
                Err(e) => return ApiResponse::err(format!("Store error: {}", e)),
            };

            match store.list_page(limit as usize, offset as usize) {
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

                    ApiResponse::ok(serde_json::json!({
                        "artifacts": items,
                        "limit": limit,
                        "offset": offset,
                    }))
                }
                Err(e) => ApiResponse::err(format!("Query failed: {}", e)),
            }
        })
        .await,
    )
}

pub async fn post_job_yaml(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JobYamlRequest>,
) -> Json<ApiResponse> {
    if req.entities.is_empty() {
        return Json(ApiResponse::err("No entities specified"));
    }

    let seeds = if req.model_seeds.is_empty() {
        vec![1]
    } else {
        req.model_seeds
    };

    let output_dir = match data_path(&state.data_dir, None, "output") {
        Ok(path) => path,
        Err(e) => return Json(ApiResponse::err(e)),
    };

    match boltr_core::emit::emit_af3_job(&req.version, &req.name, seeds, &req.entities, &output_dir)
    {
        Ok(emitted) => {
            let yaml = std::fs::read_to_string(&emitted.path).unwrap_or_default();
            Json(ApiResponse::ok(serde_json::json!({
                "emitted": true,
                "path": emitted.path.display().to_string(),
                "sha256": emitted.sha256,
                "size_bytes": emitted.size_bytes,
                "yaml": yaml,
            })))
        }
        Err(e) => Json(ApiResponse::err(format!("Emit failed: {}", e))),
    }
}

pub async fn upload_structure(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<ApiResponse> {
    let uploads = state.data_dir.join("uploads");
    if let Err(e) = std::fs::create_dir_all(&uploads) {
        return Json(ApiResponse::err(format!(
            "Cannot create uploads dir: {}",
            e
        )));
    }

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return Json(ApiResponse::err(format!("Multipart error: {}", e))),
        };

        if field.name() != Some("file") {
            continue;
        }

        let original = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "structure.cif".into());
        let ext = std::path::Path::new(&original)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("cif");
        let id = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}.{}", id, ext);
        let path = uploads.join(&filename);

        let bytes = match field.bytes().await {
            Ok(b) => b,
            Err(e) => return Json(ApiResponse::err(format!("Read error: {}", e))),
        };

        if let Err(e) = std::fs::write(&path, &bytes) {
            return Json(ApiResponse::err(format!("Write error: {}", e)));
        }

        let rel = format!("uploads/{}", filename);
        return Json(ApiResponse::ok(serde_json::json!({
            "uploaded": true,
            "path": rel,
            "size_bytes": bytes.len(),
        })));
    }

    Json(ApiResponse::err("No file field in multipart body"))
}

pub async fn list_packages(State(state): State<Arc<AppState>>) -> Json<ApiResponse> {
    let data_dir = state.data_dir.clone();
    Json(
        blocking_response(move || {
            let packages_dir = data_dir.join("packages");
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

                    ApiResponse::ok(serde_json::json!({ "packages": items }))
                }
                Err(e) => ApiResponse::err(format!("List failed: {}", e)),
            }
        })
        .await,
    )
}
