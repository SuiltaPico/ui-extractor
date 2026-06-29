use std::path::{Path, PathBuf};
use std::time::Instant;

use image::DynamicImage;
use crate::infer::{OcrEngine, Registry, RuntimeConfig};

use crate::{
    error::{ExtractError, Result},
    icon::{attach_icons_with_pack, IconConfig, IconMatchStats, IconPack},
    layout::{extract_layout_with_stages, LayoutConfig, LayoutStages},
    ocr::{extract_words_from_image_timed, load_ocr_engine, OcrConfig, OcrTimings, OcrWord},
    packs::{resolve_models_dir, DEFAULT_ICON_INDEX_PACK, DEFAULT_OCR_PACK},
    types::{Bounds, ExtractResult, UiElement},
};

use crate::layout::to_gray;

#[derive(Debug, Clone, Default)]
pub struct ExtractTimings {
    pub gray_ms: f64,
    pub layout_ms: f64,
    pub pipeline_dump_ms: f64,
    /// Wall time while layout and OCR run in parallel (0 when OCR is off).
    pub parallel_ms: f64,
    pub ocr: OcrTimings,
    pub attach_words_ms: f64,
    pub icon: IconMatchStats,
}

impl ExtractTimings {
    pub fn ocr_total_ms(&self) -> f64 {
        self.ocr.init_ms + self.ocr.predict_ms
    }

    pub fn extract_ms(&self) -> f64 {
        self.gray_ms
            + self.layout_ms
            + self.pipeline_dump_ms
            + self.ocr_total_ms()
            + self.attach_words_ms
            + self.icon.timings.match_ms
    }
}

#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub models_dir: PathBuf,
    pub runtime: RuntimeConfig,
    pub ocr_pack: String,
    pub icon_index_pack: String,
    pub layout: LayoutConfig,
    pub ocr: OcrConfig,
    pub icon: IconConfig,
    pub run_ocr: bool,
    pub run_icon: bool,
    /// When set, write intermediate layout pipeline images under this directory.
    pub pipeline_dump_dir: Option<PathBuf>,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            models_dir: resolve_models_dir(None),
            runtime: RuntimeConfig::default(),
            ocr_pack: DEFAULT_OCR_PACK.to_string(),
            icon_index_pack: DEFAULT_ICON_INDEX_PACK.to_string(),
            layout: LayoutConfig::default(),
            ocr: OcrConfig::default(),
            icon: IconConfig::default(),
            run_ocr: true,
            run_icon: true,
            pipeline_dump_dir: None,
        }
    }
}

pub fn extract_from_path(path: &Path, config: &ExtractConfig) -> Result<ExtractResult> {
    let mut engine = crate::ExtractEngine::open(config.clone())?;
    engine.extract_from_path(path).map(|(result, _)| result)
}

pub fn extract_from_image(img: &DynamicImage, config: &ExtractConfig) -> Result<ExtractResult> {
    extract_from_image_timed(img, config).map(|(result, _)| result)
}

pub fn extract_from_image_timed(
    img: &DynamicImage,
    config: &ExtractConfig,
) -> Result<(ExtractResult, ExtractTimings)> {
    let mut engine = crate::ExtractEngine::open(config.clone())?;
    engine.extract_from_image(img)
}

pub(crate) fn extract_from_image_timed_with_engine(
    img: &DynamicImage,
    config: &ExtractConfig,
    registry: &Registry,
    ocr: Option<&OcrEngine>,
    icon_pack: Option<&mut IconPack>,
) -> Result<(ExtractResult, ExtractTimings)> {
    if config.run_ocr {
        extract_with_parallel_ocr(img, config, registry, ocr)
    } else {
        extract_layout_only(img, config, icon_pack)
    }
}

fn extract_layout_only(
    img: &DynamicImage,
    config: &ExtractConfig,
    icon_pack: Option<&mut IconPack>,
) -> Result<(ExtractResult, ExtractTimings)> {
    let mut timings = ExtractTimings::default();

    let gray_start = Instant::now();
    let gray = to_gray(img);
    timings.gray_ms = ms_since(gray_start);
    let width = gray.width() as i32;
    let height = gray.height() as i32;

    let layout_start = Instant::now();
    let (mut root, stages) = extract_layout_with_stages(&gray, &config.layout)?;
    timings.layout_ms = ms_since(layout_start);

    if let Some(dir) = &config.pipeline_dump_dir {
        let dump_start = Instant::now();
        write_pipeline_dump(dir, img, &stages)?;
        timings.pipeline_dump_ms = ms_since(dump_start);
    }
    drop(stages);

    if config.run_icon {
        run_icon_pass(img, &mut root, icon_pack, &mut timings);
    }

    Ok((ExtractResult { width, height, root }, timings))
}

fn extract_with_parallel_ocr(
    img: &DynamicImage,
    config: &ExtractConfig,
    _registry: &Registry,
    ocr: Option<&OcrEngine>,
) -> Result<(ExtractResult, ExtractTimings)> {
    let parallel_start = Instant::now();
    let layout_config = config.layout.clone();
    let dump_dir = config.pipeline_dump_dir.clone();
    let ocr_config = config.ocr.clone();
    let ocr_pack = config.ocr_pack.clone();
    let models_dir = config.models_dir.clone();
    let runtime = config.runtime.clone();

    let (mut root, width, height, branch_timings, ocr_result) =
        std::thread::scope(|scope| -> Result<_> {
            let ocr_handle = scope.spawn(|| {
                if let Some(engine) = ocr {
                    extract_words_from_image_timed(img, engine)
                } else {
                    let registry = Registry::open(&models_dir, runtime.clone())
                        .map_err(|e| ExtractError::Ocr(e.to_string()))?;
                    let engine = load_ocr_engine(&registry, &ocr_pack, &ocr_config)?;
                    extract_words_from_image_timed(img, &engine)
                }
            });

            let mut branch_timings = ExtractTimings::default();

            let gray_start = Instant::now();
            let gray = to_gray(img);
            branch_timings.gray_ms = ms_since(gray_start);
            let width = gray.width() as i32;
            let height = gray.height() as i32;

            let layout_start = Instant::now();
            let (root, stages) = extract_layout_with_stages(&gray, &layout_config)?;
            branch_timings.layout_ms = ms_since(layout_start);
            drop(gray);

            if let Some(dir) = &dump_dir {
                let dump_start = Instant::now();
                write_pipeline_dump(dir, img, &stages)?;
                branch_timings.pipeline_dump_ms = ms_since(dump_start);
            }
            drop(stages);

            let ocr_result = ocr_handle
                .join()
                .map_err(|_| ExtractError::Ocr("OCR thread panicked".into()))?;

            Ok((root, width, height, branch_timings, ocr_result))
        })?;

    let mut timings = branch_timings;
    timings.parallel_ms = ms_since(parallel_start);

    match ocr_result {
        Ok((words, ocr_timings)) => {
            timings.ocr = ocr_timings;
            let attach_start = Instant::now();
            attach_words(&mut root, &words);
            timings.attach_words_ms = ms_since(attach_start);
        }
        Err(e) => {
            eprintln!("warning: OCR skipped ({e})");
        }
    }

    Ok((ExtractResult { width, height, root }, timings))
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn run_icon_pass(
    img: &DynamicImage,
    root: &mut UiElement,
    icon_pack: Option<&mut IconPack>,
    timings: &mut ExtractTimings,
) {
    let Some(pack) = icon_pack else {
        eprintln!("warning: icon recognition skipped (pack not loaded)");
        return;
    };

    let load_start = Instant::now();
    timings.icon.timings.load_ms = ms_since(load_start);

    let stats = attach_icons_with_pack(root, img, pack, &pack.match_config());
    timings.icon = stats;
}

fn write_pipeline_dump(dir: &Path, source: &DynamicImage, stages: &LayoutStages) -> Result<()> {
    stages.write_to_dir(dir)?;
    stages.write_layout_before_prune(dir, source)?;
    Ok(())
}

fn attach_words(root: &mut UiElement, words: &[OcrWord]) {
    if words.is_empty() {
        return;
    }

    let node_count = count_nodes(root);
    let mut assignments: Vec<Vec<&OcrWord>> = vec![vec![]; node_count];

    for word in words {
        if let Some(idx) = find_best_node_index(root, word) {
            assignments[idx].push(word);
        }
    }

    let _ = apply_assignments(root, &assignments, 0);
}

fn count_nodes(node: &UiElement) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn find_best_node_index(root: &UiElement, word: &OcrWord) -> Option<usize> {
    let center = word_center(&word.bounds);
    let mut best: Option<(usize, i64)> = None;

    fn walk(node: &UiElement, idx: &mut usize, center: (i32, i32), best: &mut Option<(usize, i64)>) {
        let current = *idx;
        *idx += 1;

        if point_in_bounds(center, &node.bounds) {
            let area = node.bounds.area();
            match best {
                None => *best = Some((current, area)),
                Some((_, best_area)) if area <= *best_area => *best = Some((current, area)),
                _ => {}
            }
        }

        for child in &node.children {
            walk(child, idx, center, best);
        }
    }

    let mut idx = 0;
    walk(root, &mut idx, center, &mut best);
    best.map(|(i, _)| i)
}

fn apply_assignments(node: &mut UiElement, assignments: &[Vec<&OcrWord>], slot: usize) -> usize {
    let mut next_slot = slot + 1;
    let mut new_children = Vec::new();

    for child in node.children.drain(..) {
        let (updated_slot, mut rebuilt_children) =
            apply_assignments_boxed(child, assignments, next_slot);
        next_slot = updated_slot;
        new_children.append(&mut rebuilt_children);
    }

    let mut text_elements = merge_words_to_text_elements(&assignments[slot]);
    text_elements.sort_by_key(|el| (el.bounds.y, el.bounds.x));
    new_children.append(&mut text_elements);

    node.children = new_children;
    next_slot
}

fn apply_assignments_boxed(
    mut node: UiElement,
    assignments: &[Vec<&OcrWord>],
    slot: usize,
) -> (usize, Vec<UiElement>) {
    let next_slot = apply_assignments(&mut node, assignments, slot);
    (next_slot, vec![node])
}

fn merge_words_to_text_elements(words: &[&OcrWord]) -> Vec<UiElement> {
    if words.is_empty() {
        return vec![];
    }

    let mut sorted: Vec<&OcrWord> = words.to_vec();
    sorted.sort_by(|a, b| {
        a.bounds
            .y
            .cmp(&b.bounds.y)
            .then(a.bounds.x.cmp(&b.bounds.x))
    });

    let line_threshold = median_height(&sorted) / 2;
    let mut lines: Vec<Vec<&OcrWord>> = vec![];
    for word in sorted {
        if let Some(last_line) = lines.last_mut() {
            let last_y = last_line[0].bounds.y;
            if (word.bounds.y - last_y).abs() <= line_threshold.max(4) {
                last_line.push(word);
                continue;
            }
        }
        lines.push(vec![word]);
    }

    lines
        .into_iter()
        .map(|line| {
            let mut line = line;
            line.sort_by_key(|w| w.bounds.x);
            let text: String = line
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let bounds = union_bounds(&line.iter().map(|w| w.bounds).collect::<Vec<_>>());
            let confidence = average_confidence(&line);
            UiElement::text(bounds, text, Some(confidence))
        })
        .collect()
}

fn median_height(words: &[&OcrWord]) -> i32 {
    if words.is_empty() {
        return 12;
    }
    let mut heights: Vec<i32> = words.iter().map(|w| w.bounds.height).collect();
    heights.sort_unstable();
    heights[heights.len() / 2]
}

fn average_confidence(words: &[&OcrWord]) -> f32 {
    if words.is_empty() {
        return 0.0;
    }
    words.iter().map(|w| w.confidence).sum::<f32>() / words.len() as f32
}

fn union_bounds(bounds: &[Bounds]) -> Bounds {
    let x = bounds.iter().map(|b| b.x).min().unwrap_or(0);
    let y = bounds.iter().map(|b| b.y).min().unwrap_or(0);
    let right = bounds.iter().map(|b| b.right()).max().unwrap_or(x);
    let bottom = bounds.iter().map(|b| b.bottom()).max().unwrap_or(y);
    Bounds::new(x, y, (right - x).max(1), (bottom - y).max(1))
}

fn word_center(bounds: &Bounds) -> (i32, i32) {
    (bounds.x + bounds.width / 2, bounds.y + bounds.height / 2)
}

fn point_in_bounds((px, py): (i32, i32), bounds: &Bounds) -> bool {
    px >= bounds.x && py >= bounds.y && px < bounds.right() && py < bounds.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UiElementKind;

    #[test]
    fn assigns_word_to_smallest_container() {
        let mut root = UiElement {
            bounds: Bounds::new(0, 0, 100, 100),
            kind: UiElementKind::Root,
            children: vec![UiElement::container(
                Bounds::new(10, 10, 40, 40),
                vec![],
            )],
        };
        let words = vec![OcrWord {
            text: "OK".into(),
            bounds: Bounds::new(15, 15, 20, 10),
            confidence: 90.0,
        }];
        attach_words(&mut root, &words);
        assert!(root.children[0].children.iter().any(|c| matches!(
            c.kind,
            UiElementKind::Text { ref content, .. } if content == "OK"
        )));
    }
}
