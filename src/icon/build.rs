use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use image::DynamicImage;
use rayon::prelude::*;

use crate::error::{ExtractError, Result};

use super::embedding::EmbeddingIndex;
use super::IconEmbedder;
use super::preprocess::EMBED_DIM;
use super::preprocess::template_png_to_rgb256;

thread_local! {
    static THREAD_EMBEDDER: RefCell<Option<(PathBuf, IconEmbedder)>> = const { RefCell::new(None) };
}

/// Build a precomputed embedding index from a directory of PNG icons.
///
/// Each PNG file name (without extension) becomes the icon label in the index.
/// Embeddings are computed in parallel using one model session per worker thread.
pub fn build_embedding_index(
    png_dir: &Path,
    vision_model: &Path,
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

    let jobs = default_jobs().max(1).min(paths.len());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| ExtractError::Image(e.to_string()))?;

    let errors: Mutex<Vec<String>> = Mutex::new(vec![]);
    let done = AtomicUsize::new(0);
    let total = paths.len();
    let vision_model = vision_model.to_path_buf();

    let mut results: Vec<(usize, String, Vec<f32>)> = pool.install(|| {
        paths
            .par_iter()
            .enumerate()
            .filter_map(|(idx, path)| {
                let result = with_thread_embedder(&vision_model, |embedder| {
                    embed_one(path, embedder, template_size)
                });
                match result {
                    Ok((name, embedding)) => {
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 500 == 0 || n == total {
                            eprintln!("embedded {n}/{total}");
                        }
                        Some((idx, name, embedding))
                    }
                    Err(e) => {
                        let mut guard = errors.lock().expect("errors mutex poisoned");
                        guard.push(format!("{}: {e}", path.display()));
                        None
                    }
                }
            })
            .collect()
    });

    let errors = errors.into_inner().expect("errors mutex poisoned");
    if !errors.is_empty() {
        return Err(ExtractError::Image(format!(
            "embedding failed for {} file(s): {}",
            errors.len(),
            errors.join("; ")
        )));
    }

    results.sort_by_key(|(idx, _, _)| *idx);

    let mut names = Vec::with_capacity(results.len());
    let mut vectors = Vec::with_capacity(results.len() * EMBED_DIM);
    for (_, name, embedding) in results {
        names.push(name);
        vectors.extend(embedding);
    }

    Ok(EmbeddingIndex {
        dim: EMBED_DIM as u32,
        names,
        vectors,
    })
}

fn with_thread_embedder<R>(
    vision_model: &Path,
    f: impl FnOnce(&mut IconEmbedder) -> Result<R>,
) -> Result<R> {
    THREAD_EMBEDDER.with(|cell| {
        let mut guard = cell.borrow_mut();
        let needs_load = match guard.as_ref() {
            None => true,
            Some((path, _)) => path != vision_model,
        };
        if needs_load {
            *guard = Some((
                vision_model.to_path_buf(),
                IconEmbedder::load(vision_model)?,
            ));
        }
        f(&mut guard.as_mut().unwrap().1)
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

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
