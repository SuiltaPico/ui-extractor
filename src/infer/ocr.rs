use std::io::Cursor;

use image::DynamicImage;

use crate::infer::error::{InferError, Result};
use crate::infer::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OcrBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl OcrBounds {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub bounds: OcrBounds,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default)]
pub struct OcrTimings {
    pub init_ms: f64,
    pub predict_ms: f64,
    pub det_ms: f64,
    pub rec_ms: f64,
    pub post_ms: f64,
}

pub struct OcrEngine {
    pub(crate) handle: *mut std::ffi::c_void,
}

// infer_core OCR handles are safe to use from scoped threads (read-only recognize calls).
unsafe impl Send for OcrEngine {}
unsafe impl Sync for OcrEngine {}

impl OcrEngine {
    pub fn apply_config_overrides(&mut self, min_confidence: Option<f32>, max_side: Option<u32>) {
        let _ = ffi::ocr_engine_apply_config(
            self.handle,
            min_confidence.unwrap_or(0.5),
            max_side.unwrap_or(960),
        );
    }

    pub fn recognize_timed(&self, image: &DynamicImage) -> Result<(Vec<OcrWord>, OcrTimings)> {
        let bytes = encode_image(image)?;
        let json = ffi::ocr_recognize_timed(self.handle, &bytes)?;
        parse_recognize_json(&json)
    }

    pub fn recognize(&self, image: &DynamicImage) -> Result<Vec<OcrWord>> {
        self.recognize_timed(image).map(|(words, _)| words)
    }

    pub fn plain_text(&self, image: &DynamicImage) -> Result<String> {
        Ok(self
            .recognize(image)?
            .into_iter()
            .map(|w| w.text)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

impl Drop for OcrEngine {
    fn drop(&mut self) {
        ffi::ocr_engine_destroy(self.handle);
    }
}

fn encode_image(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| InferError::Ocr(e.to_string()))?;
    Ok(buf.into_inner())
}

fn parse_recognize_json(json: &str) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let words = value
        .get("words")
        .and_then(|v| v.as_array())
        .ok_or_else(|| InferError::Ocr("missing words in OCR JSON".into()))?
        .iter()
        .map(parse_word)
        .collect::<Result<Vec<_>>>()?;
    let timings_value = value
        .get("timings")
        .ok_or_else(|| InferError::Ocr("missing timings in OCR JSON".into()))?;
    let timings = OcrTimings {
        init_ms: timings_value
            .get("init_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        predict_ms: timings_value
            .get("predict_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        det_ms: timings_value
            .get("det_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        rec_ms: timings_value
            .get("rec_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        post_ms: timings_value
            .get("post_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    };
    Ok((words, timings))
}

fn parse_word(value: &serde_json::Value) -> Result<OcrWord> {
    let text = value
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let bounds = value
        .get("bounds")
        .ok_or_else(|| InferError::Ocr("missing word bounds".into()))?;
    Ok(OcrWord {
        text,
        bounds: OcrBounds::new(
            bounds.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            bounds.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            bounds.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            bounds.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        ),
        confidence: value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32,
    })
}
