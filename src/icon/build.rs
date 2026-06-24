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

/// One PNG template and its label in the merged embedding index.
#[derive(Debug, Clone)]
pub struct PngEmbedJob {
    pub name: String,
    pub path: PathBuf,
}

/// Collect PNG templates under `root`.
///
/// Layout with namespace subdirectories (recommended):
///
/// ```text
/// assets/icons/mdi/home.png       -> mdi:home
/// assets/icons/tabler/home.png    -> tabler:home
/// ```
///
/// Legacy flat layout (no PNG subdirectories): `home.png` -> `home`.
pub fn collect_png_embed_jobs(root: &Path) -> Result<Vec<PngEmbedJob>> {
    if !root.is_dir() {
        return Err(ExtractError::Image(format!(
            "PNG directory not found: {}",
            root.display()
        )));
    }

    let mut namespace_dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut root_pngs: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(root).map_err(|e| ExtractError::Image(e.to_string()))? {
        let entry = entry.map_err(|e| ExtractError::Image(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            let Some(ns) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let pngs = list_png_files(&path)?;
            if !pngs.is_empty() {
                namespace_dirs.push((ns.to_string(), path));
            }
        } else if path.extension().is_some_and(|ext| ext == "png") {
            root_pngs.push(path);
        }
    }

    let mut jobs = Vec::new();
    if namespace_dirs.is_empty() {
        for path in root_pngs {
            let name = png_label_from_path(&path, None)?;
            jobs.push(PngEmbedJob { name, path });
        }
    } else {
        namespace_dirs.sort_by(|a, b| a.0.cmp(&b.0));
        for (namespace, dir) in namespace_dirs {
            let mut pngs = list_png_files(&dir)?;
            pngs.sort();
            for path in pngs {
                let name = png_label_from_path(&path, Some(&namespace))?;
                jobs.push(PngEmbedJob { name, path });
            }
        }
    }

    jobs.sort_by(|a, b| a.name.cmp(&b.name));

    let mut seen = std::collections::HashSet::new();
    for job in &jobs {
        if !seen.insert(&job.name) {
            return Err(ExtractError::Image(format!(
                "duplicate icon label in embedding index: {}",
                job.name
            )));
        }
    }

    if jobs.is_empty() {
        return Err(ExtractError::Image(format!(
            "no PNG files under {} (expected flat *.png or <namespace>/*.png)",
            root.display()
        )));
    }

    Ok(jobs)
}

fn list_png_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| ExtractError::Image(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn png_label_from_path(path: &Path, namespace: Option<&str>) -> Result<String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ExtractError::Image(format!("invalid PNG file name: {}", path.display()))
        })?;
    Ok(match namespace {
        Some(ns) => format!("{ns}:{stem}"),
        None => stem.to_string(),
    })
}

/// Build a precomputed embedding index from a directory of PNG icons.
///
/// Supports flat PNGs or namespace subdirectories; see [`collect_png_embed_jobs`].
/// Embeddings are computed in parallel using one model session per worker thread.
pub fn build_embedding_index(
    png_dir: &Path,
    vision_model: &Path,
    template_size: u32,
) -> Result<EmbeddingIndex> {
    let jobs = collect_png_embed_jobs(png_dir)?;
    build_embedding_index_from_jobs(&jobs, vision_model, template_size)
}

pub fn build_embedding_index_from_jobs(
    jobs: &[PngEmbedJob],
    vision_model: &Path,
    template_size: u32,
) -> Result<EmbeddingIndex> {
    let worker_jobs = embedding_worker_jobs(jobs.len());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_jobs)
        .build()
        .map_err(|e| ExtractError::Image(e.to_string()))?;

    let errors: Mutex<Vec<String>> = Mutex::new(vec![]);
    let done = AtomicUsize::new(0);
    let total = jobs.len();
    let vision_model = vision_model.to_path_buf();

    let mut results: Vec<(usize, String, Vec<f32>)> = pool.install(|| {
        jobs
            .par_iter()
            .enumerate()
            .filter_map(|(idx, job)| {
                let result = with_thread_embedder(&vision_model, |embedder| {
                    embed_one(&job.path, embedder, template_size)
                });
                match result {
                    Ok((_, embedding)) => {
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if n % 500 == 0 || n == total {
                            eprintln!("embedded {n}/{total}");
                        }
                        Some((idx, job.name.clone(), embedding))
                    }
                    Err(e) => {
                        let mut guard = errors.lock().expect("errors mutex poisoned");
                        guard.push(format!("{}: {e}", job.path.display()));
                        None
                    }
                }
            })
            .collect()
    });

    // Drop ONNX sessions on worker threads before joining the pool. Without this,
    // Windows + ORT can hang indefinitely during thread-local destructors (0% CPU).
    eprintln!("releasing embedder sessions...");
    drop_thread_embedders(&pool);

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

    Ok(EmbeddingIndex::from_float_vectors(
        EMBED_DIM as u32,
        names,
        vectors,
    )
    .map_err(|e| ExtractError::Image(e.to_string()))?)
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

pub fn embedding_worker_jobs(png_count: usize) -> usize {
    default_jobs().max(1).min(png_count)
}

fn default_jobs() -> usize {
    if infer_core::RuntimeConfig::from_env_or_default().prefer_gpu_single_session() {
        return 1;
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn drop_thread_embedders(pool: &rayon::ThreadPool) {
    pool.broadcast(|_| {
        THREAD_EMBEDDER.with(|cell| {
            *cell.borrow_mut() = None;
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_png_jobs_namespace_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("mdi")).unwrap();
        fs::create_dir_all(root.join("tabler")).unwrap();
        fs::write(root.join("mdi/home.png"), b"png").unwrap();
        fs::write(root.join("tabler/home.png"), b"png").unwrap();

        let jobs = collect_png_embed_jobs(root).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].name, "mdi:home");
        assert_eq!(jobs[1].name, "tabler:home");
    }

    #[test]
    fn collect_png_jobs_flat_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("home.png"), b"png").unwrap();

        let jobs = collect_png_embed_jobs(root).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "home");
    }

    #[test]
    #[cfg(unix)]
    fn collect_png_jobs_rejects_duplicate_labels() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("mdi")).unwrap();
        fs::write(root.join("mdi/home.png"), b"png").unwrap();
        fs::write(root.join("mdi/Home.png"), b"png").unwrap();

        let err = collect_png_embed_jobs(root).unwrap_err().to_string();
        assert!(err.contains("duplicate icon label"));
    }
}
