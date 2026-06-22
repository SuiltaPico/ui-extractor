use image::GrayImage;
use imageproc::contours::{find_contours, BorderType, Contour};

use crate::{
    error::Result,
    types::{Bounds, UiElement, UiElementKind},
};

use super::preprocess::{blur_for_edges, canny_edges, close_gaps, dilate_edges};
use super::stages::{ContourFilterStats, LayoutStages};

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Minimum outer-contour AABB area in pixels.
    pub min_area: i64,
    /// Drop hole contours whose AABB area is below this (e.g. letter counters).
    pub min_hole_area: i64,
    /// Drop hole contours whose shorter side is below this (px).
    pub min_hole_side: i32,
    /// Ignore contours whose bounds overlap parent above this IoU when linking hierarchy.
    pub max_parent_iou: f64,
    /// Morphology kernel size for closing gaps (0 = skip).
    pub close_kernel: u32,
    /// Drop a child container when it sits inside the parent with at most this inset (px).
    pub max_inset_margin: i32,
    /// Uniform padding inset up to this margin also triggers pruning.
    pub max_padding_margin: i32,
    /// Drop a child container whose area / parent area exceeds this ratio (near-duplicate).
    pub max_child_area_ratio: f64,
    /// Drop a uniformly padded inner container below this area ratio.
    pub max_padded_child_area_ratio: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            min_area: 100,
            min_hole_area: 2_000,
            min_hole_side: 40,
            max_parent_iou: 0.92,
            close_kernel: 3,
            max_inset_margin: 10,
            max_padding_margin: 20,
            max_child_area_ratio: 0.82,
            max_padded_child_area_ratio: 0.65,
        }
    }
}

struct ContourNode {
    bounds: Bounds,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// Extract a hierarchical UI layout tree from a screenshot.
pub fn extract_layout(gray: &GrayImage, config: &LayoutConfig) -> Result<UiElement> {
    extract_layout_with_stages(gray, config).map(|(root, _)| root)
}

/// Like [`extract_layout`], but also returns intermediate pipeline images for debugging.
pub fn extract_layout_with_stages(
    gray: &GrayImage,
    config: &LayoutConfig,
) -> Result<(UiElement, LayoutStages)> {
    let (width, height) = gray.dimensions();
    let screen = Bounds::new(0, 0, width as i32, height as i32);

    let blurred = blur_for_edges(gray);
    let edges = canny_edges(&blurred);
    let dilated = dilate_edges(&edges, 1);
    let mut closed = dilated.clone();
    if config.close_kernel > 0 {
        closed = close_gaps(&dilated, config.close_kernel);
    }

    let contours = find_contours::<u32>(&closed);
    let contours_all = LayoutStages::render_contours(&contours, width, height, None);

    if contours.is_empty() {
        let stages = empty_stages(
            gray,
            blurred,
            edges,
            dilated,
            closed,
            contours_all,
            width,
            height,
            screen,
        );
        return Ok((root_element(screen, vec![]), stages));
    }

    let mut nodes: Vec<ContourNode> = Vec::new();
    let mut orig_to_filtered: Vec<Option<usize>> = vec![None; contours.len()];
    let mut kept_mask = vec![false; contours.len()];
    let mut filter_stats = ContourFilterStats {
        total: contours.len(),
        kept: 0,
        dropped_hole: 0,
        dropped_area: 0,
        dropped_thin: 0,
    };

    for (orig_idx, contour) in contours.iter().enumerate() {
        let bounds = contour_bounds(contour);
        match classify_contour(contour, &bounds, config) {
            ContourKeep::Yes => {
                kept_mask[orig_idx] = true;
                filter_stats.kept += 1;
                let filtered_idx = nodes.len();
                orig_to_filtered[orig_idx] = Some(filtered_idx);
                nodes.push(ContourNode {
                    bounds,
                    parent: None,
                    children: vec![],
                });
            }
            ContourKeep::HoleTooSmall => filter_stats.dropped_hole += 1,
            ContourKeep::AreaTooSmall => filter_stats.dropped_area += 1,
            ContourKeep::TooThin => filter_stats.dropped_thin += 1,
        }
    }

    let contours_kept = LayoutStages::render_contours(&contours, width, height, Some(&kept_mask));
    let dropped_mask: Vec<bool> = kept_mask.iter().map(|k| !k).collect();
    let contours_dropped =
        LayoutStages::render_contours(&contours, width, height, Some(&dropped_mask));

    if nodes.is_empty() {
        let stages = LayoutStages {
            width,
            height,
            gray: gray.clone(),
            blurred,
            edges,
            dilated,
            closed,
            contours_all,
            contours_kept,
            contours_dropped,
            filter_stats,
            layout_before_prune: root_element(screen, vec![]),
        };
        return Ok((root_element(screen, vec![]), stages));
    }

    for (orig_idx, contour) in contours.iter().enumerate() {
        let Some(child_filtered) = orig_to_filtered[orig_idx] else {
            continue;
        };
        let Some(parent_filtered) = find_kept_ancestor(&contours, contour.parent, &orig_to_filtered)
        else {
            continue;
        };

        if nodes[child_filtered]
            .bounds
            .iou(&nodes[parent_filtered].bounds)
            <= config.max_parent_iou
        {
            nodes[child_filtered].parent = Some(parent_filtered);
            nodes[parent_filtered].children.push(child_filtered);
        }
    }

    let roots: Vec<usize> = (0..nodes.len())
        .filter(|&i| nodes[i].parent.is_none())
        .collect();

    let children = if roots.len() == 1 {
        vec![build_subtree(&nodes, roots[0])]
    } else {
        roots
            .into_iter()
            .map(|r| build_subtree(&nodes, r))
            .collect()
    };

    let layout_before_prune = root_element(screen, children);
    let mut root = layout_before_prune.clone();
    prune_redundant(&mut root, config);

    let stages = LayoutStages {
        width,
        height,
        gray: gray.clone(),
        blurred,
        edges,
        dilated,
        closed,
        contours_all,
        contours_kept,
        contours_dropped,
        filter_stats,
        layout_before_prune,
    };

    Ok((root, stages))
}

fn empty_stages(
    gray: &GrayImage,
    blurred: GrayImage,
    edges: GrayImage,
    dilated: GrayImage,
    closed: GrayImage,
    contours_all: GrayImage,
    width: u32,
    height: u32,
    screen: Bounds,
) -> LayoutStages {
    LayoutStages {
        width,
        height,
        gray: gray.clone(),
        blurred,
        edges,
        dilated,
        closed,
        contours_all,
        contours_kept: GrayImage::from_pixel(width, height, image::Luma([0])),
        contours_dropped: GrayImage::from_pixel(width, height, image::Luma([0])),
        filter_stats: ContourFilterStats {
            total: 0,
            kept: 0,
            dropped_hole: 0,
            dropped_area: 0,
            dropped_thin: 0,
        },
        layout_before_prune: root_element(screen, vec![]),
    }
}

/// Walk the contour parent chain, skipping holes and contours removed by filtering.
fn find_kept_ancestor(
    contours: &[Contour<u32>],
    start: Option<usize>,
    orig_to_filtered: &[Option<usize>],
) -> Option<usize> {
    let mut cur = start;
    while let Some(orig_idx) = cur {
        if let Some(filtered) = orig_to_filtered[orig_idx] {
            return Some(filtered);
        }
        cur = contours.get(orig_idx).and_then(|c| c.parent);
    }
    None
}

enum ContourKeep {
    Yes,
    HoleTooSmall,
    AreaTooSmall,
    TooThin,
}

fn classify_contour(contour: &Contour<u32>, bounds: &Bounds, config: &LayoutConfig) -> ContourKeep {
    if bounds.width < 2 || bounds.height < 2 {
        return ContourKeep::TooThin;
    }

    if contour.border_type == BorderType::Hole {
        if bounds.area() >= config.min_hole_area
            && bounds.width >= config.min_hole_side
            && bounds.height >= config.min_hole_side
        {
            return ContourKeep::Yes;
        }
        return ContourKeep::HoleTooSmall;
    }

    if bounds.area() >= config.min_area {
        ContourKeep::Yes
    } else {
        ContourKeep::AreaTooSmall
    }
}

fn contour_bounds(contour: &Contour<u32>) -> Bounds {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for point in &contour.points {
        let x = point.x as i32;
        let y = point.y as i32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    Bounds::new(
        min_x,
        min_y,
        (max_x - min_x + 1).max(1),
        (max_y - min_y + 1).max(1),
    )
}

fn build_subtree(nodes: &[ContourNode], idx: usize) -> UiElement {
    let node = &nodes[idx];
    let children = node
        .children
        .iter()
        .map(|&c| build_subtree(nodes, c))
        .collect();
    UiElement::container(node.bounds, children)
}

fn root_element(bounds: Bounds, children: Vec<UiElement>) -> UiElement {
    UiElement {
        bounds,
        kind: UiElementKind::Root,
        children,
    }
}

/// Remove redundant nested containers caused by border re-detection.
fn prune_redundant(element: &mut UiElement, config: &LayoutConfig) {
    for child in &mut element.children {
        prune_redundant(child, config);
    }

    let parent_bounds = element.bounds;
    element.children.retain(|child| {
        !is_redundant_nested(&parent_bounds, child, config)
    });
}

fn is_redundant_nested(parent: &Bounds, child: &UiElement, config: &LayoutConfig) -> bool {
    if matches!(
        child.kind,
        UiElementKind::Text { .. } | UiElementKind::Icon { .. } | UiElementKind::Root
    ) {
        return false;
    }

    if !parent.contains(&child.bounds) {
        return false;
    }

    let area_ratio = child.bounds.area() as f64 / parent.area().max(1) as f64;

    // Outer/inner frame of the same widget (e.g. button border + inner fill).
    if area_ratio >= config.max_child_area_ratio {
        return true;
    }

    if is_tight_inset(parent, &child.bounds, config.max_inset_margin) {
        return true;
    }

    if area_ratio <= config.max_padded_child_area_ratio
        && is_uniform_inset(parent, &child.bounds, config.max_padding_margin)
    {
        return true;
    }

    false
}

fn is_uniform_inset(outer: &Bounds, inner: &Bounds, max_margin: i32) -> bool {
    let margins = [
        inner.x - outer.x,
        inner.y - outer.y,
        outer.right() - inner.right(),
        outer.bottom() - inner.bottom(),
    ];

    if !margins.iter().all(|&m| m >= 0 && m <= max_margin) {
        return false;
    }

    let min = *margins.iter().min().unwrap();
    let max = *margins.iter().max().unwrap();
    max - min <= 6
}

fn is_tight_inset(outer: &Bounds, inner: &Bounds, max_margin: i32) -> bool {
    let margins = [
        inner.x - outer.x,
        inner.y - outer.y,
        outer.right() - inner.right(),
        outer.bottom() - inner.bottom(),
    ];

    margins.iter().all(|&m| m >= 0 && m <= max_margin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;
    use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut};
    use imageproc::rect::Rect;

    fn blank_with_rect(x: i32, y: i32, w: i32, h: i32) -> GrayImage {
        let mut img = GrayImage::from_pixel(200, 200, Luma([255]));
        let rect = Rect::at(x, y).of_size(w as u32, h as u32);
        draw_hollow_rect_mut(&mut img, rect, Luma([0]));
        img
    }

    #[test]
    fn detects_drawn_rectangle() {
        let img = blank_with_rect(40, 40, 80, 60);
        let tree = extract_layout(&img, &LayoutConfig::default()).unwrap();
        assert!(!tree.children.is_empty());
    }

    #[test]
    fn skips_hole_contours() {
        let mut img = GrayImage::from_pixel(80, 80, Luma([255]));
        draw_filled_rect_mut(&mut img, Rect::at(10, 10).of_size(60, 60), Luma([0]));
        draw_filled_rect_mut(&mut img, Rect::at(25, 25).of_size(30, 30), Luma([255]));

        let tree = extract_layout(&img, &LayoutConfig::default()).unwrap();
        let flat = flatten_bounds(&tree);
        assert!(
            flat.iter().all(|b| b.area() >= 100),
            "hole interior should not appear as its own box"
        );
    }

    #[test]
    fn prunes_tight_inset_duplicate() {
        let mut img = GrayImage::from_pixel(120, 120, Luma([255]));
        draw_hollow_rect_mut(
            &mut img,
            Rect::at(20, 20).of_size(80, 50),
            Luma([0]),
        );
        draw_filled_rect_mut(
            &mut img,
            Rect::at(23, 23).of_size(74, 44),
            Luma([0]),
        );

        let tree = extract_layout(&img, &LayoutConfig::default()).unwrap();
        let containers = count_containers(&tree);
        assert!(
            containers <= 2,
            "inset duplicate should collapse, got {containers} containers"
        );
    }

    fn flatten_bounds(node: &UiElement) -> Vec<Bounds> {
        let mut out = vec![node.bounds];
        for child in &node.children {
            out.extend(flatten_bounds(child));
        }
        out
    }

    fn count_containers(node: &UiElement) -> usize {
        let self_count = matches!(node.kind, UiElementKind::Container) as usize;
        self_count + node.children.iter().map(count_containers).sum::<usize>()
    }

    #[test]
    fn arknights_contour_filter_stats() {
        let path = std::path::Path::new("tests/cases/arknights-main-ui/input.jpg");
        if !path.exists() {
            return;
        }
        let img = image::open(path).unwrap();
        let gray = super::super::preprocess::to_gray(&img);
        let blurred = blur_for_edges(&gray);
        let edges = canny_edges(&blurred);
        let dilated = dilate_edges(&edges, 1);
        let closed = close_gaps(&dilated, 3);
        let contours = find_contours::<u32>(&closed);
        let config = LayoutConfig::default();

        let mut dropped_hole = 0usize;
        let mut dropped_area = 0usize;
        let mut kept = 0usize;

        for contour in &contours {
            let bounds = contour_bounds(contour);
            match classify_contour(contour, &bounds, &config) {
                ContourKeep::Yes => kept += 1,
                ContourKeep::HoleTooSmall => dropped_hole += 1,
                ContourKeep::AreaTooSmall => dropped_area += 1,
                ContourKeep::TooThin => {}
            }
        }

        eprintln!(
            "contours={} kept={} dropped_hole={} dropped_area={}",
            contours.len(),
            kept,
            dropped_hole,
            dropped_area
        );
    }
}
