use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, Result};

use super::build::{build_embedding_index, collect_png_embed_jobs, embedding_worker_jobs};

#[derive(Debug, Clone)]
pub struct BuildEmbeddingsOptions {
    pub png_dir: PathBuf,
    pub out: PathBuf,
    pub vision_model: PathBuf,
    pub template_size: u32,
}

/// Build `embeddings.bin` from a directory of template PNGs (`ui_icon_build_embeddings_file`).
pub fn build_embeddings_file(opts: &BuildEmbeddingsOptions) -> Result<()> {
    if !opts.png_dir.is_dir() {
        return Err(anyhow!("png dir not found: {}", opts.png_dir.display()));
    }
    if !opts.vision_model.is_file() {
        return Err(anyhow!(
            "vision model not found: {} (run scripts/download_mobileclip2.ps1)",
            opts.vision_model.display()
        ));
    }

    let jobs = collect_png_embed_jobs(&opts.png_dir).map_err(|e| anyhow!("{e}"))?;
    let png_count = jobs.len();

    let started = Instant::now();
    let index = build_embedding_index(&opts.png_dir, &opts.vision_model, opts.template_size)
        .map_err(|e| anyhow!("{e}"))?;

    index.save(&opts.out).map_err(|e| anyhow!("{e}"))?;

    let worker_jobs = embedding_worker_jobs(png_count);
    let elapsed = started.elapsed().as_secs_f64();
    let namespaces: Vec<_> = index
        .names
        .iter()
        .filter_map(|name| name.split_once(':').map(|(ns, _)| ns))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let ns_summary = if namespaces.is_empty() {
        String::new()
    } else {
        format!(", namespaces: {}", namespaces.join(", "))
    };
    println!(
        "embedded {} icons -> {} ({:.2}s, {} jobs{})",
        index.count(),
        opts.out.display(),
        elapsed,
        worker_jobs,
        ns_summary
    );
    Ok(())
}
