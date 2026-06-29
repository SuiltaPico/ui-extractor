use std::time::Instant;

use image::{DynamicImage, GrayImage};
use crate::infer::{EmbedEngine, IconIndex, Registry};

use crate::{
    error::{ExtractError, Result},
    types::Bounds,
};
use super::{IconConfig, IconTimings};
use super::preprocess::EMBED_DIM;
use super::preprocess::icon_crop_to_rgb256;

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
    embedder: EmbedEngine,
    index: IconIndex,
    match_options: IconMatchOptions,
    candidate_config: IconConfig,
}

impl IconPack {
    /// Load icon index and its embed dependency from a manifest registry.
    pub fn from_registry(
        registry: &Registry,
        icon_index_pack_id: &str,
        template_size: u32,
        candidate_config: IconConfig,
    ) -> Result<Self> {
        let manifest = registry
            .manifest(icon_index_pack_id)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        let embed_id = manifest.embed_model_id.as_deref().ok_or_else(|| {
            ExtractError::Image(format!(
                "icon_index pack {icon_index_pack_id} missing embed_model_id in manifest"
            ))
        })?;
        let embedder = registry
            .load_embed(embed_id)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        let index = registry
            .load_icon_index(icon_index_pack_id)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        Ok(Self {
            template_size,
            embedder,
            index,
            match_options: IconMatchOptions::from(&candidate_config),
            candidate_config,
        })
    }

    pub fn match_config(&self) -> IconConfig {
        self.candidate_config.clone()
    }

    pub fn embed_query_gray(&mut self, gray_crop: &GrayImage) -> Result<Vec<f32>> {
        let rgb = icon_crop_to_rgb256(gray_crop, self.template_size);
        self.embedder
            .embed_rgb256(&rgb)
            .map_err(|e| ExtractError::Image(e.to_string()))
    }

    pub fn embed_query_image(&mut self, img: &DynamicImage) -> Result<Vec<f32>> {
        let gray = crate::layout::to_gray(img);
        self.embed_query_gray(&gray)
    }

    pub fn match_gray_crop(&mut self, gray_crop: &GrayImage) -> Option<IconMatchHit> {
        let mut discard = IconTimings::default();
        self.match_gray_crop_timed(gray_crop, &mut discard)
    }

    pub fn match_gray_crop_timed(
        &mut self,
        gray_crop: &GrayImage,
        timings: &mut IconTimings,
    ) -> Option<IconMatchHit> {
        let preprocess_start = Instant::now();
        let rgb = icon_crop_to_rgb256(gray_crop, self.template_size);
        timings.preprocess_ms += preprocess_start.elapsed().as_secs_f64() * 1000.0;

        let embed_start = Instant::now();
        let embedding = self.embedder.embed_rgb256(&rgb).ok()?;
        timings.embed_ms += embed_start.elapsed().as_secs_f64() * 1000.0;

        let index_start = Instant::now();
        let hit = self.match_embedding(&embedding).ok().flatten();
        timings.index_ms += index_start.elapsed().as_secs_f64() * 1000.0;
        hit
    }

    pub fn match_embedding(&self, embedding: &[f32]) -> Result<Option<IconMatchHit>> {
        self.index
            .match_embedding(embedding, self.match_options.min_cosine)
            .map(|m| {
                m.map(|hit| IconMatchHit {
                    name: hit.name,
                    score: hit.score,
                })
            })
            .map_err(|e| ExtractError::Image(e.to_string()))
    }

    pub fn search_embedding(&self, embedding: &[f32], top_k: usize) -> Result<Vec<IconMatchHit>> {
        self.index
            .search(embedding, top_k.max(1))
            .map(|hits| {
                hits.into_iter()
                    .map(|m| IconMatchHit {
                        name: m.name,
                        score: m.score,
                    })
                    .collect()
            })
            .map_err(|e| ExtractError::Image(e.to_string()))
    }

    pub fn match_region(
        &mut self,
        source: &DynamicImage,
        bounds: &Bounds,
    ) -> Option<IconMatchHit> {
        let gray = crate::layout::to_gray(source);
        let crop = crop_gray(&gray, bounds)?;
        let rgb = icon_crop_to_rgb256(&crop, self.template_size);
        let embedding = self.embedder.embed_rgb256(&rgb).ok()?;
        self.match_embedding(&embedding).ok().flatten()
    }

    pub fn match_image(&mut self, img: &DynamicImage) -> Result<Option<IconMatchHit>> {
        let embedding = self.embed_query_image(img)?;
        self.match_embedding(&embedding)
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
