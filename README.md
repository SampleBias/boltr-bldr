# ⚡ Boltr Bldr

```
___
 _______    ______   __     ________  _______         __        __        __           
|       \  /      \ |  \   |        \|       \       |  \      |  \      |  \          
| $$$$$$$\|  $$$$$$\| $$    \$$$$$$$$| $$$$$$$\      | $$____  | $$  ____| $$  ______  
| $$__/ $$| $$  | $$| $$      | $$   | $$__| $$      | $$    \ | $$ /      $$ /      \ 
| $$    $$| $$  | $$| $$      | $$   | $$    $$      | $$$$$$$\| $$|  $$$$$$$|  $$$$$$\
| $$$$$$$\| $$  | $$| $$      | $$   | $$$$$$$\      | $$  | $$| $$| $$  | $$| $$   \$$
| $$__/ $$| $$__/ $$| $$_____ | $$   | $$  | $$      | $$__/ $$| $$| $$__| $$| $$      
| $$    $$ \$$    $$| $$     \| $$   | $$  | $$      | $$    $$| $$ \$$    $$| $$      
 \$$$$$$$   \$$$$$$  \$$$$$$$$ \$$    \$$   \$$       \$$$$$$$  \$$  \$$$$$$$ \$$      
                                                                                       
                                                                                       
                                                                                       
____
```

A **Rust** workspace (shared `boltr-core`, `boltr-cli`, `boltr-web` server) that ingests protein data from **RCSB PDB** and **UniProt**, normalizes it, and emits **Boltr-compatible YAML** input files. Supports packaging and indexing of associated artifacts (`manifest.json`, NPZ result files). YAML is the canonical input format. The WebUI is served by Axum and rendered with vanilla HTML/CSS/JS in the browser.

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
├── Boltr_bldr_go           # Start WebUI: ./Boltr_bldr_go (runs from repo root)
├── docs/                   # Project notes
├── tasks/                  # Task tracking
├── crates/
│   ├── boltr-core/         # Shared business logic (no UI)
│   │   ├── error.rs        # Error types (thiserror)
│   │   ├── models/         # Data models
│   │   │   ├── pdb.rs      # PDB entry, entities, atoms, chains
│   │   │   ├── uniprot.rs  # UniProt entry, features, cross-refs
│   │   │   ├── boltr.rs    # Boltr YAML schema models
│   │   │   ├── af3_input.rs # AlphaFold3-style job block + builder entities
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

# WebUI — Start local web server (from repository root)
./Boltr_bldr_go
# Or: cargo run -p boltr-web -- --port 8081
# Open http://127.0.0.1:8081 (default port)
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

- **Dashboard** — Real-time stats (artifacts, YAML files, NPZ files, packages, total size); browse and re-index tracked artifacts in a table
- **Job builder** — AlphaFold 3–style entities (protein / DNA / RNA / ligand), optional structure upload; emits `*.boltr.yaml` with `af3_input`. Legacy PDB/UniProt fetch lives under a collapsible section.

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
| `BOLTR_WEB_PORT` | `8081` | WebUI port |
| `BOLTR_WEB_HOST` | `127.0.0.1` | WebUI bind address |
| `BOLTR_WEB_ALLOW_REMOTE` | unset | Must be `true` to bind the WebUI to a non-loopback host |

## GitHub

Source code and issue tracking: **[github.com/SampleBias/boltr-bldr](https://github.com/SampleBias/boltr-bldr)**

## License

[MIT](LICENSE)
