use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::infer::embed::EmbedEngine;
use crate::infer::error::{InferError, Result};
use crate::infer::ffi;
use crate::infer::icon_index::IconIndex;
use crate::infer::manifest::Manifest;
use crate::infer::ocr::OcrEngine;
use crate::infer::runtime::RuntimeConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryOwnership {
    Owned,
    Borrowed,
}

/// Wrapper around infer-core `InferRegistry*` with optional ownership.
#[derive(Debug)]
pub struct Registry {
    handle: *mut std::ffi::c_void,
    ownership: RegistryOwnership,
    models_dir: Option<PathBuf>,
    runtime_config: Option<RuntimeConfig>,
    manifest_cache: RefCell<HashMap<String, Manifest>>,
}

impl Registry {
    /// Create and own a new infer-core registry.
    pub fn open(models_dir: impl AsRef<Path>, runtime_config: RuntimeConfig) -> Result<Self> {
        let models_dir = models_dir.as_ref().to_path_buf();
        if !models_dir.is_dir() {
            return Err(InferError::PackNotFound(format!(
                "models directory not found: {}",
                models_dir.display()
            )));
        }

        let runtime_json = serde_json::to_string(&runtime_config)?;
        let handle = ffi::registry_create(&models_dir, Some(&runtime_json))?;

        Ok(Self {
            handle,
            ownership: RegistryOwnership::Owned,
            models_dir: Some(models_dir),
            runtime_config: Some(runtime_config),
            manifest_cache: RefCell::new(HashMap::new()),
        })
    }

    /// Borrow an existing infer-core registry handle (not destroyed on drop).
    pub fn from_borrowed(handle: *mut std::ffi::c_void) -> Self {
        Self {
            handle,
            ownership: RegistryOwnership::Borrowed,
            models_dir: None,
            runtime_config: None,
            manifest_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn native_handle(&self) -> *mut std::ffi::c_void {
        self.handle
    }

    pub fn ownership(&self) -> RegistryOwnership {
        self.ownership
    }

    pub fn models_dir(&self) -> Option<&Path> {
        self.models_dir.as_deref()
    }

    pub fn runtime_config(&self) -> Option<&RuntimeConfig> {
        self.runtime_config.as_ref()
    }

    pub fn pack_ids(&self) -> Result<Vec<String>> {
        let json = ffi::registry_pack_ids_json(self.handle)?;
        let ids: Vec<String> = serde_json::from_str(&json)?;
        Ok(ids)
    }

    pub fn manifest(&self, pack_id: &str) -> Result<Manifest> {
        {
            let cache = self.manifest_cache.borrow();
            if let Some(m) = cache.get(pack_id) {
                return Ok(m.clone());
            }
        }

        let json = ffi::registry_manifest_json(self.handle, pack_id)?;
        let manifest: Manifest = serde_json::from_str(&json)?;
        self.manifest_cache
            .borrow_mut()
            .insert(pack_id.to_string(), manifest.clone());
        Ok(manifest)
    }

    pub fn load_ocr(&self, pack_id: &str) -> Result<OcrEngine> {
        let manifest = self.manifest(pack_id)?;
        if manifest.kind != "ocr" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not ocr",
                manifest.kind
            )));
        }
        let handle = ffi::ocr_engine_load(self.handle, pack_id)?;
        Ok(OcrEngine { handle })
    }

    pub fn load_embed(&self, pack_id: &str) -> Result<EmbedEngine> {
        let manifest = self.manifest(pack_id)?;
        if manifest.kind != "embed" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not embed",
                manifest.kind
            )));
        }
        let handle = ffi::embed_engine_load(self.handle, pack_id)?;
        Ok(EmbedEngine { handle })
    }

    pub fn load_icon_index(&self, pack_id: &str) -> Result<IconIndex> {
        let manifest = self.manifest(pack_id)?;
        if manifest.kind != "icon_index" {
            return Err(InferError::Manifest(format!(
                "pack {pack_id} kind {} is not icon_index",
                manifest.kind
            )));
        }
        let handle = ffi::icon_index_load(self.handle, pack_id)?;
        Ok(IconIndex { handle })
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        if self.ownership == RegistryOwnership::Owned {
            ffi::registry_destroy(self.handle);
        }
    }
}
