# Boltr Bldr — Project README (AI Agent Context)

## Overview
**Boltr Bldr** is a fully Rust-native application that:
1. Ingests protein data from **RCSB PDB** and **UniProt** databases
2. Normalizes the data into a unified internal representation
3. Emits **Boltr-compatible YAML** input files
4. Packages and indexes associated artifacts:
   - `manifest.json` — metadata about a dataset/run
   - `.npz` files — NumPy-compressed result arrays
5. Treats **YAML as the canonical input format**

## Dual Interface
- **CLI** (primary): `clap`-based, scriptable, pipe-friendly, for power users & CI/CD
- **WebUI** (secondary): Local `axum` HTTP server with browser-based UI exposing identical features

## Architecture
```
boltr-bldr/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── boltr-core/         # Shared core library (no UI)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs    # Error types
│   │   │   ├── models/     # Data models
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pdb.rs       # PDB structures
│   │   │   │   ├── uniprot.rs   # UniProt structures
│   │   │   │   ├── boltr.rs     # Boltr YAML schema
│   │   │   │   └── artifact.rs  # manifest.json, NPZ
│   │   │   ├── ingest/     # Data ingestion
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pdb_client.rs
│   │   │   │   └── uniprot_client.rs
│   │   │   ├── normalize.rs # Normalization pipeline
│   │   │   ├── emit.rs      # YAML emission
│   │   │   ├── artifact.rs  # Packaging & indexing
│   │   │   └── store.rs     # Local artifact store / index
│   │   └── Cargo.toml
│   ├── boltr-cli/          # CLI binary
│   │   ├── src/
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   └── boltr-web/          # WebUI binary
│       ├── src/
│       │   ├── main.rs
│       │   ├── routes.rs
│       │   └── handlers.rs
│       ├── static/         # Frontend assets
│       │   ├── index.html
│       │   ├── style.css
│       │   └── app.js
│       └── Cargo.toml
├── data/                   # Local data directory (created at runtime)
├── tasks/
│   └── todo.md
└── docs/
    ├── activity.md
    └── PROJECT_README.md
```

## Key Technologies
- **HTTP Client**: `reqwest` (with `rustls-tls`)
- **Serialization**: `serde` + `serde_yaml` + `serde_json`
- **CLI**: `clap` with derive macros
- **Web Framework**: `axum` + `tokio`
- **Database**: `rusqlite` for local indexing
- **Error Handling**: `thiserror` + `anyhow`
- **Logging**: `tracing`

## Build & Run
```bash
# Build everything
cargo build --workspace

# CLI usage
cargo run --bin boltr-cli -- --help
cargo run --bin boltr-cli -- ingest pdb 1ABC
cargo run --bin boltr-cli -- ingest uniprot P12345
cargo run --bin boltr-cli -- normalize --input data/raw/ --output data/normalized/
cargo run --bin boltr-cli -- emit --input data/normalized/ --output data/boltr/
cargo run --bin boltr-cli -- package --input data/boltr/ --output data/packages/
cargo run --bin boltr-cli -- index --rebuild
cargo run --bin boltr-cli -- pipeline --pdb 1ABC --uniprot P12345 --output data/output/

# WebUI
cargo run --bin boltr-web -- --port 8080
# Then open http://localhost:8080
```
