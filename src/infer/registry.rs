use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::infer::embed::EmbedEngine;
use crate::infer::error::{InferError, Result};
use crate::infer::ffi;
use crate::infer::icon_index::IconIndex;
use crate::infer::manifest::Manifest;
use crate::infer::ocr::OcrEngine;
use crate::infer::runtime::RuntimeConfig;

#[derive(Debug)]
pub struct Registry {
    handle: *mut std::ffi::c_void,
    models_dir: PathBuf,
    runtime_config: RuntimeConfig,
    packs: HashMap<String, PackEntry>,
}

#[derive(Debug, Clone)]
struct PackEntry {
    dir: PathBuf,
    manifest: Manifest,
}

impl Registry {
    pub fn open(models_dir: impl AsRef<Path>, runtime_config: RuntimeConfig) -> Result<Self> {
        let models_dir = models_dir.as_ref().to_path_buf();
        if !models_dir.is_dir() {
            return Err(InferError::PackNotFound(format!(
                "models directory not found: {}",
                models_dir.display()
            )));
        }

        let mut packs = HashMap::new();
        for entry in fs::read_dir(&models_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest = Manifest::load_from_dir(&path)?;
            manifest.validate_license_files(&path)?;
            let id = manifest.id.clone();
            if path.file_name().and_then(|s| s.to_str()) != Some(id.as_str()) {
                return Err(InferError::Manifest(format!(
                    "directory name must match manifest id: {} != {}",
                    path.file_name().unwrap().to_string_lossy(),
                    id
                )));
            }
            packs.insert(id, PackEntry { dir: path, manifest });
        }

        let runtime_json = serde_json::to_string(&runtime_config)?;
        let handle = ffi::registry_create(&models_dir, Some(&runtime_json))?;

        Ok(Self {
            handle,
            models_dir,
            runtime_config,
            packs,
        })
    }

    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    pub fn runtime_config(&self) -> &RuntimeConfig {
        &self.runtime_config
    }

    pub fn pack_ids(&self) -> impl Iterator<Item = &str> {
        self.packs.keys().map(String::as_str)
    }

    pub fn manifest(&self, pack_id: &str) -> Result<&Manifest> {
        self.packs
            .get(pack_id)
            .map(|p| &p.manifest)
            .ok_or_else(|| InferError::PackNotFound(pack_id.to_string()))
    }

    pub fn pack_dir(&self, pack_id: &str) -> Result<&Path> {
        self.packs
            .get(pack_id)
            .map(|p| p.dir.as_path())
            .ok_or_else(|| InferError::PackNotFound(pack_id.to_string()))
    }

    pub fn load_ocr(&self, pack_id: &str) -> Result<OcrEngine> {
        let entry = self
            .packs
            .get(pack_id)
            .ok_or_else(|| InferError::PackNotFound(pack_id.to_string()))?;
        if entry.manifest.kind != "ocr" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not ocr",
                entry.manifest.kind
            )));
        }
        let handle = ffi::ocr_engine_load(self.handle, pack_id)?;
        Ok(OcrEngine { handle })
    }

    pub fn load_embed(&self, pack_id: &str) -> Result<EmbedEngine> {
        let entry = self
            .packs
            .get(pack_id)
            .ok_or_else(|| InferError::PackNotFound(pack_id.to_string()))?;
        if entry.manifest.kind != "embed" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not embed",
                entry.manifest.kind
            )));
        }
        let handle = ffi::embed_engine_load(self.handle, pack_id)?;
        Ok(EmbedEngine { handle })
    }

    pub fn load_icon_index(&self, pack_id: &str) -> Result<IconIndex> {
        let entry = self
            .packs
            .get(pack_id)
            .ok_or_else(|| InferError::PackNotFound(pack_id.to_string()))?;
        if entry.manifest.kind != "icon_index" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not icon_index",
                entry.manifest.kind
            )));
        }
        IconIndex::from_manifest(&entry.dir, &entry.manifest)
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        ffi::registry_destroy(self.handle);
    }
}
