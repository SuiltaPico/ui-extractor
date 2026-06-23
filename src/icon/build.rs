use std::fs;
use std::path::{Path, PathBuf};

use image::DynamicImage;

use crate::error::{ExtractError, Result};

use super::embedding::EmbeddingIndex;
use super::IconEmbedder;
use super::preprocess::EMBED_DIM;
use super::preprocess::template_png_to_rgb256;

/// Build a precomputed embedding index from a directory of PNG icons.
///
/// Each PNG file name (without extension) becomes the icon label in the index.
pub fn build_embedding_index(
    png_dir: &Path,
    embedder: &mut IconEmbedder,
    template_size: u32,
) -> Result<EmbeddingIndex> {
    if !png_dir.is_dir() {
        return Err(ExtractError::Image(format!(
            "PNG directory not found: {}",
            png_dir.display()
        )));
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(png_dir)
        .map_err(|e| ExtractError::Image(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        return Err(ExtractError::Image(format!(
            "no PNG files under {}",
            png_dir.display()
        )));
    }

    let mut names = Vec::with_capacity(paths.len());
    let mut vectors = Vec::with_capacity(paths.len() * EMBED_DIM);
    let mut errors = Vec::new();

    for path in &paths {
        match embed_one(path, embedder, template_size) {
            Ok((name, embedding)) => {
                names.push(name);
                vectors.extend(embedding);
            }
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    if !errors.is_empty() {
        return Err(ExtractError::Image(format!(
            "embedding failed for {} file(s): {}",
            errors.len(),
            errors.join("; ")
        )));
    }

    Ok(EmbeddingIndex {
        dim: EMBED_DIM as u32,
        names,
        vectors,
    })
}

fn embed_one(
    path: &Path,
    embedder: &mut IconEmbedder,
    template_size: u32,
) -> Result<(String, Vec<f32>)> {
    let img = image::open(path).map_err(|e| ExtractError::Image(e.to_string()))?;
    embed_png_image(&img, path, embedder, template_size)
}

/// Embed a single template PNG (label defaults to file stem).
pub fn embed_png_image(
    img: &DynamicImage,
    label_source: &Path,
    embedder: &mut IconEmbedder,
    template_size: u32,
) -> Result<(String, Vec<f32>)> {
    let rgb = template_png_to_rgb256(img, template_size);
    let embedding = embedder.embed_rgb256(&rgb)?;
    let name = label_source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ExtractError::Image(format!(
                "invalid PNG file name: {}",
                label_source.display()
            ))
        })?
        .to_string();
    Ok((name, embedding))
}
