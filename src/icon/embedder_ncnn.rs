use std::path::Path;

use image::RgbImage;
use ncnn_bind::{Mat, Net};

use crate::error::{ExtractError, Result};
use crate::inference::ncnn::{load_net, mat_from_hwc, mat_to_vec_f32, nchw01_to_hwc};

use super::preprocess::{finalize_embedding, EMBED_DIM, INPUT_SIZE};

/// MobileCLIP2-S0 vision encoder (ncnn).
pub struct IconEmbedder {
    net: Net,
}

impl IconEmbedder {
    pub fn load(param_path: &Path) -> Result<Self> {
        let net = load_net(param_path).map_err(|e| match e {
            ExtractError::Image(msg) if msg.contains("ncnn") => ExtractError::Image(format!(
                "MobileCLIP2 ncnn model not found: {} (run scripts/convert_models_ncnn.ps1)",
                param_path.display()
            )),
            other => other,
        })?;
        Ok(Self { net })
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

        let hwc = nchw01_to_hwc(nchw, INPUT_SIZE, INPUT_SIZE);
        let input = mat_from_hwc(&hwc, INPUT_SIZE as i32, INPUT_SIZE as i32)?;

        let mut ex = self.net.create_extractor();
        ex.input("in0", &input)
            .map_err(|e| ExtractError::Image(format!("ncnn input: {e}")))?;

        let mut out = Mat::new();
        ex.extract("out0", &mut out)
            .map_err(|e| ExtractError::Image(format!("ncnn extract: {e}")))?;

        let raw = mat_to_vec_f32(&out)?;
        if raw.len() < EMBED_DIM {
            return Err(ExtractError::Image(format!(
                "ncnn embedding dim {} < expected {EMBED_DIM}",
                raw.len()
            )));
        }
        finalize_embedding(raw)
    }
}
