use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use resvg::{tiny_skia, usvg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconRasterColor {
    Black,
    White,
}

#[derive(Debug, Clone)]
pub struct RasterizeSvgOptions {
    pub svg_dir: PathBuf,
    pub out_dir: PathBuf,
    pub size: u32,
    pub color: IconRasterColor,
    pub jobs: Option<usize>,
    pub skip_existing: bool,
}

pub fn rasterize_svg_icons(opts: &RasterizeSvgOptions) -> Result<()> {
    if !opts.svg_dir.is_dir() {
        return Err(anyhow!("svg dir not found: {}", opts.svg_dir.display()));
    }
    fs::create_dir_all(&opts.out_dir)
        .with_context(|| format!("failed to create output dir {}", opts.out_dir.display()))?;

    let mut files: Vec<PathBuf> = fs::read_dir(&opts.svg_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("svg")))
        .filter(|path| {
            if !opts.skip_existing {
                return true;
            }
            let stem = path.file_stem().and_then(|s| s.to_str());
            stem.is_some_and(|name| !opts.out_dir.join(format!("{name}.png")).is_file())
        })
        .collect();
    files.sort();

    if files.is_empty() {
        if opts.skip_existing {
            println!("all png files already exist under {}", opts.out_dir.display());
        } else {
            println!("no svg files under {}", opts.svg_dir.display());
        }
        return Ok(());
    }

    let jobs = opts.jobs.unwrap_or_else(default_jobs).max(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs.min(files.len()))
        .build()
        .context("failed to build rayon thread pool")?;

    let done = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(vec![]);

    let started = std::time::Instant::now();
    pool.install(|| {
        files.par_iter().for_each(|path| {
            if let Err(e) = rasterize_one(path, &opts.out_dir, opts.size, opts.color) {
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
        opts.out_dir.display(),
        elapsed,
        jobs.min(files.len())
    );
    Ok(())
}

fn rasterize_one(svg_path: &Path, out_dir: &Path, size: u32, color: IconRasterColor) -> Result<()> {
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

fn tint_svg(svg: &str, color: IconRasterColor) -> String {
    match color {
        IconRasterColor::Black => svg.to_owned(),
        IconRasterColor::White => svg.replace("<path ", "<path fill=\"#FFFFFF\" "),
    }
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
