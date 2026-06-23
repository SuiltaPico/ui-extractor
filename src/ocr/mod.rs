use std::path::{Path, PathBuf};

use image::{DynamicImage, RgbImage};

use crate::{
    error::Result,
    types::Bounds,
};

#[cfg(feature = "backend-ncnn")]
mod ncnn;
#[cfg(feature = "backend-ort")]
mod ort;

#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Directory containing det/rec models and dictionary.
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
        #[cfg(feature = "backend-ort")]
        {
            self.model_dir.join("pp-ocrv5_mobile_det.onnx")
        }
        #[cfg(feature = "backend-ncnn")]
        {
            self.model_dir.join("pp-ocrv5_mobile_det.ncnn.param")
        }
    }

    pub fn rec_model(&self) -> PathBuf {
        #[cfg(feature = "backend-ort")]
        {
            self.model_dir.join("pp-ocrv5_mobile_rec.onnx")
        }
        #[cfg(feature = "backend-ncnn")]
        {
            self.model_dir.join("pp-ocrv5_mobile_rec.ncnn.param")
        }
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

#[derive(Debug, Clone, Default)]
pub struct OcrTimings {
    /// Model load / engine init (0 after first case with same models).
    pub init_ms: f64,
    pub predict_ms: f64,
}

/// Run PaddleOCR PP-OCRv5 mobile and return line-level words.
pub fn extract_words(image_path: &Path, config: &OcrConfig) -> Result<Vec<OcrWord>> {
    backend::extract_words(image_path, config)
}

pub fn extract_words_from_image(image: &DynamicImage, config: &OcrConfig) -> Result<Vec<OcrWord>> {
    backend::extract_words_from_image(image, config)
}

pub fn extract_words_from_image_timed(
    image: &DynamicImage,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    backend::extract_words_from_image_timed(image, config)
}

pub fn extract_words_timed(
    image_path: &Path,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    backend::extract_words_timed(image_path, config)
}

#[cfg(feature = "backend-ort")]
mod backend {
    pub use super::ort::*;
}

#[cfg(feature = "backend-ncnn")]
mod backend {
    pub use super::ncnn::*;
}

/// Downscale so the long edge is at most `max_side`. Returns `(image, coord_scale)` where
/// `coord_scale` maps OCR coordinates back to the original image space.
pub(crate) fn resize_rgb_for_ocr(rgb: RgbImage, max_side: u32) -> (RgbImage, f32) {
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

pub(crate) fn scale_bounds(bounds: Bounds, coord_scale: f32) -> Bounds {
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

pub(crate) fn ms_since(start: std::time::Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

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
