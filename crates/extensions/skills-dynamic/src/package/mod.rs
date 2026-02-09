//! Skill package format for distribution.
//!
//! Provides `.skill` single-file package format for easy distribution and installation.
//!
//! # Format
//!
//! A `.skill` file is a tar.gz archive with an optional signature header:
//!
//! ```text
//! [SKIL][v1][signature?][tar.gz content]
//! ```
//!
//! The archive contains the skill directory structure with `SKILL.markdown` as the entry point.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Archive, Builder};
use tracing::{debug, info};

use autohands_protocols::error::SkillError;

/// Magic bytes for .skill files.
const MAGIC: &[u8; 4] = b"SKIL";

/// Current package format version.
const VERSION: u8 = 1;

/// Skill package for distribution.
pub struct SkillPackage {
    /// Package format version.
    pub version: u8,
    /// Optional Ed25519 signature (64 bytes).
    pub signature: Option<[u8; 64]>,
    /// Compressed archive data.
    pub archive: Vec<u8>,
}

impl SkillPackage {
    /// Create a new package from raw archive data.
    pub fn new(archive: Vec<u8>) -> Self {
        Self {
            version: VERSION,
            signature: None,
            archive,
        }
    }

    /// Create a signed package.
    pub fn with_signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Read a package from a file.
    pub fn from_file(path: &Path) -> Result<Self, SkillError> {
        let file = File::open(path).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to open package {}: {}", path.display(), e))
        })?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).map_err(|e| {
            SkillError::ParsingError(format!("Failed to read package magic: {}", e))
        })?;

        if &magic != MAGIC {
            return Err(SkillError::ParsingError(
                "Invalid skill package: wrong magic bytes".to_string(),
            ));
        }

        // Read version
        let mut version = [0u8; 1];
        reader.read_exact(&mut version).map_err(|e| {
            SkillError::ParsingError(format!("Failed to read package version: {}", e))
        })?;
        let version = version[0];

        if version > VERSION {
            return Err(SkillError::ParsingError(format!(
                "Unsupported package version: {} (max supported: {})",
                version, VERSION
            )));
        }

        // Read signature flag
        let mut has_sig = [0u8; 1];
        reader.read_exact(&mut has_sig).map_err(|e| {
            SkillError::ParsingError(format!("Failed to read signature flag: {}", e))
        })?;

        let signature = if has_sig[0] == 1 {
            let mut sig = [0u8; 64];
            reader.read_exact(&mut sig).map_err(|e| {
                SkillError::ParsingError(format!("Failed to read signature: {}", e))
            })?;
            Some(sig)
        } else {
            None
        };

        // Read archive data
        let mut archive = Vec::new();
        reader.read_to_end(&mut archive).map_err(|e| {
            SkillError::ParsingError(format!("Failed to read archive data: {}", e))
        })?;

        Ok(Self {
            version,
            signature,
            archive,
        })
    }

    /// Write the package to a file.
    pub fn to_file(&self, path: &Path) -> Result<(), SkillError> {
        let file = File::create(path).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to create package {}: {}", path.display(), e))
        })?;
        let mut writer = BufWriter::new(file);

        // Write magic
        writer.write_all(MAGIC).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to write magic: {}", e))
        })?;

        // Write version
        writer.write_all(&[self.version]).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to write version: {}", e))
        })?;

        // Write signature
        if let Some(sig) = &self.signature {
            writer.write_all(&[1]).map_err(|e| {
                SkillError::LoadingFailed(format!("Failed to write signature flag: {}", e))
            })?;
            writer.write_all(sig).map_err(|e| {
                SkillError::LoadingFailed(format!("Failed to write signature: {}", e))
            })?;
        } else {
            writer.write_all(&[0]).map_err(|e| {
                SkillError::LoadingFailed(format!("Failed to write signature flag: {}", e))
            })?;
        }

        // Write archive data
        writer.write_all(&self.archive).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to write archive: {}", e))
        })?;

        Ok(())
    }

    /// Extract the package to a directory.
    pub fn extract(&self, dest: &Path) -> Result<PathBuf, SkillError> {
        // Create destination if needed
        fs::create_dir_all(dest).map_err(|e| {
            SkillError::LoadingFailed(format!(
                "Failed to create destination {}: {}",
                dest.display(),
                e
            ))
        })?;

        // Decompress and extract
        let decoder = GzDecoder::new(self.archive.as_slice());
        let mut archive = Archive::new(decoder);

        archive.unpack(dest).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to extract archive: {}", e))
        })?;

        // Find the skill directory (first directory or the destination itself)
        let mut skill_dir = dest.to_path_buf();
        for entry in fs::read_dir(dest).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to read destination: {}", e))
        })? {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    skill_dir = entry.path();
                    break;
                }
            }
        }

        Ok(skill_dir)
    }
}

/// Skill packager for creating .skill files.
pub struct SkillPackager;

impl SkillPackager {
    /// Pack a skill directory into a .skill file.
    ///
    /// Returns the path to the created package.
    pub fn pack(skill_dir: &Path, output_dir: &Path) -> Result<PathBuf, SkillError> {
        // Verify SKILL.markdown exists
        let skill_file = skill_dir.join("SKILL.markdown");
        let skill_file = if skill_file.exists() {
            skill_file
        } else {
            let alt = skill_dir.join("SKILL.md");
            if alt.exists() {
                alt
            } else {
                return Err(SkillError::NotFound(format!(
                    "No SKILL.markdown found in {}",
                    skill_dir.display()
                )));
            }
        };

        // Parse the skill to get metadata
        let content = fs::read_to_string(&skill_file).map_err(|e| {
            SkillError::LoadingFailed(format!("Failed to read skill file: {}", e))
        })?;

        let skill = crate::loader::parse_skill_markdown(&content, Some(skill_dir))?;

        // Determine package name
        let version = skill
            .definition
            .metadata
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        let package_name = format!("{}-{}.skill", skill.definition.id, version);
        let package_path = output_dir.join(&package_name);

        // Create tar.gz archive
        let mut archive_data = Vec::new();
        {
            let encoder = GzEncoder::new(&mut archive_data, Compression::default());
            let mut tar = Builder::new(encoder);

            // Get the skill directory name for the archive root
            let skill_name = skill_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&skill.definition.id);

            // Add all files from the skill directory
            Self::add_directory_to_tar(&mut tar, skill_dir, skill_name)?;

            tar.finish().map_err(|e| {
                SkillError::LoadingFailed(format!("Failed to finalize archive: {}", e))
            })?;
        }

        // Create and save package
        let package = SkillPackage::new(archive_data);
        package.to_file(&package_path)?;

        info!(
            "Created skill package: {} ({} bytes)",
            package_path.display(),
            fs::metadata(&package_path)
                .map(|m| m.len())
                .unwrap_or(0)
        );

        Ok(package_path)
    }

    /// Install a .skill package.
    pub fn install(package_path: &Path, skills_dir: &Path) -> Result<PathBuf, SkillError> {
        let package = SkillPackage::from_file(package_path)?;

        // Extract to skills directory
        let skill_dir = package.extract(skills_dir)?;

        info!("Installed skill to: {}", skill_dir.display());
        Ok(skill_dir)
    }

    // Note: install_from_url is not implemented to avoid reqwest dependency
    // Use CLI tools like curl/wget to download .skill files, then install locally

    /// Add a directory to a tar archive recursively.
    fn add_directory_to_tar<W: Write>(
        tar: &mut Builder<W>,
        dir: &Path,
        archive_prefix: &str,
    ) -> Result<(), SkillError> {
        for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let relative = path
                .strip_prefix(dir)
                .map_err(|e| SkillError::LoadingFailed(format!("Path error: {}", e)))?;

            // Build the path in the archive
            let archive_path = if relative.as_os_str().is_empty() {
                PathBuf::from(archive_prefix)
            } else {
                PathBuf::from(archive_prefix).join(relative)
            };

            if path.is_file() {
                debug!("Adding to archive: {}", archive_path.display());
                let mut file = File::open(path).map_err(|e| {
                    SkillError::LoadingFailed(format!(
                        "Failed to open {}: {}",
                        path.display(),
                        e
                    ))
                })?;
                tar.append_file(&archive_path, &mut file).map_err(|e| {
                    SkillError::LoadingFailed(format!(
                        "Failed to add {} to archive: {}",
                        path.display(),
                        e
                    ))
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_skill_dir(dir: &Path) {
        fs::create_dir_all(dir).unwrap();

        let skill_content = r#"---
id: test-package
name: Test Package Skill
version: 1.2.3
description: A skill for testing packaging
---

# Test Package Skill

This is a test skill for packaging.
"#;

        fs::write(dir.join("SKILL.markdown"), skill_content).unwrap();

        // Add some additional files
        fs::write(dir.join("README.md"), "# Test Skill\n\nReadme content.").unwrap();
    }

    #[test]
    fn test_package_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill_dir(&skill_dir);

        // Pack
        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();
        assert!(package_path.exists());
        assert!(package_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".skill"));

        // Read package
        let package = SkillPackage::from_file(&package_path).unwrap();
        assert_eq!(package.version, VERSION);
        assert!(package.signature.is_none());
        assert!(!package.archive.is_empty());

        // Extract
        let extract_dir = temp_dir.path().join("extracted");
        let extracted = package.extract(&extract_dir).unwrap();

        // Verify extraction
        let extracted_skill = extracted.join("SKILL.markdown");
        assert!(extracted_skill.exists());
    }

    #[test]
    fn test_package_name_format() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("my-skill");
        create_test_skill_dir(&skill_dir);

        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();

        let file_name = package_path.file_name().unwrap().to_str().unwrap();
        assert_eq!(file_name, "test-package-1.2.3.skill");
    }

    #[test]
    fn test_install_package() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("original");
        create_test_skill_dir(&skill_dir);

        // Pack
        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();

        // Install to new location
        let install_dir = temp_dir.path().join("installed");
        let installed = SkillPackager::install(&package_path, &install_dir).unwrap();

        // Verify
        let skill_file = installed.join("SKILL.markdown");
        assert!(skill_file.exists());
    }

    #[test]
    fn test_invalid_package_magic() {
        let temp_dir = TempDir::new().unwrap();
        let fake_package = temp_dir.path().join("fake.skill");
        fs::write(&fake_package, b"NOT A SKILL PACKAGE").unwrap();

        let result = SkillPackage::from_file(&fake_package);
        assert!(result.is_err());
    }

    #[test]
    fn test_package_with_signature() {
        let archive_data = vec![1, 2, 3, 4];
        let signature = [42u8; 64];

        let package = SkillPackage::new(archive_data).with_signature(signature);

        assert!(package.signature.is_some());
        assert_eq!(package.signature.unwrap(), signature);
    }

    #[test]
    fn test_pack_missing_skill_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_dir = temp_dir.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        let result = SkillPackager::pack(&empty_dir, temp_dir.path());
        assert!(result.is_err());
    }
}
