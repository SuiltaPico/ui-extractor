use std::path::Path;

use image::{DynamicImage, GrayImage};

use crate::{
    error::{ExtractError, Result},
    types::Bounds,
};

use super::build::build_embedding_index;
use super::IconEmbedder;
use super::preprocess::EMBED_DIM;
use super::library::IconLibrary;
use super::preprocess::{icon_crop_to_rgb256, template_png_to_rgb256};
use super::IconConfig;

/// Match thresholds used when querying an [`IconPack`].
#[derive(Debug, Clone)]
pub struct IconMatchOptions {
    pub min_cosine: f64,
}

impl Default for IconMatchOptions {
    fn default() -> Self {
        Self {
            min_cosine: IconConfig::default().min_cosine,
        }
    }
}

impl From<&IconConfig> for IconMatchOptions {
    fn from(config: &IconConfig) -> Self {
        Self {
            min_cosine: config.min_cosine,
        }
    }
}

/// One icon match hit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IconMatchHit {
    pub name: String,
    pub score: f64,
}

/// Loaded embeddings + vision encoder for embed/match operations.
pub struct IconPack {
    pub template_size: u32,
    pub library: IconLibrary,
    pub embedder: IconEmbedder,
    pub match_options: IconMatchOptions,
}

impl IconPack {
    /// Load a precomputed embedding index and vision model from disk.
    pub fn load(
        embedding_index: impl AsRef<Path>,
        vision_model: impl AsRef<Path>,
        template_size: u32,
        match_options: IconMatchOptions,
    ) -> Result<Self> {
        let library = IconLibrary::load(embedding_index.as_ref())?;
        let embedder = IconEmbedder::load(vision_model.as_ref())?;
        Ok(Self {
            template_size,
            library,
            embedder,
            match_options,
        })
    }

    /// Build embeddings from PNG templates in memory (offline index build).
    pub fn build_from_dir(
        png_dir: impl AsRef<Path>,
        vision_model: impl AsRef<Path>,
        template_size: u32,
        match_options: IconMatchOptions,
    ) -> Result<Self> {
        let embeddings =
            build_embedding_index(png_dir.as_ref(), vision_model.as_ref(), template_size)?;
        let embedder = IconEmbedder::load(vision_model.as_ref())?;
        Ok(Self {
            template_size,
            library: IconLibrary::from_index(embeddings),
            embedder,
            match_options,
        })
    }

    /// Persist the in-memory embedding index (`embed-mdi` equivalent output).
    pub fn save_embeddings(&self, path: impl AsRef<Path>) -> Result<()> {
        self.library.embeddings.save(path.as_ref())
    }

    /// Embed a template PNG using the library path (color composited on white).
    pub fn embed_template_png(&mut self, png_path: impl AsRef<Path>) -> Result<Vec<f32>> {
        let png_path = png_path.as_ref();
        let img = image::open(png_path).map_err(|e| ExtractError::Image(e.to_string()))?;
        let rgb = template_png_to_rgb256(&img, self.template_size);
        self.embedder.embed_rgb256(&rgb)
    }

    /// Embed a screenshot/icon crop (grayscale adaptive mask → RGB).
    pub fn embed_query_gray(&mut self, gray_crop: &GrayImage) -> Result<Vec<f32>> {
        let rgb = icon_crop_to_rgb256(gray_crop, self.template_size);
        self.embedder.embed_rgb256(&rgb)
    }

    /// Embed a full image decoded from bytes, treating the whole frame as the icon.
    pub fn embed_query_image(&mut self, img: &DynamicImage) -> Result<Vec<f32>> {
        let gray = crate::layout::to_gray(img);
        self.embed_query_gray(&gray)
    }

    /// Match a precomputed embedding using cosine similarity.
    pub fn match_embedding(&self, embedding: &[f32]) -> Option<IconMatchHit> {
        let (name, score) = self
            .library
            .best_match(embedding, self.match_options.min_cosine)?;
        Some(IconMatchHit { name, score })
    }

    /// Return top-k cosine hits.
    pub fn search_embedding(&self, embedding: &[f32], top_k: usize) -> Vec<IconMatchHit> {
        self.library
            .embeddings
            .top_k(embedding, top_k.max(1))
            .into_iter()
            .map(|(idx, cosine)| IconMatchHit {
                name: self.library.embeddings.names[idx].clone(),
                score: cosine,
            })
            .collect()
    }

    /// Match an arbitrary image region in pixel coordinates.
    pub fn match_region(
        &mut self,
        source: &DynamicImage,
        bounds: &Bounds,
    ) -> Option<IconMatchHit> {
        let gray = crate::layout::to_gray(source);
        let crop = crop_gray(&gray, bounds)?;
        let rgb = icon_crop_to_rgb256(&crop, self.template_size);
        let embedding = self.embedder.embed_rgb256(&rgb).ok()?;
        self.match_embedding(&embedding)
    }

    /// Match treating the entire decoded image as a single icon crop.
    pub fn match_image(&mut self, img: &DynamicImage) -> Result<Option<IconMatchHit>> {
        let embedding = self.embed_query_image(img)?;
        Ok(self.match_embedding(&embedding))
    }

    pub fn embedding_dim() -> usize {
        EMBED_DIM
    }
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
