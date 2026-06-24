pub mod annotate;
pub mod cases;
pub mod engine;
pub mod ffi;
pub mod skeleton;
pub mod error;
#[cfg(feature = "backend-ncnn")]
pub mod inference;
pub mod icon;
pub mod layout;
pub mod ocr;
pub mod pipeline;
pub mod types;

pub use engine::ExtractEngine;
pub use annotate::{render_annotation, render_layout_annotation};
pub use skeleton::{render_skeleton_html, write_skeleton_html};
pub use cases::{format_ms, process_case, run_cases, CaseBatchSummary, CaseOutputs, CaseTimings};
pub use icon::{
    attach_icons, attach_icons_with_pack, build_embedding_index, build_embeddings_file,
    try_load_library, BuildEmbeddingsOptions, IconConfig, IconEmbedder, IconMatchHit,
    IconMatchOptions, IconMatchStats, IconPack, IconRasterColor, IconTimings,
    RasterizeSvgOptions, rasterize_svg_icons,
};
pub use layout::{LayoutConfig, LayoutStages};
pub use ocr::{OcrConfig, OcrTimings};
pub use pipeline::{extract_from_image, extract_from_image_timed, extract_from_path, ExtractConfig, ExtractTimings};
pub use error::{ExtractError, Result};
pub use types::{Bounds, ExtractResult, UiElement, UiElementKind};
