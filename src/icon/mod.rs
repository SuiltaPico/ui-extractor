mod build;
mod embed;
mod embedding;
#[cfg(feature = "backend-ncnn")]
mod embedder_ncnn;
#[cfg(feature = "backend-ort")]
mod embedder_ort;
mod library;
mod pack;
mod preprocess;
mod rasterize;

use std::path::PathBuf;
use std::time::Instant;

use image::{DynamicImage, GrayImage};

use crate::{
    error::Result,
    types::{Bounds, UiElement, UiElementKind},
};

pub use build::{
    build_embedding_index, build_embedding_index_from_jobs, collect_png_embed_jobs,
    embedding_worker_jobs, PngEmbedJob,
};
pub use embed::{build_embeddings_file, BuildEmbeddingsOptions};
pub use embedding::EmbeddingIndex;
#[cfg(feature = "backend-ncnn")]
pub use embedder_ncnn::IconEmbedder;
#[cfg(feature = "backend-ort")]
pub use embedder_ort::IconEmbedder;
pub use rasterize::{rasterize_svg_icons, IconRasterColor, RasterizeSvgOptions};
pub use library::IconLibrary;
pub use pack::{IconMatchHit, IconMatchOptions, IconPack};
pub use preprocess::{icon_crop_to_rgb256, template_png_to_rgb256, EMBED_DIM, INPUT_SIZE};

fn default_vision_model() -> PathBuf {
    #[cfg(feature = "backend-ort")]
    {
        PathBuf::from("models/mobileclip2-s0-vision.onnx")
    }
    #[cfg(feature = "backend-ncnn")]
    {
        PathBuf::from("models/mobileclip2-s0-vision.ncnn.param")
    }
}

#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Precomputed embedding index (e.g. `assets/embeddings.bin`).
    pub embedding_index: PathBuf,
    /// MobileCLIP2-S0 vision model path.
    pub vision_model: PathBuf,
    /// Template edge length in pixels for query crop preprocessing.
    pub template_size: u32,
    /// Minimum cosine similarity to accept a match (0–1).
    pub min_cosine: f64,
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
            embedding_index: PathBuf::from("assets/embeddings.bin"),
            vision_model: default_vision_model(),
            template_size: 48,
            min_cosine: 0.72,
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

/// Like [`attach_icons`], but reuses a loaded [`IconPack`] (library + embedder).
pub fn attach_icons_with_pack(
    root: &mut UiElement,
    source: &DynamicImage,
    pack: &mut IconPack,
    config: &IconConfig,
) -> IconMatchStats {
    let match_start = Instant::now();
    let gray = crate::layout::to_gray(source);
    let mut stats = IconMatchStats {
        candidates: 0,
        matched: 0,
        timings: IconTimings::default(),
    };

    walk_mut_pack(root, &gray, pack, config, &mut stats);
    stats.timings.match_ms = match_start.elapsed().as_secs_f64() * 1000.0;
    stats
}

pub fn try_load_library(config: &IconConfig) -> Result<IconLibrary> {
    IconLibrary::load(&config.embedding_index)
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

fn walk_mut_pack(
    node: &mut UiElement,
    gray: &GrayImage,
    pack: &mut IconPack,
    config: &IconConfig,
    stats: &mut IconMatchStats,
) {
    for child in &mut node.children {
        walk_mut_pack(child, gray, pack, config, stats);
    }

    let mut i = 0;
    while i < node.children.len() {
        let child = &node.children[i];
        if is_icon_candidate(child, config) {
            stats.candidates += 1;
            if let Some(hit) = match_candidate_pack(gray, &child.bounds, pack) {
                let bounds = child.bounds;
                node.children[i] = UiElement::icon(bounds, hit.name, Some(hit.score as f32));
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
    let rgb = icon_crop_to_rgb256(&crop, config.template_size);
    let embedding = embedder.embed_rgb256(&rgb).ok()?;
    library.best_match(&embedding, config.min_cosine)
}

fn match_candidate_pack(
    gray: &GrayImage,
    bounds: &Bounds,
    pack: &mut IconPack,
) -> Option<IconMatchHit> {
    let crop = crop_gray(gray, bounds)?;
    let rgb = icon_crop_to_rgb256(&crop, pack.template_size);
    let embedding = pack.embedder.embed_rgb256(&rgb).ok()?;
    pack.match_embedding(&embedding)
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
    fn template_png_rgb_differs_between_icons() {
        let dir = Path::new("assets/icons/mdi");
        if !dir.is_dir() {
            return;
        }
        let home = dir.join("home.png");
        let menu = dir.join("menu.png");
        if !home.is_file() || !menu.is_file() {
            return;
        }
        let home_rgb = template_png_to_rgb256(&image::open(home).unwrap(), 48);
        let menu_rgb = template_png_to_rgb256(&image::open(menu).unwrap(), 48);
        assert_ne!(home_rgb.as_raw(), menu_rgb.as_raw());
    }

    #[test]
    fn embedding_roundtrip_when_assets_present() {
        let index_path = Path::new("assets/embeddings.bin");
        let model = default_vision_model();
        if !index_path.is_file() || !model.is_file() {
            return;
        }

        let config = IconConfig::default();
        let library = match try_load_library(&config) {
            Ok(lib) => lib,
            Err(_) => return,
        };

        let dir = Path::new("assets/icons/mdi");
        let name = "mdi:home";
        let png_path = dir.join("home.png");
        if !png_path.is_file() {
            return;
        }

        let mut embedder = IconEmbedder::load(&model).unwrap();
        let img = image::open(&png_path).unwrap();
        let rgb = template_png_to_rgb256(&img, config.template_size);
        let embedding = embedder.embed_rgb256(&rgb).unwrap();
        let (matched, score) = library.best_match(&embedding, 0.5).unwrap();
        assert_eq!(matched, name);
        assert!(score >= 0.85, "self-match score {score}");
    }
}
