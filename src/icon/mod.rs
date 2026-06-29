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
pub struct IconTimings {
    pub load_ms: f64,
    pub match_ms: f64,
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

fn match_candidate_pack(
    gray: &GrayImage,
    bounds: &Bounds,
    pack: &mut IconPack,
) -> Option<IconMatchHit> {
    let crop = crop_gray(gray, bounds)?;
    pack.match_gray_crop(&crop)
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
