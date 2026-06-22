mod preprocess;
mod stages;
mod tree;

pub use preprocess::to_gray;
pub use stages::LayoutStages;
pub use tree::{extract_layout, extract_layout_with_stages, LayoutConfig};
