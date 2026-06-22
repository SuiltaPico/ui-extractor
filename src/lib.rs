pub mod annotate;
pub mod cases;
pub mod skeleton;
pub mod error;
pub mod icon;
pub mod layout;
pub mod ocr;
pub mod pipeline;
pub mod types;

pub use annotate::{render_annotation, render_layout_annotation};
pub use skeleton::{render_skeleton_html, write_skeleton_html};
pub use cases::{format_ms, process_case, run_cases, CaseBatchSummary, CaseOutputs, CaseTimings};
pub use icon::{attach_icons, try_load_library, IconConfig, IconEmbedder, IconMatchStats, IconTimings};
pub use layout::{LayoutConfig, LayoutStages};
pub use ocr::{OcrConfig, OcrTimings};
pub use pipeline::{extract_from_image, extract_from_image_timed, extract_from_path, ExtractConfig, ExtractTimings};
pub use error::{ExtractError, Result};
pub use types::{Bounds, ExtractResult, UiElement, UiElementKind};
