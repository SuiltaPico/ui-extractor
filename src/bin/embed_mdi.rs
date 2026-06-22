use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use ui_extractor::icon::{
    mdi_png_to_rgb256, EmbeddingIndex, IconEmbedder, EMBED_DIM,
};

#[derive(Debug, Parser)]
#[command(name = "embed-mdi", about = "Precompute MobileCLIP2-S0 embeddings for MDI icons")]
struct Cli {
    /// Input directory containing rasterized .png icons.
    #[arg(long, default_value = "assets/mdi/png-48-black")]
    png_dir: PathBuf,

    /// Output embedding index path.
    #[arg(long, default_value = "assets/mdi/embeddings.bin")]
    out: PathBuf,

    /// MobileCLIP2-S0 vision ONNX model.
    #[arg(long, default_value = "models/mobileclip2-s0-vision.onnx")]
    model: PathBuf,

    /// Mask normalization size before rendering to 256×256.
    #[arg(long, default_value_t = 48)]
    template_size: u32,

    /// Number of worker threads (embedding inference is sequential per model).
    #[arg(long, default_value_t = 1)]
    jobs: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    if !cli.png_dir.is_dir() {
        return Err(anyhow!("png dir not found: {}", cli.png_dir.display()));
    }
    if !cli.model.is_file() {
        return Err(anyhow!(
            "vision model not found: {} (run scripts/download_mobileclip2.ps1)",
            cli.model.display()
        ));
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(&cli.png_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        return Err(anyhow!("no png files under {}", cli.png_dir.display()));
    }

    let mut embedder = IconEmbedder::load(&cli.model)
        .map_err(|e| anyhow!("{e}"))?;

    let started = Instant::now();
    let done = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(vec![]);

    let mut names = Vec::with_capacity(paths.len());
    let mut vectors = Vec::with_capacity(paths.len() * EMBED_DIM);

    // ONNX session is not Sync; embed sequentially (fast enough for ~7400 icons).
    for path in &paths {
        match embed_one(path, &mut embedder, cli.template_size) {
            Ok((name, embedding)) => {
                names.push(name);
                vectors.extend(embedding);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n % 500 == 0 || n == paths.len() {
                    eprintln!("embedded {n}/{}", paths.len());
                }
            }
            Err(e) => {
                errors
                    .lock()
                    .expect("errors mutex poisoned")
                    .push(format!("{}: {e:#}", path.display()));
            }
        }
    }

    let errors = errors.into_inner().expect("errors mutex poisoned");
    if !errors.is_empty() {
        eprintln!("{} file(s) failed:", errors.len());
        for err in errors.iter().take(20) {
            eprintln!("  - {err}");
        }
        if errors.len() > 20 {
            eprintln!("  ... {} more", errors.len() - 20);
        }
        return Err(anyhow!("embedding failed for {} file(s)", errors.len()));
    }

    let index = EmbeddingIndex {
        dim: EMBED_DIM as u32,
        names,
        vectors,
    };
    index
        .save(&cli.out)
        .map_err(|e| anyhow!("{e}"))?;

    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "embedded {} icons -> {} ({:.2}s)",
        index.count(),
        cli.out.display(),
        elapsed
    );
    Ok(())
}

fn embed_one(
    path: &Path,
    embedder: &mut IconEmbedder,
    template_size: u32,
) -> Result<(String, Vec<f32>)> {
    let img = image::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let rgb = mdi_png_to_rgb256(&img, template_size);
    let embedding = embedder
        .embed_rgb256(&rgb)
        .map_err(|e| anyhow!("{e}"))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid png file name: {}", path.display()))?
        .to_string();

    Ok((name, embedding))
}
