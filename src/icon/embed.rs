use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Result};

use super::{build_embedding_index, IconEmbedder};

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

    let png_count = count_pngs(&opts.png_dir)?;
    if png_count == 0 {
        return Err(anyhow!("no png files under {}", opts.png_dir.display()));
    }

    let mut embedder = IconEmbedder::load(&opts.vision_model).map_err(|e| anyhow!("{e}"))?;

    let started = Instant::now();
    let index = build_embedding_index(&opts.png_dir, &mut embedder, opts.template_size)
        .map_err(|e| anyhow!("{e}"))?;

    index.save(&opts.out).map_err(|e| anyhow!("{e}"))?;

    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "embedded {} icons -> {} ({:.2}s)",
        index.count(),
        opts.out.display(),
        elapsed
    );
    Ok(())
}

fn count_pngs(dir: &Path) -> Result<usize> {
    let count = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "png")
        })
        .count();
    Ok(count)
}
