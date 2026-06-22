use std::fs;
use std::path::Path;

use image::GrayImage;
use rayon::prelude::*;

use crate::error::{ExtractError, Result};

use super::embedding::EmbeddingIndex;

/// Binary ink mask for a single MDI template (`1` = ink, `0` = background).
#[derive(Debug, Clone)]
pub struct IconTemplate {
    pub name: String,
    pub mask: Vec<u8>,
    pub ink_count: u32,
}

#[derive(Debug)]
pub struct IconLibrary {
    pub size: u32,
    pub templates: Vec<IconTemplate>,
    pub embeddings: EmbeddingIndex,
    name_to_template: std::collections::HashMap<String, usize>,
}

impl IconLibrary {
    pub fn load(dir: &Path, size: u32, embeddings: EmbeddingIndex) -> Result<Self> {
        if !dir.is_dir() {
            return Err(ExtractError::Image(format!(
                "MDI PNG directory not found: {}",
                dir.display()
            )));
        }

        let mut paths: Vec<_> = fs::read_dir(dir)
            .map_err(|e| ExtractError::Image(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
            .collect();
        paths.sort();

        if paths.is_empty() {
            return Err(ExtractError::Image(format!(
                "no PNG templates under {}",
                dir.display()
            )));
        }

        let templates: Vec<IconTemplate> = paths
            .par_iter()
            .filter_map(|path| load_template(path, size).ok())
            .collect();

        if templates.is_empty() {
            return Err(ExtractError::Image(format!(
                "failed to load any templates from {}",
                dir.display()
            )));
        }

        let name_to_template = templates
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();

        Ok(Self {
            size,
            templates,
            embeddings,
            name_to_template,
        })
    }

    /// Cosine retrieval with optional IoU rerank among top-k candidates.
    pub fn best_match(
        &self,
        query_embedding: &[f32],
        query_mask: &[u8],
        min_cosine: f64,
        rerank_top_k: usize,
        min_iou: f64,
    ) -> Option<(String, f64)> {
        if query_mask.len() != (self.size * self.size) as usize {
            return None;
        }

        let top = self.embeddings.top_k(query_embedding, rerank_top_k.max(1));
        if top.is_empty() {
            return None;
        }

        let (best_idx, best_cosine) = top[0];
        if best_cosine < min_cosine {
            return None;
        }

        if rerank_top_k <= 1 || min_iou <= 0.0 {
            return Some((
                self.embeddings.names[best_idx].clone(),
                best_cosine,
            ));
        }

        let mut best: Option<(String, f64)> = None;
        for (idx, cosine) in top {
            let name = &self.embeddings.names[idx];
            let iou = self
                .name_to_template
                .get(name)
                .map(|&ti| mask_iou(query_mask, &self.templates[ti].mask, self.templates[ti].ink_count))
                .unwrap_or(0.0);

            let accept = cosine >= min_cosine && (iou >= min_iou || cosine >= 0.85);
            if !accept {
                continue;
            }

            let score = cosine * 0.7 + iou * 0.3;
            if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
                best = Some((name.clone(), score));
            }
        }

        best.or_else(|| {
            if best_cosine >= 0.85 {
                Some((self.embeddings.names[best_idx].clone(), best_cosine))
            } else {
                None
            }
        })
    }

    /// Legacy mask-only match (used in unit tests).
    pub fn best_match_mask(&self, query_mask: &[u8]) -> Option<(String, f64)> {
        if query_mask.len() != (self.size * self.size) as usize {
            return None;
        }

        self.templates
            .par_iter()
            .filter_map(|template| {
                let score = mask_iou(query_mask, &template.mask, template.ink_count);
                Some((template.name.clone(), score))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }
}

fn load_template(path: &Path, size: u32) -> Result<IconTemplate> {
    let img = image::open(path).map_err(|e| ExtractError::Image(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w != size || h != size {
        return Err(ExtractError::Image(format!(
            "expected {size}x{size} PNG, got {w}x{h}: {}",
            path.display()
        )));
    }

    let mut mask = Vec::with_capacity((size * size) as usize);
    let mut ink_count = 0u32;
    for pixel in rgba.pixels() {
        let ink = pixel[3] > 16 && luminance(pixel) < 200;
        mask.push(ink as u8);
        if ink {
            ink_count += 1;
        }
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(IconTemplate {
        name,
        mask,
        ink_count,
    })
}

fn luminance(pixel: &image::Rgba<u8>) -> u8 {
    let r = pixel[0] as u32;
    let g = pixel[1] as u32;
    let b = pixel[2] as u32;
    ((r * 299 + g * 587 + b * 114) / 1000) as u8
}

/// Jaccard similarity on ink pixels.
pub fn mask_iou(query: &[u8], template: &[u8], template_ink: u32) -> f64 {
    let mut intersection = 0u32;
    let mut query_ink = 0u32;
    for (q, t) in query.iter().zip(template.iter()) {
        let q_ink = *q > 0;
        let t_ink = *t > 0;
        if q_ink {
            query_ink += 1;
        }
        if q_ink && t_ink {
            intersection += 1;
        }
    }

    let union = query_ink + template_ink - intersection;
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Resize a grayscale crop and derive an ink mask, trying both polarities.
pub fn normalize_query_mask(gray_crop: &GrayImage, size: u32) -> (Vec<u8>, bool) {
    let resized = image::imageops::resize(
        gray_crop,
        size,
        size,
        image::imageops::FilterType::Triangle,
    );

    let dark = ink_mask_from_gray(&resized, true);
    let light = ink_mask_from_gray(&resized, false);
    let dark_ink = dark.iter().filter(|&&v| v > 0).count();
    let light_ink = light.iter().filter(|&&v| v > 0).count();

    let total = (size * size) as usize;
    let dark_ok = ink_ratio_ok(dark_ink, total);
    let light_ok = ink_ratio_ok(light_ink, total);

    match (dark_ok, light_ok) {
        (true, true) => {
            if dark_ink <= light_ink {
                (dark, true)
            } else {
                (light, false)
            }
        }
        (true, false) => (dark, true),
        (false, true) => (light, false),
        (false, false) => {
            if dark_ink.abs_diff(light_ink) <= 8 {
                (dark, true)
            } else if dark_ink < light_ink {
                (dark, true)
            } else {
                (light, false)
            }
        }
    }
}

fn ink_ratio_ok(ink: usize, total: usize) -> bool {
    if total == 0 {
        return false;
    }
    let ratio = ink as f64 / total as f64;
    (0.03..=0.72).contains(&ratio)
}

fn ink_mask_from_gray(img: &GrayImage, dark_icon: bool) -> Vec<u8> {
    let bg = estimate_background(img);
    let threshold = adaptive_threshold(img, bg);

    img.pixels()
        .map(|p| {
            let v = p.0[0];
            let ink = if dark_icon {
                v.saturating_add(threshold) < bg
            } else {
                v.saturating_sub(threshold) > bg
            };
            ink as u8
        })
        .collect()
}

fn estimate_background(img: &GrayImage) -> u8 {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 255;
    }

    let mut samples = Vec::new();
    for x in 0..w {
        samples.push(img.get_pixel(x, 0).0[0]);
        samples.push(img.get_pixel(x, h - 1).0[0]);
    }
    for y in 1..h.saturating_sub(1) {
        samples.push(img.get_pixel(0, y).0[0]);
        samples.push(img.get_pixel(w - 1, y).0[0]);
    }

    if samples.is_empty() {
        return 128;
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn adaptive_threshold(img: &GrayImage, bg: u8) -> u8 {
    let values: Vec<u8> = img.pixels().map(|p| p.0[0]).collect();
    let mut diffs: Vec<u8> = values
        .iter()
        .map(|v| v.abs_diff(bg))
        .filter(|d| *d > 0)
        .collect();
    if diffs.is_empty() {
        return 24;
    }
    diffs.sort_unstable();
    let median = diffs[diffs.len() / 2];
    median.clamp(12, 48)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn identical_masks_score_one() {
        let mask = vec![0, 1, 1, 0];
        assert!((mask_iou(&mask, &mask, 2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn disjoint_masks_score_zero() {
        let a = vec![1, 0, 0, 0];
        let b = vec![0, 0, 0, 1];
        assert_eq!(mask_iou(&a, &b, 1), 0.0);
    }

    #[test]
    fn normalize_filled_square() {
        let mut img = GrayImage::from_pixel(32, 32, Luma([255]));
        for y in 8..24 {
            for x in 8..24 {
                img.put_pixel(x, y, Luma([0]));
            }
        }
        let (mask, dark) = normalize_query_mask(&img, 16);
        assert!(dark);
        assert!(mask.iter().filter(|&&v| v > 0).count() > 20);
    }
}
