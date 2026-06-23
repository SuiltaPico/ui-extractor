use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use image::{DynamicImage, GrayImage, RgbImage};
use imageproc::contours::find_contours;
use ncnn_bind::{Mat, MatPixelType, Net};

use crate::error::{ExtractError, Result};
use crate::inference::ncnn::{load_net, mat_to_vec_f32};
use crate::types::Bounds;

use super::{ms_since, scale_bounds, OcrConfig, OcrTimings, OcrWord};

const DET_TARGET_SIZE: u32 = 960;
const DET_STRIDE: u32 = 32;
const DET_THRESHOLD: f32 = 0.3;
const BOX_THRESH: f32 = 0.6;
const ENLARGE_RATIO: f32 = 1.95;
const REC_HEIGHT: u32 = 48;

fn ocr_engine() -> &'static Mutex<Option<CachedOcr>> {
    static ENGINE: OnceLock<Mutex<Option<CachedOcr>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(None))
}

struct CachedOcr {
    key: String,
    det: Net,
    rec: Net,
    dict: Vec<String>,
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
    extract_words_from_rgb_timed(image.to_rgb8(), config)
}

pub fn extract_words_timed(
    image_path: &Path,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let rgb = image::open(image_path)
        .map_err(|e| ExtractError::Ocr(e.to_string()))?
        .to_rgb8();
    extract_words_from_rgb_timed(rgb, config)
}

pub fn extract_words_from_rgb_timed(
    rgb: RgbImage,
    config: &OcrConfig,
) -> Result<(Vec<OcrWord>, OcrTimings)> {
    let mut timings = OcrTimings::default();
    let (rgb, coord_scale) = super::resize_rgb_for_ocr(rgb, config.max_side);

    for path in [config.det_model(), config.rec_model(), config.dict_path()] {
        if !path.is_file() {
            return Err(ExtractError::Ocr(format!(
                "OCR ncnn model not found: {} (run scripts/convert_models_ncnn.ps1 or scripts/download_models_ncnn.ps1)",
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
        let det = load_net(&config.det_model()).map_err(map_ncnn_err)?;
        let rec = load_net(&config.rec_model()).map_err(map_ncnn_err)?;
        let dict = load_dict(&config.dict_path())?;
        timings.init_ms = ms_since(init_start);
        *guard = Some(CachedOcr {
            key,
            det,
            rec,
            dict,
        });
    }

    let predict_start = Instant::now();
    let engine = guard.as_ref().expect("engine initialized");
    let boxes = detect_text_boxes(&engine.det, &rgb)?;
    let mut words = Vec::new();
    for bbox in boxes {
        let text = recognize_crop(&engine.rec, &engine.dict, &rgb, &bbox)?;
        if text.is_empty() {
            continue;
        }
        let confidence = 100.0;
        if confidence / 100.0 < config.min_confidence {
            continue;
        }
        words.push(OcrWord {
            text,
            bounds: bbox,
            confidence,
        });
    }
    timings.predict_ms = ms_since(predict_start);

    if coord_scale != 1.0 {
        for word in &mut words {
            word.bounds = scale_bounds(word.bounds, coord_scale);
        }
    }
    Ok((words, timings))
}

fn map_ncnn_err(err: ExtractError) -> ExtractError {
    match err {
        ExtractError::Image(msg) => ExtractError::Ocr(msg),
        other => other,
    }
}

fn load_dict(path: &Path) -> Result<Vec<String>> {
    let raw = fs::read_to_string(path).map_err(|e| ExtractError::Ocr(e.to_string()))?;
    Ok(raw.lines().map(|line| line.to_string()).collect())
}

struct DetContext {
    scale: f32,
    pad_left: u32,
    pad_top: u32,
    prob_w: u32,
    prob_h: u32,
}

fn detect_text_boxes(det: &Net, rgb: &RgbImage) -> Result<Vec<Bounds>> {
    let (img_w, img_h) = rgb.dimensions();
    let mut w = img_w;
    let mut h = img_h;
    let mut scale = 1.0f32;

    let target = w.max(h).min(DET_TARGET_SIZE);
    if w.max(h) > target {
        if w > h {
            scale = target as f32 / w as f32;
            w = target;
            h = ((img_h as f32 * scale).round() as u32).max(1);
        } else {
            scale = target as f32 / h as f32;
            h = target;
            w = ((img_w as f32 * scale).round() as u32).max(1);
        }
    }

    let resized = if w == img_w && h == img_h {
        rgb.clone()
    } else {
        image::imageops::resize(rgb, w, h, image::imageops::FilterType::Triangle)
    };

    let pad_w = ((w + DET_STRIDE - 1) / DET_STRIDE) * DET_STRIDE;
    let pad_h = ((h + DET_STRIDE - 1) / DET_STRIDE) * DET_STRIDE;
    let pad_left = (pad_w - w) / 2;
    let pad_top = (pad_h - h) / 2;
    let pad_right = pad_w - w - pad_left;
    let pad_bottom = pad_h - h - pad_top;

    let padded = pad_rgb(&resized, pad_left, pad_top, pad_right, pad_bottom, 114);

    let mut in_mat = Mat::from_pixels(
        padded.as_raw(),
        MatPixelType::RGB,
        pad_w as i32,
        pad_h as i32,
        None,
    )
    .map_err(|e| ExtractError::Ocr(format!("ncnn det input: {e}")))?;

    let mean = [0.485f32 * 255.0, 0.456 * 255.0, 0.406 * 255.0];
    let norm = [
        1.0 / (0.229 * 255.0),
        1.0 / (0.224 * 255.0),
        1.0 / (0.225 * 255.0),
    ];
    in_mat.substract_mean_normalize(&mean, &norm);

    let mut ex = det.create_extractor();
    ex.input("in0", &in_mat)
        .map_err(|e| ExtractError::Ocr(format!("ncnn det input: {e}")))?;

    let mut out = Mat::new();
    ex.extract("out0", &mut out)
        .map_err(|e| ExtractError::Ocr(format!("ncnn det extract: {e}")))?;

    let prob_w = out.w() as u32;
    let prob_h = out.h() as u32;
    let probs = mat_to_vec_f32(&out).map_err(map_ncnn_err)?;

    let ctx = DetContext {
        scale,
        pad_left,
        pad_top,
        prob_w,
        prob_h,
    };

    boxes_from_prob_map(&probs, prob_w, prob_h, &ctx)
}

fn pad_rgb(
    img: &RgbImage,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    value: u8,
) -> RgbImage {
    let (w, h) = img.dimensions();
    let out_w = w + left + right;
    let out_h = h + top + bottom;
    let fill = image::Rgb([value, value, value]);
    let mut out = RgbImage::from_pixel(out_w, out_h, fill);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(x + left, y + top, *img.get_pixel(x, y));
        }
    }
    out
}

fn boxes_from_prob_map(
    probs: &[f32],
    width: u32,
    height: u32,
    ctx: &DetContext,
) -> Result<Vec<Bounds>> {
    let mut binary = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let p = probs[(y * width + x) as usize];
            let v = if p >= DET_THRESHOLD { 255u8 } else { 0u8 };
            binary.put_pixel(x, y, image::Luma([v]));
        }
    }

    let contours = find_contours::<u32>(&binary);
    let min_size = (3.0 * ctx.scale).max(3.0);
    let mut boxes = Vec::new();

    for contour in contours {
        if contour.points.len() <= 2 {
            continue;
        }
        let score = contour_mean_score(probs, width, height, &contour.points);
        if score < BOX_THRESH {
            continue;
        }

        let (x0, y0, x1, y1) = contour_aabb(&contour.points);
        let bw = (x1 - x0) as f32;
        let bh = (y1 - y0) as f32;
        if bw.max(bh) < min_size {
            continue;
        }

        let cx = (x0 + x1) as f32 / 2.0;
        let cy = (y0 + y1) as f32 / 2.0;
        let mut w = bw * ENLARGE_RATIO;
        let mut h = bh * ENLARGE_RATIO;
        w = w.max(h * 0.3);
        h = h.max(w * 0.3);

        let mut x = cx - w / 2.0;
        let mut y = cy - h / 2.0;

        x = (x - ctx.pad_left as f32) / ctx.scale;
        y = (y - ctx.pad_top as f32) / ctx.scale;
        w /= ctx.scale;
        h /= ctx.scale;

        boxes.push(Bounds::new(
            x.round() as i32,
            y.round() as i32,
            w.round().max(1.0) as i32,
            h.round().max(1.0) as i32,
        ));
    }

    Ok(boxes)
}

fn contour_mean_score(probs: &[f32], width: u32, height: u32, points: &[imageproc::point::Point<u32>]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    if x1 <= x0 || y1 <= y0 {
        return 0.0;
    }

    let mut sum = 0.0f32;
    let mut count = 0u32;
    for y in y0..=y1.min(height - 1) {
        for x in x0..=x1.min(width - 1) {
            if point_in_contour(x, y, points) {
                sum += probs[(y * width + x) as usize];
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

fn point_in_contour(x: u32, y: u32, points: &[imageproc::point::Point<u32>]) -> bool {
    let mut inside = false;
    let n = points.len();
    for i in 0..n {
        let j = (i + n - 1) % n;
        let pi = points[i];
        let pj = points[j];
        let x_cross = (pj.x as f64 - pi.x as f64) * (y as f64 - pi.y as f64)
            / (pj.y as f64 - pi.y as f64 + f64::EPSILON)
            + pi.x as f64;
        let intersect = ((pi.y > y) != (pj.y > y)) && (x as f64) < x_cross;
        if intersect {
            inside = !inside;
        }
    }
    inside
}

fn contour_aabb(points: &[imageproc::point::Point<u32>]) -> (u32, u32, u32, u32) {
    let mut x0 = u32::MAX;
    let mut y0 = u32::MAX;
    let mut x1 = 0u32;
    let mut y1 = 0u32;
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x0, y0, x1, y1)
}

fn recognize_crop(rec: &Net, dict: &[String], rgb: &RgbImage, bounds: &Bounds) -> Result<String> {
    let crop = crop_rgb(rgb, bounds);
    if crop.width() == 0 || crop.height() == 0 {
        return Ok(String::new());
    }

    let (cw, ch) = crop.dimensions();
    let target_w = ((cw as f32 * REC_HEIGHT as f32 / ch as f32).round() as u32).max(1);
    let resized = if ch == REC_HEIGHT && cw == target_w {
        crop
    } else {
        image::imageops::resize(
            &crop,
            target_w,
            REC_HEIGHT,
            image::imageops::FilterType::Triangle,
        )
    };

    let mut in_mat = Mat::from_pixels(
        resized.as_raw(),
        MatPixelType::RGB,
        resized.width() as i32,
        resized.height() as i32,
        None,
    )
    .map_err(|e| ExtractError::Ocr(format!("ncnn rec input: {e}")))?;

    let mean = [127.5f32; 3];
    let norm = [1.0 / 127.5; 3];
    in_mat.substract_mean_normalize(&mean, &norm);

    let mut ex = rec.create_extractor();
    ex.input("in0", &in_mat)
        .map_err(|e| ExtractError::Ocr(format!("ncnn rec input: {e}")))?;

    let mut out = Mat::new();
    ex.extract("out0", &mut out)
        .map_err(|e| ExtractError::Ocr(format!("ncnn rec extract: {e}")))?;

    Ok(decode_ctc(&out, dict))
}

fn decode_ctc(out: &Mat, dict: &[String]) -> String {
    let w = out.w() as usize;
    let h = out.h() as usize;
    if w == 0 || h == 0 {
        return String::new();
    }

    let data = unsafe {
        std::slice::from_raw_parts(out.data() as *const f32, w * h)
    };

    let mut text = String::new();
    let mut last_token = 0usize;

    for t in 0..h {
        let row = &data[t * w..(t + 1) * w];
        let (mut best_idx, mut best_score) = (0usize, f32::NEG_INFINITY);
        for (j, &score) in row.iter().enumerate() {
            if score > best_score {
                best_score = score;
                best_idx = j;
            }
        }

        if best_idx == last_token {
            continue;
        }
        last_token = best_idx;
        if best_idx == 0 {
            continue;
        }
        let dict_idx = best_idx - 1;
        if dict_idx < dict.len() {
            text.push_str(&dict[dict_idx]);
        }
    }

    text
}

fn crop_rgb(rgb: &RgbImage, bounds: &Bounds) -> RgbImage {
    let (img_w, img_h) = rgb.dimensions();
    let x0 = bounds.x.max(0) as u32;
    let y0 = bounds.y.max(0) as u32;
    let x1 = bounds.right().min(img_w as i32) as u32;
    let y1 = bounds.bottom().min(img_h as i32) as u32;
    if x1 <= x0 || y1 <= y0 {
        return RgbImage::new(0, 0);
    }

    let w = x1 - x0;
    let h = y1 - y0;
    let mut out = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(x, y, *rgb.get_pixel(x0 + x, y0 + y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctc_skips_blank_and_repeats() {
        let dict = vec!["a".into(), "b".into()];
        // 3 timesteps x 3 classes (blank + a + b)
        let mut mat = Mat::new_2d(3, 3, None);
        mat.fill(0.0);
        let ptr = mat.data() as *mut f32;
        unsafe {
            let s = std::slice::from_raw_parts_mut(ptr, 9);
            s[1] = 1.0; // t0 -> a
            s[1] = 1.0;
            s[4] = 1.0; // t1 -> a (repeat, skip)
            s[8] = 1.0; // t2 -> b (index 2)
        }
        assert_eq!(decode_ctc(&mat, &dict), "ab");
    }
}
