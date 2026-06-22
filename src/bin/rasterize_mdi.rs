use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use resvg::{tiny_skia, usvg};

#[derive(Debug, Clone, Parser)]
#[command(name = "rasterize-mdi", about = "Batch convert MDI SVG icons to PNG")]
struct Cli {
    /// Input directory containing .svg files.
    #[arg(long, default_value = "assets/mdi/svg")]
    svg_dir: PathBuf,

    /// Output directory for generated .png files.
    #[arg(long, default_value = "assets/mdi/png-48-black")]
    out_dir: PathBuf,

    /// PNG width/height in pixels.
    #[arg(long, default_value_t = 48)]
    size: u32,

    /// Output icon color.
    #[arg(long, value_enum, default_value_t = IconColor::Black)]
    color: IconColor,

    /// Number of worker threads.
    #[arg(long)]
    jobs: Option<usize>,

    /// Skip files whose PNG output already exists.
    #[arg(long)]
    skip_existing: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum IconColor {
    Black,
    White,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    if !cli.svg_dir.is_dir() {
        return Err(anyhow!("svg dir not found: {}", cli.svg_dir.display()));
    }
    fs::create_dir_all(&cli.out_dir)
        .with_context(|| format!("failed to create output dir {}", cli.out_dir.display()))?;

    let mut files: Vec<PathBuf> = fs::read_dir(&cli.svg_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("svg")))
        .filter(|path| {
            if !cli.skip_existing {
                return true;
            }
            let stem = path.file_stem().and_then(|s| s.to_str());
            stem.is_some_and(|name| !cli.out_dir.join(format!("{name}.png")).is_file())
        })
        .collect();
    files.sort();

    if files.is_empty() {
        if cli.skip_existing {
            println!("all png files already exist under {}", cli.out_dir.display());
        } else {
            println!("no svg files under {}", cli.svg_dir.display());
        }
        return Ok(());
    }

    let jobs = cli.jobs.unwrap_or_else(default_jobs).max(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs.min(files.len()))
        .build()
        .context("failed to build rayon thread pool")?;

    let done = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(vec![]);

    let started = std::time::Instant::now();
    pool.install(|| {
        files.par_iter().for_each(|path| {
            if let Err(e) = rasterize_one(path, &cli.out_dir, cli.size, cli.color) {
                let mut guard = errors.lock().expect("errors mutex poisoned");
                guard.push(format!("{}: {e:#}", path.display()));
                return;
            }
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 500 == 0 || n == files.len() {
                eprintln!("rasterized {n}/{}", files.len());
            }
        });
    });

    let elapsed = started.elapsed().as_secs_f64();
    let finished = done.load(Ordering::Relaxed);
    let errors = errors.into_inner().expect("errors mutex poisoned");

    if !errors.is_empty() {
        eprintln!("{} file(s) failed:", errors.len());
        for err in errors.iter().take(20) {
            eprintln!("  - {err}");
        }
        if errors.len() > 20 {
            eprintln!("  ... {} more", errors.len() - 20);
        }
        return Err(anyhow!("rasterization failed for {} file(s)", errors.len()));
    }

    println!(
        "rasterized {} icons -> {} ({:.2}s, {} jobs)",
        finished,
        cli.out_dir.display(),
        elapsed,
        jobs.min(files.len())
    );
    Ok(())
}

fn rasterize_one(svg_path: &Path, out_dir: &Path, size: u32, color: IconColor) -> Result<()> {
    let svg_text = fs::read_to_string(svg_path)
        .with_context(|| format!("failed to read {}", svg_path.display()))?;
    let svg_text = tint_svg(&svg_text, color);

    let tree = usvg::Tree::from_str(&svg_text, &usvg::Options::default())
        .with_context(|| format!("failed to parse SVG {}", svg_path.display()))?;

    let svg_size = tree.size();
    let sx = size as f32 / svg_size.width();
    let sy = size as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(sx, sy);

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| anyhow!("failed to allocate pixmap for {}", svg_path.display()))?;

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let stem = svg_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid svg file name: {}", svg_path.display()))?;
    let out_path = out_dir.join(format!("{stem}.png"));
    pixmap
        .save_png(&out_path)
        .with_context(|| format!("failed to write {}", out_path.display()))?;
    Ok(())
}

fn tint_svg(svg: &str, color: IconColor) -> String {
    match color {
        IconColor::Black => svg.to_owned(),
        // MDI SVGs generally rely on implicit black fill.
        IconColor::White => svg.replace("<path ", "<path fill=\"#FFFFFF\" "),
    }
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
