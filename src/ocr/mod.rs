use std::path::{Path, PathBuf};
#[cfg(feature = "backend-ncnn")]
use std::time::Instant;

use image::DynamicImage;
#[cfg(feature = "backend-ncnn")]
use image::RgbImage;
use infer_core::{OcrEngine, OcrWord as CoreWord, RuntimeConfig};

use crate::{
    error::Result,
    types::Bounds,
};

#[cfg(feature = "backend-ncnn")]
mod ncnn;

#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Directory containing det/rec models and dictionary.
    pub model_dir: PathBuf,
    pub min_confidence: f32,
    /// OCR input long-edge limit (0 = no downscale).
    pub max_side: u32,
    /// ONNX Runtime EP chain (passed through to infer-core).
    pub runtime: RuntimeConfig,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            model_dir: PathBuf::from("models"),
            min_confidence: 0.5,
            max_side: 960,
            runtime: RuntimeConfig::from_env_or_default(),
        }
    }
}

impl OcrConfig {
    fn infer_config(&self) -> infer_core::OcrConfig {
        infer_core::OcrConfig {
            min_confidence: self.min_confidence,
            max_side: self.max_side,
            ..Default::default()
        }
    }

    /// Legacy flat `models/` layout; Phase 3a will use Registry + pack id.
    #[allow(deprecated)]
    fn engine(&self) -> Result<OcrEngine> {
        OcrEngine::from_model_dir(&self.model_dir, self.infer_config(), self.runtime.clone())
            .map_err(|e| crate::error::ExtractError::Ocr(e.to_string()))
    }

    pub fn det_model(&self) -> PathBuf {
        self.model_dir.join("pp-ocrv5_mobile_det.ncnn.param")
    }

    pub fn rec_model(&self) -> PathBuf {
        self.model_dir.join("pp-ocrv5_mobile_rec.ncnn.param")
    }

    pub fn dict_path(&self) -> PathBuf {
        self.model_dir.join("ppocrv5_dict.txt")
    }
}

#[cfg(feature = "backend-ncnn")]
pub(crate) fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(feature = "backend-ncnn")]
pub(crate) fn resize_rgb_for_ocr(rgb: RgbImage, max_side: u32) -> (RgbImage, f32) {
    infer_core::ocr::resize_rgb_for_ocr(rgb, max_side)
}

#[cfg(feature = "backend-ncnn")]
pub(crate) fn scale_bounds(bounds: Bounds, coord_scale: f32) -> Bounds {
    let scaled = infer_core::ocr::scale_bounds(
        infer_core::OcrBounds::new(bounds.x, bounds.y, bounds.width, bounds.height),
        coord_scale,
    );
    Bounds::new(scaled.x, scaled.y, scaled.width, scaled.height)
}

#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub bounds: Bounds,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default)]
pub struct OcrTimings {
    pub init_ms: f64,
    pub predict_ms: f64,
}

pub fn extract_words(image_path: &Path, config: &OcrConfig) -> Result<Vec<OcrWord>> {
    extract_words_timed(image_path, config).map(|(words, _)| words)
}

pub fn extract_words_from_image(image: &DynamicImage, config: &OcrConfig) -> Result<Vec<OcrWord>> {
    extract_words_from_image_timed(image, config).map(|(words, _)| words)
}

pub fn extract_words_from_image_timed(
    image: &DynamicImage,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    #[cfg(feature = "backend-ncnn")]
    if uses_ncnn(&config.model_dir) {
        return ncnn::extract_words_from_image_timed(image, config);
    }

    let engine = config.engine()?;
    let (words, timings) = engine
        .recognize_timed(image)
        .map_err(|e| crate::error::ExtractError::Ocr(e.to_string()))?;
    Ok((convert_words(words), convert_timings(timings)))
}

pub fn extract_words_timed(
    image_path: &Path,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    #[cfg(feature = "backend-ncnn")]
    if uses_ncnn(&config.model_dir) {
        return ncnn::extract_words_timed(image_path, config);
    }

    let image = image::open(image_path)
        .map_err(|_| crate::error::ExtractError::ImageRead(image_path.display().to_string()))?;
    extract_words_from_image_timed(&image, config)
}

#[cfg(feature = "backend-ncnn")]
fn uses_ncnn(model_dir: &Path) -> bool {
    model_dir
        .join("pp-ocrv5_mobile_det.ncnn.param")
        .is_file()
}

fn convert_words(words: Vec<CoreWord>) -> Vec<OcrWord> {
    words
        .into_iter()
        .map(|w| OcrWord {
            text: w.text,
            bounds: Bounds::new(w.bounds.x, w.bounds.y, w.bounds.width, w.bounds.height),
            confidence: w.confidence,
        })
        .collect()
}

fn convert_timings(t: infer_core::OcrTimings) -> OcrTimings {
    OcrTimings {
        init_ms: t.init_ms,
        predict_ms: t.predict_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infer_core::OcrBounds;

    #[test]
    fn ocr_config_defaults_runtime() {
        let config = OcrConfig::default();
        assert!(config.max_side > 0);
    }

    #[test]
    fn convert_bounds_roundtrip() {
        let core = infer_core::OcrWord {
            text: "hi".into(),
            bounds: OcrBounds::new(1, 2, 3, 4),
            confidence: 99.0,
        };
        let word = convert_words(vec![core]).pop().unwrap();
        assert_eq!(word.bounds, Bounds::new(1, 2, 3, 4));
    }
}
