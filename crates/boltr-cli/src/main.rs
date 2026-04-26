//! Boltr Bldr CLI — primary interface for power users and automation
//!
//! Provides subcommands for ingesting, normalizing, emitting, packaging,
//! and indexing protein data.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

const NORMALIZED_RECORDS_FILE: &str = "full_normalized.json";
const LEGACY_NORMALIZED_RECORDS_FILE: &str = "full_normalized.bincode";

#[derive(Parser)]
#[command(name = "boltr-bldr")]
#[command(
    version,
    about = "Boltr Bldr — ingest, normalize & emit Boltr-compatible YAML from protein data"
)]
#[command(
    long_about = "An all-Rust tool that ingests protein data from RCSB PDB and UniProt, \
    normalizes it, and emits Boltr-compatible YAML input files. Also supports packaging and \
    indexing of artifacts (manifest.json, NPZ result files). YAML is the canonical input format."
)]
struct Cli {
    /// Base data directory
    #[arg(long, global = true, default_value = "data", env = "BOLTR_DATA_DIR")]
    data_dir: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, global = true, default_value = "info", env = "BOLTR_LOG_LEVEL")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest protein data from RCSB PDB and/or UniProt
    Ingest {
        /// PDB IDs to fetch (e.g., 1ABC 7BV2)
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        pdb: Vec<String>,

        /// UniProt accessions to fetch (e.g., P12345 Q9Y6K9)
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        uniprot: Vec<String>,

        /// Output directory for raw data
        #[arg(long, default_value = "raw")]
        output: String,

        /// Save raw JSON responses
        #[arg(long)]
        save_raw: bool,
    },

    /// Normalize ingested data into unified records
    Normalize {
        /// Input directory (containing raw ingest data)
        #[arg(long, default_value = "raw")]
        input: String,

        /// Output directory for normalized data
        #[arg(long, default_value = "normalized")]
        output: String,
    },

    /// Emit Boltr-compatible YAML from normalized data
    Emit {
        /// Input directory (containing normalized data)
        #[arg(long, default_value = "normalized")]
        input: String,

        /// Output directory for Boltr YAML files
        #[arg(long, default_value = "boltr")]
        output: String,

        /// Schema version
        #[arg(long, default_value = "1.0.0")]
        version: String,
    },

    /// Package artifacts into a bundle with manifest.json
    Package {
        /// Input directory (containing Boltr YAML + artifacts)
        #[arg(long, default_value = "boltr")]
        input: String,

        /// Output directory for packages
        #[arg(long, default_value = "packages")]
        output: String,

        /// Package description
        #[arg(long)]
        description: Option<String>,

        /// Tags for categorization
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// Index artifacts in the local store
    Index {
        /// Rebuild the index from scratch
        #[arg(long)]
        rebuild: bool,

        /// Package directory to index
        #[arg(long)]
        package_dir: Option<PathBuf>,
    },

    /// Run the full pipeline (ingest → normalize → emit → package → index)
    Pipeline {
        /// PDB IDs to process
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        pdb: Vec<String>,

        /// UniProt accessions to process
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        uniprot: Vec<String>,

        /// Output directory
        #[arg(long, default_value = "output")]
        output: String,

        /// Skip indexing step
        #[arg(long)]
        skip_index: bool,
    },

    /// Show status / list indexed artifacts
    Status {
        /// Show detailed information
        #[arg(long, short)]
        verbose: bool,

        /// Filter by file type
        #[arg(long)]
        file_type: Option<String>,

        /// Filter by source database
        #[arg(long)]
        source: Option<String>,
    },

    /// List all packages
    List {
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .init();

    match cli.command {
        Commands::Ingest {
            pdb,
            uniprot,
            output,
            save_raw,
        } => {
            run_ingest(&cli.data_dir, &pdb, &uniprot, &output, save_raw).await?;
        }

        Commands::Normalize { input, output } => {
            run_normalize(&cli.data_dir, &input, &output).await?;
        }

        Commands::Emit {
            input,
            output,
            version,
        } => {
            run_emit(&cli.data_dir, &input, &output, &version)?;
        }

        Commands::Package {
            input,
            output,
            description,
            tags,
        } => {
            run_package(&cli.data_dir, &input, &output, description, &tags)?;
        }

        Commands::Index {
            rebuild,
            package_dir,
        } => {
            run_index(&cli.data_dir, rebuild, package_dir.as_deref())?;
        }

        Commands::Pipeline {
            pdb,
            uniprot,
            output,
            skip_index,
        } => {
            run_pipeline(&cli.data_dir, &pdb, &uniprot, &output, !skip_index).await?;
        }

        Commands::Status {
            verbose,
            file_type,
            source,
        } => {
            run_status(
                &cli.data_dir,
                verbose,
                file_type.as_deref(),
                source.as_deref(),
            )?;
        }

        Commands::List { format } => {
            run_list(&cli.data_dir, &format)?;
        }
    }

    Ok(())
}

async fn run_ingest(
    data_dir: &std::path::Path,
    pdb_ids: &[String],
    uniprot_accessions: &[String],
    output: &str,
    save_raw: bool,
) -> anyhow::Result<()> {
    if pdb_ids.is_empty() && uniprot_accessions.is_empty() {
        anyhow::bail!(
            "No sources specified. Use --pdb and/or --uniprot to specify entries to ingest."
        );
    }

    println!("🔬 Ingesting protein data...");
    println!(
        "   PDB IDs: {}",
        if pdb_ids.is_empty() {
            "(none)".into()
        } else {
            pdb_ids.join(", ")
        }
    );
    println!(
        "   UniProt: {}",
        if uniprot_accessions.is_empty() {
            "(none)".into()
        } else {
            uniprot_accessions.join(", ")
        }
    );

    let result = boltr_core::ingest::ingest_sources(pdb_ids, uniprot_accessions).await?;

    let output_dir = data_dir.join(output);
    std::fs::create_dir_all(&output_dir)?;

    // Save raw entries as JSON
    if save_raw {
        for entry in &result.pdb_entries {
            if let Some(ref raw) = entry.raw_json {
                let path = output_dir.join(format!("pdb_{}.json", entry.id.to_lowercase()));
                let json = serde_json::to_string_pretty(raw)?;
                std::fs::write(&path, json)?;
                println!("   📄 Saved: {}", path.display());
            }
        }
        for entry in &result.uniprot_entries {
            if let Some(ref raw) = entry.raw_json {
                let path =
                    output_dir.join(format!("uniprot_{}.json", entry.accession.to_lowercase()));
                let json = serde_json::to_string_pretty(raw)?;
                std::fs::write(&path, json)?;
                println!("   📄 Saved: {}", path.display());
            }
        }
    }

    // Save parsed entries as JSON
    for entry in &result.pdb_entries {
        let path = output_dir.join(format!("parsed_pdb_{}.json", entry.id.to_lowercase()));
        let json = serde_json::to_string_pretty(entry)?;
        std::fs::write(&path, json)?;
    }
    for entry in &result.uniprot_entries {
        let path = output_dir.join(format!(
            "parsed_uniprot_{}.json",
            entry.accession.to_lowercase()
        ));
        let json = serde_json::to_string_pretty(entry)?;
        std::fs::write(&path, json)?;
    }

    // Update pipeline state
    let store = boltr_core::store::Store::open(data_dir)?;
    for entry in &result.pdb_entries {
        store.update_pipeline_state("pdb", &entry.id, "ingested", None)?;
    }
    for entry in &result.uniprot_entries {
        store.update_pipeline_state("uniprot", &entry.accession, "ingested", None)?;
    }

    println!(
        "✅ Ingested {} entries ({} PDB, {} UniProt)",
        result.total_count(),
        result.pdb_entries.len(),
        result.uniprot_entries.len()
    );
    Ok(())
}

async fn run_normalize(
    data_dir: &std::path::Path,
    input: &str,
    output: &str,
) -> anyhow::Result<()> {
    println!("🔧 Normalizing protein data...");

    let input_dir = data_dir.join(input);
    let output_dir = data_dir.join(output);
    std::fs::create_dir_all(&output_dir)?;

    // Load parsed entries from the input directory
    let mut pdb_entries = Vec::new();
    let mut uniprot_entries = Vec::new();

    for entry in std::fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let content = std::fs::read_to_string(&path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        if name.starts_with("parsed_pdb_") {
            let pdb: boltr_core::models::pdb::PdbEntry = serde_json::from_value(json)?;
            pdb_entries.push(pdb);
        } else if name.starts_with("parsed_uniprot_") {
            let uniprot: boltr_core::models::uniprot::UniProtEntry = serde_json::from_value(json)?;
            uniprot_entries.push(uniprot);
        }
    }

    if pdb_entries.is_empty() && uniprot_entries.is_empty() {
        anyhow::bail!(
            "No parsed entries found in {}. Run `ingest` first.",
            input_dir.display()
        );
    }

    let records = boltr_core::normalize::normalize_batch(pdb_entries, uniprot_entries)?;

    // Save normalized records
    for record in &records {
        let path = output_dir.join(format!("{}.json", record.id));
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "id": record.id,
            "has_pdb": record.pdb.is_some(),
            "has_uniprot": record.uniprot.is_some(),
        }))?;
        std::fs::write(&path, json)?;
    }

    // Also save the full records
    let all_records_path = output_dir.join("normalized_records.json");
    let records_json = serde_json::to_string_pretty(&serde_json::json!({
        "count": records.len(),
        "records": records.iter().map(|r| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), serde_json::Value::String(r.id.clone()));
            if let Some(ref pdb) = r.pdb {
                obj.insert("pdb_id".into(), serde_json::Value::String(pdb.id.clone()));
            }
            if let Some(ref up) = r.uniprot {
                obj.insert("uniprot_accession".into(), serde_json::Value::String(up.accession.clone()));
            }
            serde_json::Value::Object(obj)
        }).collect::<Vec<_>>()
    }))?;
    std::fs::write(&all_records_path, records_json)?;

    // Save raw normalized data for emit step
    let full_path = output_dir.join(NORMALIZED_RECORDS_FILE);
    let encoded = serde_json::to_vec(&records)?;
    std::fs::write(&full_path, encoded)?;

    println!(
        "✅ Normalized {} records → {}",
        records.len(),
        output_dir.display()
    );
    Ok(())
}

fn run_emit(
    data_dir: &std::path::Path,
    input: &str,
    output: &str,
    version: &str,
) -> anyhow::Result<()> {
    println!("📝 Emitting Boltr-compatible YAML...");

    let input_dir = data_dir.join(input);
    let output_dir = data_dir.join(output);

    // Load normalized records
    let records = read_normalized_records(&input_dir)?;

    let opts = boltr_core::emit::EmitOptions {
        output_dir: output_dir.clone(),
        version: version.to_string(),
        include_raw: false,
    };

    let emitted = boltr_core::emit::emit_batch(&records, &opts)?;

    for file in &emitted {
        println!("   📄 {}", file.path.display());
    }

    println!(
        "✅ Emitted {} Boltr YAML files → {}",
        emitted.len(),
        output_dir.display()
    );
    Ok(())
}

fn read_normalized_records(
    input_dir: &std::path::Path,
) -> anyhow::Result<Vec<boltr_core::normalize::NormalizedRecord>> {
    let json_path = input_dir.join(NORMALIZED_RECORDS_FILE);
    let legacy_path = input_dir.join(LEGACY_NORMALIZED_RECORDS_FILE);
    let selected = if json_path.exists() {
        json_path
    } else if legacy_path.exists() {
        legacy_path
    } else {
        anyhow::bail!("No normalized data found. Run `normalize` first.");
    };

    let encoded = std::fs::read(&selected)?;
    let records = serde_json::from_slice(&encoded)?;
    Ok(records)
}

fn run_package(
    data_dir: &std::path::Path,
    input: &str,
    output: &str,
    description: Option<String>,
    tags: &[String],
) -> anyhow::Result<()> {
    println!("📦 Packaging artifacts...");

    let input_dir = data_dir.join(input);
    let output_dir = data_dir.join(output);

    let package_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let manager = boltr_core::artifact::ArtifactManager::new(&output_dir);
    let (pkg_path, manifest) =
        manager.package(&package_id, &input_dir, description, tags.to_vec())?;

    println!("   📦 Package: {}", pkg_path.display());
    println!("   📄 Files: {}", manifest.files.len());
    println!("   💾 Size: {} bytes", manifest.total_size_bytes);

    println!("✅ Package created: {}", package_id);
    Ok(())
}

fn run_index(
    data_dir: &std::path::Path,
    rebuild: bool,
    package_dir: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    println!("🗂️  Indexing artifacts...");

    let store = boltr_core::store::Store::open(data_dir)?;

    if rebuild {
        store.rebuild_index()?;
        println!("   🗑️  Index cleared");
    }

    let packages_dir = package_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| data_dir.join("packages"));

    let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);
    let packages = manager.list_packages()?;

    for manifest in &packages {
        let pkg_path = packages_dir.join(&manifest.package_id);
        store.index_manifest(manifest, &pkg_path)?;
        println!(
            "   📦 Indexed package: {} ({} files)",
            manifest.package_id,
            manifest.files.len()
        );
    }

    let stats = store.stats()?;
    println!(
        "✅ Index updated: {} artifacts, {} packages",
        stats.total_artifacts, stats.total_packages
    );
    Ok(())
}

async fn run_pipeline(
    data_dir: &std::path::Path,
    pdb_ids: &[String],
    uniprot_accessions: &[String],
    output: &str,
    do_index: bool,
) -> anyhow::Result<()> {
    println!("🚀 Running full pipeline...");
    println!(
        "   PDB: {}",
        if pdb_ids.is_empty() {
            "(none)".into()
        } else {
            pdb_ids.join(", ")
        }
    );
    println!(
        "   UniProt: {}",
        if uniprot_accessions.is_empty() {
            "(none)".into()
        } else {
            uniprot_accessions.join(", ")
        }
    );

    println!("\n📥 Step 1: Ingest");
    println!("\n🔧 Step 2: Normalize");
    println!("\n📝 Step 3: Emit YAML");
    println!("\n📦 Step 4: Package");
    if do_index {
        println!("\n🗂️  Step 5: Index");
    }

    let result = boltr_core::pipeline::run(boltr_core::pipeline::PipelineOptions {
        data_dir: data_dir.to_path_buf(),
        output_dir: data_dir.join(output),
        package_dir: data_dir.join("packages"),
        pdb_ids: pdb_ids.to_vec(),
        uniprot_accessions: uniprot_accessions.to_vec(),
        version: "1.0.0".to_string(),
        package_description: Some(format!(
            "Pipeline run for PDB:{}/UniProt:{}",
            pdb_ids.join(","),
            uniprot_accessions.join(",")
        )),
        package_tags: vec!["pipeline".to_string()],
        index: do_index,
    })
    .await?;

    println!("   Normalized {} records", result.records_normalized);
    println!("   Emitted {} YAML files", result.emitted.len());
    println!(
        "   Package: {} ({} files)",
        result.package_id,
        result.manifest.files.len()
    );
    if let Some(stats) = result.stats.as_ref() {
        println!("   Indexed: {} total artifacts", stats.total_artifacts);
    }

    println!("\n✅ Pipeline complete!");
    println!(
        "   YAML files: {}",
        result
            .emitted
            .iter()
            .map(|f| f.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("   Package: {}", result.package_path.display());

    Ok(())
}

fn run_status(
    data_dir: &std::path::Path,
    verbose: bool,
    file_type: Option<&str>,
    source: Option<&str>,
) -> anyhow::Result<()> {
    let store = boltr_core::store::Store::open(data_dir)?;
    let stats = store.stats()?;

    println!("📊 Boltr Bldr Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Total artifacts: {}", stats.total_artifacts);
    println!("   YAML files:      {}", stats.total_yaml);
    println!("   NPZ files:       {}", stats.total_npz);
    println!("   Packages:        {}", stats.total_packages);
    println!(
        "   Total size:      {} bytes ({:.2} MB)",
        stats.total_size_bytes,
        stats.total_size_bytes as f64 / 1_048_576.0
    );

    if verbose || file_type.is_some() || source.is_some() {
        println!("\n📄 Artifacts:");

        let artifacts = if let Some(ft) = file_type {
            store.find_by_type(ft)?
        } else {
            store.list_all()?
        };

        let filtered: Vec<_> = if let Some(src) = source {
            artifacts
                .into_iter()
                .filter(|a| a.source_db == src)
                .collect()
        } else {
            artifacts
        };

        for artifact in &filtered {
            println!(
                "   [{}] {} ({} bytes, {})",
                artifact.file_type, artifact.file_path, artifact.size_bytes, artifact.source_db
            );
            if verbose {
                println!("      SHA256: {}...", &artifact.sha256[..32]);
            }
        }

        if filtered.is_empty() {
            println!("   (no matching artifacts)");
        }
    }

    Ok(())
}

fn run_list(data_dir: &std::path::Path, format: &str) -> anyhow::Result<()> {
    let packages_dir = data_dir.join("packages");
    let manager = boltr_core::artifact::ArtifactManager::new(&packages_dir);
    let packages = manager.list_packages()?;

    if packages.is_empty() {
        println!("No packages found.");
        return Ok(());
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&packages)?;
            println!("{}", json);
        }
        _ => {
            println!("📦 Packages:");
            println!("{:<12} {:<8} {:<12} Created", "ID", "Files", "Size");
            println!("─────────────────────────────────────────────────────");
            for pkg in &packages {
                println!(
                    "{:<12} {:<8} {:<12} {}",
                    pkg.package_id,
                    pkg.files.len(),
                    format!("{} B", pkg.total_size_bytes),
                    pkg.created_at.format("%Y-%m-%d %H:%M")
                );
            }
        }
    }

    Ok(())
}
