mod pack;
mod preprocess;

use std::time::Instant;

use image::{DynamicImage, GrayImage};

use crate::{
    types::{Bounds, UiElement, UiElementKind},
};

pub use pack::{IconMatchHit, IconMatchOptions, IconPack};
pub use preprocess::{icon_crop_to_rgb256, EMBED_DIM, INPUT_SIZE};

#[derive(Debug, Clone)]
pub struct IconConfig {
    pub template_size: u32,
    pub min_cosine: f64,
    pub min_side: i32,
    pub max_side: i32,
    pub min_aspect: f64,
    pub max_aspect: f64,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            template_size: 48,
            min_cosine: 0.72,
            min_side: 12,
            max_side: 96,
            min_aspect: 0.55,
            max_aspect: 1.85,
        }
    }
}

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct IconEmbedDetail {
    pub resize_ms: f64,
    pub pack_nchw_ms: f64,
    pub copy_input_ms: f64,
    pub run_session_ms: f64,
    pub read_output_ms: f64,
    pub finalize_ms: f64,
    pub batch_runs: u32,
    pub image_count: u32,
}

impl From<crate::infer::EmbedTimings> for IconEmbedDetail {
    fn from(t: crate::infer::EmbedTimings) -> Self {
        Self {
            resize_ms: t.resize_ms,
            pack_nchw_ms: t.pack_nchw_ms,
            copy_input_ms: t.copy_input_ms,
            run_session_ms: t.run_session_ms,
            read_output_ms: t.read_output_ms,
            finalize_ms: t.finalize_ms,
            batch_runs: t.batch_runs,
            image_count: t.image_count,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IconTimings {
    pub load_ms: f64,
    /// Grayscale conversion at the start of the icon pass.
    pub gray_ms: f64,
    /// Per-candidate crop from the screenshot gray buffer.
    pub crop_ms: f64,
    /// Mask normalization + 256×256 RGB render per candidate.
    pub preprocess_ms: f64,
    /// MobileCLIP embed inference per candidate (usually dominates).
    pub embed_ms: f64,
    /// Cosine search in the icon index per candidate.
    pub index_ms: f64,
    /// Wall time for the entire icon pass (includes tree walk overhead).
    pub match_ms: f64,
    /// Native embed stage breakdown from infer-core (when available).
    #[serde(default)]
    pub embed_detail: IconEmbedDetail,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IconMatchStats {
    pub candidates: usize,
    pub matched: usize,
    pub timings: IconTimings,
}

pub fn attach_icons_with_pack(
    root: &mut UiElement,
    source: &DynamicImage,
    pack: &mut IconPack,
    config: &IconConfig,
) -> IconMatchStats {
    let match_start = Instant::now();
    let gray_start = Instant::now();
    let gray = crate::layout::to_gray(source);
    let mut stats = IconMatchStats {
        candidates: 0,
        matched: 0,
        timings: IconTimings {
            gray_ms: gray_start.elapsed().as_secs_f64() * 1000.0,
            ..IconTimings::default()
        },
    };

    let mut jobs = Vec::new();
    collect_icon_jobs(root, &mut Vec::new(), config, &mut jobs);
    if jobs.is_empty() {
        stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
        return stats;
    }

    let mut valid_jobs = Vec::with_capacity(jobs.len());
    let mut gray_crops = Vec::with_capacity(jobs.len());
    for job in jobs {
        let crop_start = Instant::now();
        if let Some(crop) = crop_gray(&gray, &job.bounds) {
            stats.timings.crop_ms += crop_start.elapsed().as_secs_f64() * 1000.0;
            gray_crops.push(crop);
            valid_jobs.push(job);
        } else {
            stats.timings.crop_ms += crop_start.elapsed().as_secs_f64() * 1000.0;
        }
    }
    stats.candidates = valid_jobs.len();

    if valid_jobs.is_empty() {
        stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
        return stats;
    }

    let preprocess_start = Instant::now();
    let rgbs: Vec<_> = gray_crops
        .iter()
        .map(|crop| icon_crop_to_rgb256(crop, pack.template_size))
        .collect();
    stats.timings.preprocess_ms += preprocess_start.elapsed().as_secs_f64() * 1000.0;

    let embed_start = Instant::now();
    let (embeddings, embed_detail) = match pack.embed_rgb256_batch_timed(&rgbs) {
        Ok(v) => v,
        Err(_) => {
            stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
            return stats;
        }
    };
    stats.timings.embed_ms += embed_start.elapsed().as_secs_f64() * 1000.0;
    stats.timings.embed_detail = embed_detail.into();

    let index_start = Instant::now();
    let embedding_refs: Vec<&[f32]> = embeddings.iter().map(|e| e.as_slice()).collect();
    let hits = pack
        .match_embeddings_batch(&embedding_refs)
        .unwrap_or_default();
    stats.timings.index_ms += index_start.elapsed().as_secs_f64() * 1000.0;

    for (job, hit) in valid_jobs.iter().zip(hits) {
        if let Some(hit) = hit {
            if let Some(node) = node_at_path_mut(root, &job.path) {
                let bounds = node.bounds;
                *node = UiElement::icon(bounds, hit.name, Some(hit.score as f32));
                stats.matched += 1;
            }
        }
    }

    stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
    stats
}

#[derive(Debug, Clone)]
struct IconJob {
    path: Vec<usize>,
    bounds: Bounds,
}

fn collect_icon_jobs(
    node: &UiElement,
    path: &mut Vec<usize>,
    config: &IconConfig,
    jobs: &mut Vec<IconJob>,
) {
    for (i, child) in node.children.iter().enumerate() {
        if is_icon_candidate(child, config) {
            jobs.push(IconJob {
                path: {
                    let mut p = path.clone();
                    p.push(i);
                    p
                },
                bounds: child.bounds,
            });
        }
        path.push(i);
        collect_icon_jobs(child, path, config, jobs);
        path.pop();
    }
}

fn node_at_path_mut<'a>(root: &'a mut UiElement, path: &[usize]) -> Option<&'a mut UiElement> {
    let mut node = root;
    for &i in path {
        node = node.children.get_mut(i)?;
    }
    Some(node)
}

fn is_icon_candidate(element: &UiElement, config: &IconConfig) -> bool {
    if !matches!(element.kind, UiElementKind::Container) {
        return false;
    }
    if !element.children.is_empty() {
        return false;
    }
    if has_text_descendant(element) {
        return false;
    }

    let b = &element.bounds;
    let short = b.width.min(b.height);
    let long = b.width.max(b.height);
    if short < config.min_side || long > config.max_side {
        return false;
    }

    let aspect = b.width as f64 / b.height.max(1) as f64;
    aspect >= config.min_aspect && aspect <= config.max_aspect
}

fn has_text_descendant(element: &UiElement) -> bool {
    element.children.iter().any(|child| {
        matches!(child.kind, UiElementKind::Text { .. }) || has_text_descendant(child)
    })
}

fn crop_gray(gray: &GrayImage, bounds: &Bounds) -> Option<GrayImage> {
    let (img_w, img_h) = gray.dimensions();
    let x0 = bounds.x.max(0) as u32;
    let y0 = bounds.y.max(0) as u32;
    let x1 = bounds.right().min(img_w as i32) as u32;
    let y1 = bounds.bottom().min(img_h as i32) as u32;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    let w = x1 - x0;
    let h = y1 - y0;
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(x, y, *gray.get_pixel(x0 + x, y0 + y));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_square_leaf_container() {
        let el = UiElement::container(Bounds::new(0, 0, 24, 24), vec![]);
        assert!(is_icon_candidate(&el, &IconConfig::default()));

        let with_text = UiElement::container(
            Bounds::new(0, 0, 24, 24),
            vec![UiElement::text(Bounds::new(2, 2, 20, 20), "x".into(), None)],
        );
        assert!(!is_icon_candidate(&with_text, &IconConfig::default()));
    }

    #[test]
    fn rejects_wide_bar() {
        let el = UiElement::container(Bounds::new(0, 0, 80, 16), vec![]);
        assert!(!is_icon_candidate(&el, &IconConfig::default()));
    }
}
