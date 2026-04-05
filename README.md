# ⚡ Boltr Bldr

An all-Rust tool that ingests protein data from **RCSB PDB** and **UniProt**, normalizes it, and emits **Boltr-compatible YAML** input files. Supports packaging and indexing of associated artifacts (`manifest.json`, NPZ result files). YAML is the canonical input format.

## Dual Interface

| Interface | Purpose | Technology |
|-----------|---------|------------|
| **CLI** | Primary interface for power users & automation | `clap` with derive macros |
| **WebUI** | Local browser interface with equivalent features | `axum` + vanilla HTML/CSS/JS |

Both interfaces expose the **exact same core features** through the shared `boltr-core` library.

## Architecture

```
boltr-bldr/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── boltr-core/         # Shared business logic (no UI)
│   │   ├── error.rs        # Error types (thiserror)
│   │   ├── models/         # Data models
│   │   │   ├── pdb.rs      # PDB entry, entities, atoms, chains
│   │   │   ├── uniprot.rs  # UniProt entry, features, cross-refs
│   │   │   ├── boltr.rs    # Boltr YAML schema models
│   │   │   └── artifact.rs # manifest.json, NPZ metadata
│   │   ├── ingest/         # HTTP clients for PDB & UniProt
│   │   ├── normalize.rs    # Normalization pipeline
│   │   ├── emit.rs         # Boltr YAML emitter
│   │   ├── artifact.rs     # Packaging & NPZ handling
│   │   └── store.rs        # SQLite-backed artifact index
│   ├── boltr-cli/          # CLI binary
│   └── boltr-web/          # WebUI binary (axum + static files)
└── data/                   # Local data directory (runtime)
```

## Quick Start

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# CLI — Ingest data from PDB and UniProt
cargo run --bin boltr-cli -- ingest --pdb 1ABC 7BV2 --uniprot P12345

# CLI — Run the full pipeline (ingest → normalize → emit → package → index)
cargo run --bin boltr-cli -- pipeline --pdb 4HHB --uniprot P68871

# CLI — Check status
cargo run --bin boltr-cli -- status --verbose

# WebUI — Start local web server
cargo run --bin boltr-web -- --port 8080
# Open http://localhost:8080
```

## CLI Subcommands

| Command | Description |
|---------|-------------|
| `ingest` | Fetch protein data from RCSB PDB and/or UniProt |
| `normalize` | Normalize ingested data into unified records |
| `emit` | Generate Boltr-compatible YAML files |
| `package` | Bundle artifacts with manifest.json |
| `index` | Index artifacts in local SQLite store |
| `pipeline` | Run full pipeline (ingest → normalize → emit → package → index) |
| `status` | Show indexed artifact statistics |
| `list` | List all packages |

## WebUI Pages

- **Dashboard** — Real-time stats (artifacts, YAML files, NPZ files, packages, total size)
- **Ingest** — Step-by-step: Fetch → Normalize → Emit YAML
- **Pipeline** — One-click full pipeline execution
- **Artifacts** — Browse and re-index all tracked artifacts
- **Packages** — View and create artifact bundles

## Key Technologies

| Component | Crate |
|-----------|-------|
| HTTP Client | `reqwest` (rustls-tls) |
| Serialization | `serde` + `serde_yaml` + `serde_json` |
| CLI | `clap` (derive) |
| Web Server | `axum` + `tokio` |
| Database | `rusqlite` (bundled SQLite) |
| Error Handling | `thiserror` |
| Logging | `tracing` + `tracing-subscriber` |
| NPZ/ZIP | `zip` crate |
| Hashing | `sha2` |

## Pipeline Flow

```
RCSB PDB ──→ Fetch ──→ Parse ──┐
                                ├─→ Normalize ─→ Emit YAML ─→ Package ─→ Index
UniProt ────→ Fetch ──→ Parse ──┘
```

## Configuration

Environment variables (also available as CLI flags):

| Variable | Default | Description |
|----------|---------|-------------|
| `BOLTR_DATA_DIR` | `data` | Base data directory |
| `BOLTR_LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |
| `BOLTR_WEB_PORT` | `8080` | WebUI port |
| `BOLTR_WEB_HOST` | `127.0.0.1` | WebUI bind address |

## License

MIT
