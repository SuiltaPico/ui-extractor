use std::path::Path;

use image::RgbImage;
use infer_core::{EmbedEngine, RuntimeConfig};

use crate::error::{ExtractError, Result};

use super::preprocess::INPUT_SIZE;

/// MobileCLIP2-S0 vision encoder (ONNX via infer-core).
pub struct IconEmbedder {
    inner: EmbedEngine,
}

impl IconEmbedder {
    pub fn load(model_path: &Path) -> Result<Self> {
        Self::load_with_runtime(model_path, RuntimeConfig::from_env_or_default())
    }

    pub fn load_with_runtime(model_path: &Path, runtime: RuntimeConfig) -> Result<Self> {
        let inner = EmbedEngine::load(model_path, &runtime)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        Ok(Self { inner })
    }

    pub fn embed_rgb256(&mut self, rgb: &RgbImage) -> Result<Vec<f32>> {
        self.inner
            .embed_rgb256(rgb)
            .map_err(|e| ExtractError::Image(e.to_string()))
    }

    pub fn embed_nchw(&mut self, nchw: &[f32]) -> Result<Vec<f32>> {
        self.inner
            .embed_nchw(nchw)
            .map_err(|e| ExtractError::Image(e.to_string()))
    }
}

#[allow(dead_code)]
const _: () = assert!(INPUT_SIZE == infer_core::INPUT_SIZE);
