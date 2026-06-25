pub mod embed;
pub mod error;
pub mod icon_index;
pub mod manifest;
pub mod ocr;
pub mod registry;
pub mod runtime;

mod ffi;

pub use embed::{
    cosine, finalize_embedding, l2_normalize, rgb256_to_nchw, EmbedEngine, EMBED_DIM, INPUT_SIZE,
};
pub use error::{InferError, Result};
pub use icon_index::{EmbeddingIndex, IconIndex, IconMatch, IndexStorageFormat};
pub use manifest::{LicenseInfo, Manifest};
pub use ocr::{OcrBounds, OcrEngine, OcrTimings, OcrWord};
pub use registry::Registry;
pub use runtime::{MnnConfig, OnnxConfig, RuntimeConfig};
