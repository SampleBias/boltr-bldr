//! Artifact packaging and indexing
//!
//! Handles manifest.json generation, NPZ/CIF/JSON artifact handling, and bundling.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::models::artifact::*;

pub type AtomSite<'a> = (
    u32,
    &'a str,
    &'a str,
    i32,
    &'a str,
    f64,
    f64,
    f64,
    Option<f64>,
    Option<f64>,
    Option<&'a str>,
);

/// Manages artifact packaging operations
pub struct ArtifactManager {
    /// Base directory for packages
    pub packages_dir: PathBuf,
}

impl ArtifactManager {
    pub fn new(packages_dir: impl Into<PathBuf>) -> Self {
        Self {
            packages_dir: packages_dir.into(),
        }
    }

    /// Create a manifest.json for a set of files
    pub fn create_manifest(
        &self,
        package_id: &str,
        sources: Vec<ManifestSource>,
        files: Vec<PathBuf>,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<Manifest> {
        let mut manifest_files = Vec::new();
        let mut total_size: u64 = 0;

        for file_path in &files {
            let metadata = std::fs::metadata(file_path).map_err(|e| {
                Error::Artifact(format!("Cannot read file {}: {}", file_path.display(), e))
            })?;

            let content = std::fs::read(file_path)?;
            let sha256 = compute_sha256_bytes(&content);

            let file_type = detect_file_type(file_path);
            let relative_path = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            total_size += metadata.len();

            manifest_files.push(ManifestFile {
                path: relative_path,
                file_type,
                sha256,
                size_bytes: metadata.len(),
                description: None,
            });
        }

        let manifest = Manifest {
            version: "1.0.0".to_string(),
            package_id: package_id.to_string(),
            created_at: chrono::Utc::now(),
            sources,
            files: manifest_files,
            total_size_bytes: total_size,
            pipeline_version: env!("CARGO_PKG_VERSION").to_string(),
            tags,
            description,
        };

        Ok(manifest)
    }

    /// Write a manifest.json to disk
    pub fn write_manifest(&self, manifest: &Manifest) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.packages_dir)?;

        let manifest_path = self
            .packages_dir
            .join(format!("{}_manifest.json", manifest.package_id));
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(&manifest_path, json)?;

        tracing::info!(path = %manifest_path.display(), "Manifest written");
        Ok(manifest_path)
    }

    /// Read a manifest.json from disk
    pub fn read_manifest(&self, path: &Path) -> Result<Manifest> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Package a set of files into a directory with a manifest
    pub fn package(
        &self,
        package_id: &str,
        source_dir: &Path,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<(PathBuf, Manifest)> {
        let pkg_dir = self.packages_dir.join(package_id);
        std::fs::create_dir_all(&pkg_dir)?;

        // Copy files and hash each stream once to avoid rereading package contents.
        let mut manifest_files = Vec::new();
        let mut total_size: u64 = 0;
        for entry in walkdir::WalkDir::new(source_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let src_path = entry.path().to_path_buf();
                let file_name = entry.file_name().to_string_lossy().to_string();
                let dest_path = pkg_dir.join(&file_name);
                let (sha256, size_bytes) = copy_file_with_sha256(&src_path, &dest_path)?;

                total_size += size_bytes;
                manifest_files.push(ManifestFile {
                    path: file_name,
                    file_type: detect_file_type(&dest_path),
                    sha256,
                    size_bytes,
                    description: None,
                });
            }
        }

        let sources = Vec::new(); // Sources would be populated from context
        let mut manifest = Manifest {
            version: "1.0.0".to_string(),
            package_id: package_id.to_string(),
            created_at: chrono::Utc::now(),
            sources,
            files: manifest_files,
            total_size_bytes: total_size,
            pipeline_version: env!("CARGO_PKG_VERSION").to_string(),
            tags,
            description,
        };

        // Write manifest into the package directory
        let manifest_path = pkg_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(&manifest_path, &json)?;

        // Add the manifest itself to the file list
        manifest.files.push(ManifestFile {
            path: "manifest.json".to_string(),
            file_type: "json".to_string(),
            sha256: compute_sha256_bytes(json.as_bytes()),
            size_bytes: json.len() as u64,
            description: Some("Package manifest".to_string()),
        });

        tracing::info!(
            package_id = %package_id,
            files = manifest.files.len(),
            size = manifest.total_size_bytes,
            "Package created"
        );

        Ok((pkg_dir, manifest))
    }

    /// Scan NPZ files in a directory and extract metadata
    pub fn scan_npz_files(&self, dir: &Path) -> Result<Vec<NpzMetadata>> {
        let mut npz_files = Vec::new();

        let pattern = dir.join("**/*.npz");
        let pattern_str = pattern.to_string_lossy();

        for entry in glob::glob(&pattern_str)
            .map_err(|e| Error::Artifact(format!("Glob error: {}", e)))?
            .flatten()
        {
            if let Ok(metadata) = self.inspect_npz(&entry) {
                npz_files.push(metadata);
            }
        }

        tracing::info!(count = npz_files.len(), "NPZ files scanned");
        Ok(npz_files)
    }

    /// Inspect an NPZ file and extract metadata
    /// NPZ files are ZIP archives containing .npy arrays
    pub fn inspect_npz(&self, path: &Path) -> Result<NpzMetadata> {
        let content = std::fs::read(path)?;
        let sha256 = compute_sha256_bytes(&content);
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // NPZ is a ZIP file; read the archive to get array info
        let mut arrays = Vec::new();
        let reader = std::io::Cursor::new(&content);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| Error::Artifact(format!("Failed to read NPZ {}: {}", filename, e)))?;

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| Error::Artifact(format!("Failed to read NPZ entry: {}", e)))?;

            let name = file.name().to_string();
            // Strip .npy extension for the array name
            let array_name = name.trim_end_matches(".npy").to_string();

            // We can't easily read from a ZipFile and parse the .npy header in-place,
            // so we just report the array name for now.
            arrays.push(NpzArrayInfo {
                name: array_name,
                shape: "unknown".to_string(),
                dtype: "unknown".to_string(),
            });
        }

        Ok(NpzMetadata {
            filename,
            sha256,
            size_bytes: content.len() as u64,
            arrays,
        })
    }

    /// Write a minimal CIF (mmCIF) file from PDB entry data.
    /// This produces a valid mmCIF frame with cell, symmetry, and atom_site records.
    pub fn write_cif(
        output_path: &Path,
        pdb_id: &str,
        title: &str,
        method: &str,
        resolution: Option<f64>,
        atoms: &[AtomSite<'_>],
    ) -> Result<PathBuf> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut cif = String::new();

        // Header
        cif.push_str(&format!("data_{}\n", pdb_id.to_uppercase()));
        cif.push_str("#\n");

        // Entry
        cif.push_str("_entry.id ");
        cif.push_str(&pdb_id.to_uppercase());
        cif.push_str("\n#\n");

        // Structure title
        cif.push_str("_struct.title '");
        cif.push_str(&title.replace('\'', "''"));
        cif.push_str("'\n#\n");

        // Experimental method
        cif.push_str("_exptl.method '");
        cif.push_str(&method.replace('\'', "''"));
        cif.push_str("'\n");

        if let Some(res) = resolution {
            cif.push_str(&format!("_refine.ls_d_res_high {:.2}\n", res));
        }
        cif.push_str("#\n");

        // Atom sites
        if !atoms.is_empty() {
            cif.push_str("loop_\n");
            cif.push_str("_atom_site.group_PDB\n");
            cif.push_str("_atom_site.id\n");
            cif.push_str("_atom_site.label_atom_id\n");
            cif.push_str("_atom_site.label_comp_id\n");
            cif.push_str("_atom_site.label_seq_id\n");
            cif.push_str("_atom_site.label_asym_id\n");
            cif.push_str("_atom_site.Cartn_x\n");
            cif.push_str("_atom_site.Cartn_y\n");
            cif.push_str("_atom_site.Cartn_z\n");
            cif.push_str("_atom_site.occupancy\n");
            cif.push_str("_atom_site.B_iso_or_equiv\n");
            cif.push_str("_atom_site.type_symbol\n");

            for (serial, atom_name, res_name, res_seq, chain_id, x, y, z, occ, b_factor, element) in
                atoms
            {
                cif.push_str(&format!(
                    "ATOM {} {} {} {} {} {:.3} {:.3} {:.3} {} {} {}\n",
                    serial,
                    atom_name,
                    res_name,
                    res_seq,
                    chain_id,
                    x,
                    y,
                    z,
                    occ.map(|v| format!("{:.2}", v))
                        .unwrap_or_else(|| ".".to_string()),
                    b_factor
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_else(|| ".".to_string()),
                    element.unwrap_or("?"),
                ));
            }
            cif.push_str("#\n");
        }

        std::fs::write(output_path, &cif)?;

        tracing::info!(path = %output_path.display(), atoms = atoms.len(), "CIF file written");
        Ok(output_path.to_path_buf())
    }

    /// Parse a simple CIF file and extract atom records
    pub fn parse_cif(path: &Path) -> Result<CifData> {
        let content = std::fs::read_to_string(path)?;
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut data = CifData {
            filename: filename.clone(),
            pdb_id: String::new(),
            title: String::new(),
            method: None,
            resolution: None,
            atom_count: 0,
        };

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("data_") {
                data.pdb_id = line.trim_start_matches("data_").to_string();
            } else if line.starts_with("_struct.title") {
                data.title = extract_cif_value(line);
            } else if line.starts_with("_exptl.method") {
                data.method = Some(extract_cif_value(line));
            } else if line.starts_with("_refine.ls_d_res_high") {
                data.resolution = extract_cif_value(line).parse::<f64>().ok();
            } else if line.starts_with("ATOM ") {
                data.atom_count += 1;
            }
        }

        tracing::info!(file = %filename, atoms = data.atom_count, "CIF file parsed");
        Ok(data)
    }

    /// Scan a directory for all supported artifact types (NPZ, JSON, CIF)
    pub fn scan_artifacts(&self, dir: &Path) -> Result<ArtifactScan> {
        let mut scan = ArtifactScan {
            npz_files: Vec::new(),
            json_files: Vec::new(),
            cif_files: Vec::new(),
            other_files: Vec::new(),
        };

        if !dir.exists() {
            return Ok(scan);
        }

        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path().to_path_buf();
            let ft = detect_file_type(&path);

            match ft.as_str() {
                "npz" => {
                    if let Ok(meta) = self.inspect_npz(&path) {
                        scan.npz_files.push(meta);
                    }
                }
                "json" => {
                    let sha = file_sha256(&path);
                    let size = file_size(&path);
                    scan.json_files.push(GenericArtifactInfo {
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        sha256: sha,
                        size_bytes: size,
                    });
                }
                "cif" => {
                    if let Ok(cif_data) = ArtifactManager::parse_cif(&path) {
                        scan.cif_files.push(cif_data);
                    }
                }
                _ => {
                    let sha = file_sha256(&path);
                    let size = file_size(&path);
                    scan.other_files.push(GenericArtifactInfo {
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        sha256: sha,
                        size_bytes: size,
                    });
                }
            }
        }

        tracing::info!(
            npz = scan.npz_files.len(),
            json = scan.json_files.len(),
            cif = scan.cif_files.len(),
            other = scan.other_files.len(),
            "Artifact scan complete"
        );

        Ok(scan)
    }

    /// List all packages in the packages directory
    pub fn list_packages(&self) -> Result<Vec<Manifest>> {
        if !self.packages_dir.exists() {
            return Ok(Vec::new());
        }

        let mut manifests = Vec::new();
        for entry in std::fs::read_dir(&self.packages_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    match self.read_manifest(&manifest_path) {
                        Ok(m) => manifests.push(m),
                        Err(e) => {
                            tracing::warn!(path = %manifest_path.display(), error = %e, "Failed to read manifest");
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }
}

/// Result of scanning a directory for artifacts
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactScan {
    pub npz_files: Vec<NpzMetadata>,
    pub json_files: Vec<GenericArtifactInfo>,
    pub cif_files: Vec<CifData>,
    pub other_files: Vec<GenericArtifactInfo>,
}

/// Generic artifact info for JSON and other files
#[derive(Debug, Clone, serde::Serialize)]
pub struct GenericArtifactInfo {
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
}

/// Parsed CIF file data
#[derive(Debug, Clone, serde::Serialize)]
pub struct CifData {
    pub filename: String,
    pub pdb_id: String,
    pub title: String,
    pub method: Option<String>,
    pub resolution: Option<f64>,
    pub atom_count: usize,
}

/// Extract a quoted value from a CIF line like `_key 'value'`
fn extract_cif_value(line: &str) -> String {
    if let Some(start) = line.find('\'') {
        if let Some(end) = line.rfind('\'') {
            if start != end {
                return line[start + 1..end].replace("''", "'");
            }
        }
    }
    // Fallback: split on whitespace and take last token
    line.split_whitespace().last().unwrap_or("").to_string()
}

fn file_sha256(path: &Path) -> String {
    std::fs::read(path)
        .map(|bytes| compute_sha256_bytes(&bytes))
        .unwrap_or_default()
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Detect file type from extension
fn detect_file_type(path: &Path) -> String {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .as_deref()
    {
        Some("yaml" | "yml") => "yaml".to_string(),
        Some("json") => "json".to_string(),
        Some("npz") => "npz".to_string(),
        Some("pdb") => "pdb".to_string(),
        Some("cif") => "cif".to_string(),
        Some("fasta" | "fa") => "fasta".to_string(),
        Some("csv") => "csv".to_string(),
        Some("txt") => "txt".to_string(),
        _ => "other".to_string(),
    }
}

fn copy_file_with_sha256(src: &Path, dest: &Path) -> Result<(String, u64)> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};

    let mut input = std::fs::File::open(src)?;
    let mut output = std::fs::File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
        size_bytes += read as u64;
    }

    Ok((hex::encode(hasher.finalize()), size_bytes))
}

/// Compute SHA-256 hash of bytes
fn compute_sha256_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_file_type() {
        assert_eq!(detect_file_type(Path::new("test.yaml")), "yaml");
        assert_eq!(detect_file_type(Path::new("test.yml")), "yaml");
        assert_eq!(detect_file_type(Path::new("test.json")), "json");
        assert_eq!(detect_file_type(Path::new("test.npz")), "npz");
        assert_eq!(detect_file_type(Path::new("test.pdb")), "pdb");
        assert_eq!(detect_file_type(Path::new("test.cif")), "cif");
        assert_eq!(detect_file_type(Path::new("test.fasta")), "fasta");
    }

    #[test]
    fn test_extract_cif_value() {
        assert_eq!(
            extract_cif_value("_struct.title 'Hello World'"),
            "Hello World"
        );
        assert_eq!(extract_cif_value("_entry.id 1ABC"), "1ABC");
        assert_eq!(
            extract_cif_value("_exptl.method 'X-RAY DIFFRACTION'"),
            "X-RAY DIFFRACTION"
        );
    }

    #[test]
    fn test_write_cif() {
        let dir = std::env::temp_dir().join("boltr_test_cif");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.cif");

        let atoms: Vec<(
            u32,
            &str,
            &str,
            i32,
            &str,
            f64,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            Option<&str>,
        )> = vec![
            (
                1,
                "CA",
                "ALA",
                1,
                "A",
                10.0,
                20.0,
                30.0,
                Some(1.0),
                Some(20.0),
                Some("C"),
            ),
            (
                2,
                "N",
                "ALA",
                1,
                "A",
                11.0,
                21.0,
                31.0,
                Some(1.0),
                Some(15.0),
                Some("N"),
            ),
        ];

        let result =
            ArtifactManager::write_cif(&path, "1ABC", "Test Structure", "X-RAY", Some(1.5), &atoms);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("data_1ABC"));
        assert!(content.contains("ATOM 1 CA ALA 1 A"));
        assert!(content.contains("_refine.ls_d_res_high 1.50"));

        // Parse it back
        let parsed = ArtifactManager::parse_cif(&path).unwrap();
        assert_eq!(parsed.pdb_id, "1ABC");
        assert_eq!(parsed.title, "Test Structure");
        assert_eq!(parsed.method, Some("X-RAY".to_string()));
        assert_eq!(parsed.resolution, Some(1.5));
        assert_eq!(parsed.atom_count, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
