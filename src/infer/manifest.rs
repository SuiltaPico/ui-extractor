use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infer::error::{InferError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub quant: Option<String>,
    #[serde(default)]
    pub files: serde_json::Value,
    #[serde(default)]
    pub license: Option<LicenseInfo>,
    #[serde(default)]
    pub inputs: Option<serde_json::Value>,
    #[serde(default)]
    pub detection: Option<serde_json::Value>,
    #[serde(default)]
    pub dim: Option<u32>,
    #[serde(default)]
    pub embed_model_id: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub spdx: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub upstream: serde_json::Value,
}

impl Manifest {
    pub fn load_from_dir(pack_dir: &Path) -> Result<Self> {
        let path = pack_dir.join("manifest.json");
        let text = fs::read_to_string(&path).map_err(|e| {
            InferError::Manifest(format!("read {}: {e}", path.display()))
        })?;
        let manifest: Manifest = serde_json::from_str(&text)?;
        if manifest.schema != 1 {
            return Err(InferError::Manifest(format!(
                "unsupported schema {} in {}",
                manifest.schema,
                path.display()
            )));
        }
        if manifest.id.is_empty() {
            return Err(InferError::Manifest(format!(
                "missing id in {}",
                path.display()
            )));
        }
        Ok(manifest)
    }

    pub fn validate_license_files(&self, pack_dir: &Path) -> Result<()> {
        let Some(license) = &self.license else {
            return Err(InferError::License(format!(
                "pack {} missing license section in manifest",
                self.id
            )));
        };

        if license.spdx.is_empty() {
            return Err(InferError::License(format!(
                "pack {} missing license.spdx",
                self.id
            )));
        }

        if license.files.is_empty() {
            return Err(InferError::License(format!(
                "pack {} missing license.files",
                self.id
            )));
        }

        if license.upstream.is_null() {
            return Err(InferError::License(format!(
                "pack {} missing license.upstream",
                self.id
            )));
        }

        for rel in &license.files {
            let path = pack_dir.join(rel);
            if !path.is_file() {
                return Err(InferError::License(format!(
                    "pack {} missing license file: {}",
                    self.id,
                    path.display()
                )));
            }
            let meta = fs::metadata(&path)?;
            if meta.len() == 0 {
                return Err(InferError::License(format!(
                    "pack {} license file is empty: {}",
                    self.id,
                    path.display()
                )));
            }
        }

        Ok(())
    }

    pub fn file_path(&self, pack_dir: &Path, key: &str) -> Result<PathBuf> {
        let files = self.files.as_object().ok_or_else(|| {
            InferError::Manifest(format!("pack {} files is not an object", self.id))
        })?;
        let name = files.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
            InferError::Manifest(format!("pack {} missing files.{key}", self.id))
        })?;
        Ok(pack_dir.join(name))
    }

    /// Ensures every entry in `files` exists on disk (for pack CI / release).
    pub fn validate_pack_files(&self, pack_dir: &Path) -> Result<()> {
        let files = self.files.as_object().ok_or_else(|| {
            InferError::Manifest(format!("pack {} files is not an object", self.id))
        })?;
        for (key, value) in files {
            let name = value.as_str().ok_or_else(|| {
                InferError::Manifest(format!("pack {} files.{key} is not a string", self.id))
            })?;
            let path = pack_dir.join(name);
            if !path.is_file() {
                return Err(InferError::Manifest(format!(
                    "pack {} missing weight file {} ({})",
                    self.id,
                    key,
                    path.display()
                )));
            }
        }
        Ok(())
    }
}
