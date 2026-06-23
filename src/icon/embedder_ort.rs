use std::path::Path;

use image::RgbImage;
use ort::session::Session;
use ort::value::Tensor;

use crate::error::{ExtractError, Result};

use super::preprocess::{finalize_embedding, INPUT_SIZE};

/// MobileCLIP2-S0 vision encoder (ONNX).
pub struct IconEmbedder {
    session: Session,
    input_name: String,
    output_name: String,
}

impl IconEmbedder {
    pub fn load(model_path: &Path) -> Result<Self> {
        if !model_path.is_file() {
            return Err(ExtractError::Image(format!(
                "MobileCLIP2 vision model not found: {} (run scripts/download_mobileclip2.ps1)",
                model_path.display()
            )));
        }

        let mut builder = Session::builder().map_err(|e| ExtractError::Image(e.to_string()))?;
        builder = crate::ort_runtime::apply_session_builder(builder, "icon embedder")?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let input_name = session
            .inputs()
            .first()
            .ok_or_else(|| ExtractError::Image("ONNX model has no inputs".into()))?
            .name()
            .to_string();
        let output_name = session
            .outputs()
            .first()
            .ok_or_else(|| ExtractError::Image("ONNX model has no outputs".into()))?
            .name()
            .to_string();

        Ok(Self {
            session,
            input_name,
            output_name,
        })
    }

    pub fn embed_rgb256(&mut self, rgb: &RgbImage) -> Result<Vec<f32>> {
        let tensor = super::preprocess::rgb256_to_nchw(rgb);
        self.embed_nchw(&tensor)
    }

    pub fn embed_nchw(&mut self, nchw: &[f32]) -> Result<Vec<f32>> {
        let expected = 3 * INPUT_SIZE as usize * INPUT_SIZE as usize;
        if nchw.len() != expected {
            return Err(ExtractError::Image(format!(
                "expected {expected} floats for NCHW input, got {}",
                nchw.len()
            )));
        }

        let input = Tensor::from_array((
            [1i64, 3, INPUT_SIZE as i64, INPUT_SIZE as i64],
            nchw.to_vec(),
        ))
        .map_err(|e| ExtractError::Image(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![self.input_name.as_str() => input])
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let (_shape, data) = outputs[self.output_name.as_str()]
            .try_extract_tensor::<f32>()
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        finalize_embedding(data.to_vec())
    }
}
