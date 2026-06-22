mod embedding;
mod library;
mod preprocess;

use std::path::PathBuf;
use std::time::Instant;

use image::{DynamicImage, GrayImage};

use crate::{
    error::Result,
    types::{Bounds, UiElement, UiElementKind},
};

pub use embedding::{EmbeddingIndex, IconEmbedder};
pub use library::{IconLibrary, mask_iou, normalize_query_mask};
pub use preprocess::{icon_crop_to_rgb256, mdi_png_to_rgb256, EMBED_DIM, INPUT_SIZE};

#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Directory of rasterized MDI PNG templates (e.g. `assets/mdi/png-48-black`).
    pub mdi_png_dir: PathBuf,
    /// Precomputed embedding index (`embed-mdi` output).
    pub embedding_index: PathBuf,
    /// MobileCLIP2-S0 vision ONNX model path.
    pub vision_model: PathBuf,
    /// Template edge length in pixels (must match PNG files).
    pub template_size: u32,
    /// Minimum cosine similarity to accept a match (0–1).
    pub min_cosine: f64,
    /// Top-k candidates for IoU rerank (0 = embedding only).
    pub rerank_top_k: usize,
    /// Minimum mask IoU when reranking (ignored if rerank_top_k <= 1).
    pub min_iou: f64,
    /// Candidate box shorter side (px).
    pub min_side: i32,
    /// Candidate box longer side (px).
    pub max_side: i32,
    /// Width / height ratio limits.
    pub min_aspect: f64,
    pub max_aspect: f64,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            mdi_png_dir: PathBuf::from("assets/mdi/png-48-black"),
            embedding_index: PathBuf::from("assets/mdi/embeddings.bin"),
            vision_model: PathBuf::from("models/mobileclip2-s0-vision.onnx"),
            template_size: 48,
            min_cosine: 0.72,
            rerank_top_k: 10,
            min_iou: 0.35,
            min_side: 12,
            max_side: 96,
            min_aspect: 0.55,
            max_aspect: 1.85,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IconTimings {
    pub load_ms: f64,
    pub match_ms: f64,
}

#[derive(Debug, Clone, Default)]
pub struct IconMatchStats {
    pub candidates: usize,
    pub matched: usize,
    pub timings: IconTimings,
}

/// Identify icon candidates in the layout tree and replace matches with `kind: icon`.
pub fn attach_icons(
    root: &mut UiElement,
    source: &DynamicImage,
    library: &IconLibrary,
    embedder: &mut IconEmbedder,
    config: &IconConfig,
) -> IconMatchStats {
    let match_start = Instant::now();
    let gray = crate::layout::to_gray(source);
    let mut stats = IconMatchStats {
        candidates: 0,
        matched: 0,
        timings: IconTimings::default(),
    };

    walk_mut(root, &gray, library, embedder, config, &mut stats);
    stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
    stats
}

pub fn try_load_library(config: &IconConfig) -> Result<IconLibrary> {
    let embeddings = EmbeddingIndex::load(&config.embedding_index)?;
    IconLibrary::load(&config.mdi_png_dir, config.template_size, embeddings)
}

fn walk_mut(
    node: &mut UiElement,
    gray: &GrayImage,
    library: &IconLibrary,
    embedder: &mut IconEmbedder,
    config: &IconConfig,
    stats: &mut IconMatchStats,
) {
    for child in &mut node.children {
        walk_mut(child, gray, library, embedder, config, stats);
    }

    let mut i = 0;
    while i < node.children.len() {
        let child = &node.children[i];
        if is_icon_candidate(child, config) {
            stats.candidates += 1;
            if let Some((name, score)) =
                match_candidate(gray, &child.bounds, library, embedder, config)
            {
                let bounds = child.bounds;
                node.children[i] = UiElement::icon(bounds, name, Some(score as f32));
                stats.matched += 1;
            }
        }
        i += 1;
    }
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

fn match_candidate(
    gray: &GrayImage,
    bounds: &Bounds,
    library: &IconLibrary,
    embedder: &mut IconEmbedder,
    config: &IconConfig,
) -> Option<(String, f64)> {
    let crop = crop_gray(gray, bounds)?;
    let (mask, _) = normalize_query_mask(&crop, library.size);
    let rgb = icon_crop_to_rgb256(&crop, library.size);
    let embedding = embedder.embed_rgb256(&rgb).ok()?;
    let (name, score) = library.best_match(
        &embedding,
        &mask,
        config.min_cosine,
        config.rerank_top_k,
        config.min_iou,
    )?;
    Some((name, score))
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
    use std::path::Path;

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

    #[test]
    fn mdi_png_rgb_differs_between_icons() {
        let dir = Path::new("assets/mdi/png-48-black");
        if !dir.is_dir() {
            return;
        }
        let home = dir.join("home.png");
        let menu = dir.join("menu.png");
        if !home.is_file() || !menu.is_file() {
            return;
        }
        let home_rgb = mdi_png_to_rgb256(&image::open(home).unwrap(), 48);
        let menu_rgb = mdi_png_to_rgb256(&image::open(menu).unwrap(), 48);
        assert_ne!(home_rgb.as_raw(), menu_rgb.as_raw());
    }

    #[test]
    fn mdi_embedding_roundtrip_when_assets_present() {
        let dir = Path::new("assets/mdi/png-48-black");
        let model = Path::new("models/mobileclip2-s0-vision.onnx");
        if !dir.is_dir() || !model.is_file() {
            return;
        }

        let config = IconConfig::default();
        let library = match try_load_library(&config) {
            Ok(lib) => lib,
            Err(_) => return,
        };

        let name = "home";
        let png_path = dir.join(format!("{name}.png"));
        if !png_path.is_file() {
            return;
        }

        let mut embedder = IconEmbedder::load(model).unwrap();
        let img = image::open(&png_path).unwrap();
        let rgb = mdi_png_to_rgb256(&img, config.template_size);
        let embedding = embedder.embed_rgb256(&rgb).unwrap();
        let gray = crate::layout::to_gray(&img);
        let (mask, _) = normalize_query_mask(&gray, config.template_size);
        let (matched, score) = library
            .best_match(&embedding, &mask, 0.5, 10, 0.0)
            .unwrap();
        assert_eq!(matched, name);
        assert!(score >= 0.85, "self-match score {score}");
    }
}
