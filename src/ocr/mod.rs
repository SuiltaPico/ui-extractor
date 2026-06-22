use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use image::{DynamicImage, RgbImage};
use oar_ocr::oarocr::{OAROCR, OAROCRBuilder, TextRegion};
use oar_ocr::processors::BoundingBox;
use oar_ocr::utils::load_image;

use crate::{
    error::{ExtractError, Result},
    types::Bounds,
};

#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Directory containing det/rec ONNX models and dictionary.
    pub model_dir: PathBuf,
    pub min_confidence: f32,
    /// OCR input long-edge limit (0 = no downscale).
    pub max_side: u32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            model_dir: PathBuf::from("models"),
            min_confidence: 0.5,
            max_side: 960,
        }
    }
}

impl OcrConfig {
    pub fn det_model(&self) -> PathBuf {
        self.model_dir.join("pp-ocrv5_mobile_det.onnx")
    }

    pub fn rec_model(&self) -> PathBuf {
        self.model_dir.join("pp-ocrv5_mobile_rec.onnx")
    }

    pub fn dict_path(&self) -> PathBuf {
        self.model_dir.join("ppocrv5_dict.txt")
    }
}

#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub bounds: Bounds,
    pub confidence: f32,
}

fn ocr_engine() -> &'static Mutex<Option<CachedOcr>> {
    static ENGINE: OnceLock<Mutex<Option<CachedOcr>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(None))
}

struct CachedOcr {
    key: String,
    engine: OAROCR,
}

#[derive(Debug, Clone, Default)]
pub struct OcrTimings {
    /// ONNX model load / engine init (0 after first case with same models).
    pub init_ms: f64,
    pub predict_ms: f64,
}

/// Run PaddleOCR (PP-OCRv5 mobile via oar-ocr) and return line-level words.
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
    let rgb = image.to_rgb8();
    extract_words_from_rgb_timed(rgb, config)
}

pub fn extract_words_timed(
    image_path: &Path,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let rgb = load_image(image_path).map_err(|e| ExtractError::Ocr(e.to_string()))?;
    extract_words_from_rgb_timed(rgb, config)
}

fn extract_words_from_rgb_timed(
    rgb: RgbImage,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let mut timings = OcrTimings::default();
    let (rgb, coord_scale) = resize_rgb_for_ocr(rgb, config.max_side);

    for path in [config.det_model(), config.rec_model(), config.dict_path()] {
        if !path.is_file() {
            return Err(ExtractError::Ocr(format!(
                "OCR model file not found: {} (run scripts/download_models.ps1)",
                path.display()
            )));
        }
    }

    let key = format!(
        "{}|{}|{}",
        config.det_model().display(),
        config.rec_model().display(),
        config.dict_path().display()
    );

    let mut guard = ocr_engine()
        .lock()
        .map_err(|e| ExtractError::Ocr(format!("OCR engine lock poisoned: {e}")))?;

    let needs_rebuild = guard
        .as_ref()
        .map(|cached| cached.key != key)
        .unwrap_or(true);

    if needs_rebuild {
        let init_start = Instant::now();
        let engine = OAROCRBuilder::new(
            config.det_model(),
            config.rec_model(),
            config.dict_path(),
        )
        .build()
        .map_err(|e| ExtractError::Ocr(e.to_string()))?;
        timings.init_ms = ms_since(init_start);
        *guard = Some(CachedOcr { key, engine });
    }

    let predict_start = Instant::now();
    let engine = &guard.as_ref().expect("engine initialized").engine;
    let results = engine
        .predict(vec![rgb])
        .map_err(|e| ExtractError::Ocr(e.to_string()))?;
    timings.predict_ms = ms_since(predict_start);

    let Some(result) = results.into_iter().next() else {
        return Ok((vec![], timings));
    };

    let mut words = Vec::new();
    for region in result.text_regions {
        words.extend(region_to_words(&region, config.min_confidence));
    }
    if coord_scale != 1.0 {
        for word in &mut words {
            word.bounds = scale_bounds(word.bounds, coord_scale);
        }
    }
    Ok((words, timings))
}

/// Downscale so the long edge is at most `max_side`. Returns `(image, coord_scale)` where
/// `coord_scale` maps OCR coordinates back to the original image space.
fn resize_rgb_for_ocr(rgb: RgbImage, max_side: u32) -> (RgbImage, f32) {
    if max_side == 0 {
        return (rgb, 1.0);
    }

    let (width, height) = rgb.dimensions();
    let longest = width.max(height);
    if longest <= max_side {
        return (rgb, 1.0);
    }

    let scale = max_side as f32 / longest as f32;
    let new_width = ((width as f32 * scale).round() as u32).max(1);
    let new_height = ((height as f32 * scale).round() as u32).max(1);
    let resized = image::imageops::resize(
        &rgb,
        new_width,
        new_height,
        image::imageops::FilterType::Triangle,
    );
    let coord_scale = width as f32 / new_width as f32;
    (resized, coord_scale)
}

fn scale_bounds(bounds: Bounds, coord_scale: f32) -> Bounds {
    if coord_scale == 1.0 {
        return bounds;
    }

    Bounds::new(
        (bounds.x as f32 * coord_scale).round() as i32,
        (bounds.y as f32 * coord_scale).round() as i32,
        (bounds.width as f32 * coord_scale).round().max(1.0) as i32,
        (bounds.height as f32 * coord_scale).round().max(1.0) as i32,
    )
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn region_to_words(region: &TextRegion, min_confidence: f32) -> Vec<OcrWord> {
    let Some(text) = region.text.as_ref().map(|t| t.trim()).filter(|t| !t.is_empty()) else {
        return vec![];
    };

    let confidence = region.confidence.unwrap_or(0.0);
    if confidence < min_confidence {
        return vec![];
    }

    let display_confidence = confidence * 100.0;

    if let Some(word_boxes) = &region.word_boxes {
        if !word_boxes.is_empty() {
            return word_boxes_to_words(text, word_boxes, display_confidence, min_confidence);
        }
    }

    vec![OcrWord {
        text: text.to_string(),
        bounds: bbox_to_bounds(&region.bounding_box),
        confidence: display_confidence,
    }]
}

fn word_boxes_to_words(
    text: &str,
    word_boxes: &[BoundingBox],
    line_confidence: f32,
    min_confidence: f32,
) -> Vec<OcrWord> {
    if line_confidence < min_confidence * 100.0 {
        return vec![];
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() == word_boxes.len() {
        return tokens
            .iter()
            .zip(word_boxes.iter())
            .map(|(token, bbox)| OcrWord {
                text: (*token).to_string(),
                bounds: bbox_to_bounds(bbox),
                confidence: line_confidence,
            })
            .collect();
    }

    if word_boxes.len() == text.chars().count() {
        return word_boxes
            .iter()
            .zip(text.chars())
            .map(|(bbox, ch)| OcrWord {
                text: ch.to_string(),
                bounds: bbox_to_bounds(bbox),
                confidence: line_confidence,
            })
            .collect();
    }

    vec![OcrWord {
        text: text.to_string(),
        bounds: bbox_to_bounds(&word_boxes[0]),
        confidence: line_confidence,
    }]
}

fn bbox_to_bounds(bbox: &BoundingBox) -> Bounds {
    let x_min = bbox.x_min();
    let y_min = bbox.y_min();
    let x_max = bbox.x_max();
    let y_max = bbox.y_max();
    let width = (x_max - x_min).round().max(1.0) as i32;
    let height = (y_max - y_min).round().max(1.0) as i32;
    Bounds::new(x_min.round() as i32, y_min.round() as i32, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn bbox_to_bounds_uses_axis_aligned_box() {
        let bbox = BoundingBox::from_coords(10.2, 20.7, 50.9, 40.1);
        let bounds = bbox_to_bounds(&bbox);
        assert_eq!(bounds.x, 10);
        assert_eq!(bounds.y, 21);
        assert_eq!(bounds.width, 41);
        assert_eq!(bounds.height, 19);
    }

    #[test]
    fn region_to_words_skips_low_confidence() {
        let region = TextRegion::with_recognition(
            BoundingBox::from_coords(0.0, 0.0, 10.0, 10.0),
            Some(Arc::from("hello")),
            Some(0.2),
        );
        assert!(region_to_words(&region, 0.5).is_empty());
    }

    #[test]
    fn resize_rgb_for_ocr_keeps_small_images() {
        let rgb = RgbImage::new(800, 600);
        let (out, scale) = resize_rgb_for_ocr(rgb, 960);
        assert_eq!(out.dimensions(), (800, 600));
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn resize_rgb_for_ocr_scales_long_edge() {
        let rgb = RgbImage::new(1920, 873);
        let (out, scale) = resize_rgb_for_ocr(rgb, 960);
        assert_eq!(out.dimensions(), (960, 437));
        assert!((scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn scale_bounds_maps_back_to_original_space() {
        let scaled = Bounds::new(10, 20, 30, 40);
        let original = scale_bounds(scaled, 2.0);
        assert_eq!(original, Bounds::new(20, 40, 60, 80));
    }
}
