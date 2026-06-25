use std::path::Path;

use image::DynamicImage;
use crate::infer::{OcrEngine, OcrWord as CoreWord, Registry};

use crate::{
    error::Result,
    types::Bounds,
};

#[derive(Debug, Clone)]
pub struct OcrConfig {
    pub min_confidence: f32,
    /// OCR input long-edge limit (0 = no downscale).
    pub max_side: u32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_side: 960,
        }
    }
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

pub fn load_ocr_engine(registry: &Registry, pack_id: &str, config: &OcrConfig) -> Result<OcrEngine> {
    let mut engine = registry
        .load_ocr(pack_id)
        .map_err(|e| crate::error::ExtractError::Ocr(e.to_string()))?;
    engine.apply_config_overrides(Some(config.min_confidence), Some(config.max_side));
    Ok(engine)
}

pub fn extract_words_from_image_timed(
    image: &DynamicImage,
    engine: &OcrEngine,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let (words, timings) = engine
        .recognize_timed(image)
        .map_err(|e| crate::error::ExtractError::Ocr(e.to_string()))?;
    Ok((convert_words(words), convert_timings(timings)))
}

pub fn extract_words_timed(
    image_path: &Path,
    engine: &OcrEngine,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let image = image::open(image_path)
        .map_err(|_| crate::error::ExtractError::ImageRead(image_path.display().to_string()))?;
    extract_words_from_image_timed(&image, engine)
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

fn convert_timings(t: crate::infer::OcrTimings) -> OcrTimings {
    OcrTimings {
        init_ms: t.init_ms,
        predict_ms: t.predict_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::OcrBounds;

    #[test]
    fn ocr_config_defaults() {
        let config = OcrConfig::default();
        assert!(config.max_side > 0);
    }

    #[test]
    fn convert_bounds_roundtrip() {
        let core = crate::infer::OcrWord {
            text: "hi".into(),
            bounds: OcrBounds::new(1, 2, 3, 4),
            confidence: 99.0,
        };
        let word = convert_words(vec![core]).pop().unwrap();
        assert_eq!(word.bounds, Bounds::new(1, 2, 3, 4));
    }
}
