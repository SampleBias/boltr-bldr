//! Boltr Core — shared business logic for Boltr Bldr
//!
//! This crate provides the core pipeline for:
//! - Ingesting protein data from RCSB PDB and UniProt
//! - Normalizing data into a unified representation
//! - Emitting Boltr-compatible YAML
//! - Packaging and indexing artifacts (manifest.json, NPZ)

pub mod artifact;
pub mod emit;
pub mod error;
pub mod ingest;
pub mod models;
pub mod normalize;
pub mod pipeline;
pub mod store;

pub use error::{Error, Result};
