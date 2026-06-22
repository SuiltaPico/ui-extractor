use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("image error: {0}")]
    Image(String),

    #[error("OCR failed: {0}")]
    Ocr(String),

    #[error("image has no contours after filtering")]
    EmptyLayout,

    #[error("failed to read image: {0}")]
    ImageRead(String),
}

pub type Result<T> = std::result::Result<T, ExtractError>;

impl From<image::ImageError> for ExtractError {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value.to_string())
    }
}
