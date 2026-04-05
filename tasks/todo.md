# Boltr Bldr — Task Tracker

## Phase 1: Project Scaffolding
- [x] Create Cargo workspace with 3 crates (core, cli, web)
- [x] Define all Cargo.toml dependencies
- [x] Create core library skeleton with module structure

## Phase 2: Core — Data Models & Error Handling
- [x] Define error types (thiserror) for the entire application
- [x] Define protein data models (PDB structures, UniProt structures)
- [x] Define Boltr-compatible YAML schema models
- [x] Define artifact models (manifest.json, NPZ)

## Phase 3: Core — Ingest Pipeline
- [x] Implement RCSB PDB HTTP client and parser
- [x] Implement UniProt HTTP client and parser
- [x] Implement ingest orchestration (fetch + parse)

## Phase 4: Core — Normalization
- [x] Implement data normalization pipeline (PDB + UniProt → unified model)
- [x] Implement validation rules for normalized data

## Phase 5: Core — YAML Emission
- [x] Implement Boltr-compatible YAML emitter
- [x] Implement YAML round-trip validation (read back emitted YAML)

## Phase 6: Core — Artifact Packaging & Indexing
- [x] Implement manifest.json generation and parsing
- [x] Implement NPZ file handling (read/write/index)
- [x] Implement artifact store with local index (SQLite)

## Phase 7: CLI Interface
- [x] Implement clap CLI with all subcommands
- [x] Wire CLI to core library
- [x] Implement `ingest` subcommand (fetch from PDB/UniProt)
- [x] Implement `normalize` subcommand
- [x] Implement `emit` subcommand (generate Boltr YAML)
- [x] Implement `package` subcommand (create artifact bundle)
- [x] Implement `index` subcommand (index artifacts)
- [x] Implement `status` / `list` subcommand
- [x] Implement `pipeline` subcommand (run full pipeline)

## Phase 8: WebUI Backend (axum)
- [x] Create axum HTTP server with router
- [x] Implement REST API endpoints mirroring CLI commands
- [x] Implement static file serving for frontend
- [x] Add CORS and error handling middleware

## Phase 9: WebUI Frontend
- [x] Create HTML shell with navigation
- [x] Create CSS styling (clean, functional)
- [x] Create JavaScript app (fetch API, DOM manipulation)
- [x] Implement all pages: Dashboard, Ingest, Pipeline, Artifacts, Packages

## Phase 10: Integration & Polish
- [x] Full workspace compiles cleanly (cargo build --workspace)
- [x] All tests pass (cargo test --workspace)
- [x] CLI help output verified
- [x] README documentation
- [x] Final review and cleanup
