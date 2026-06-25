use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use image::GrayImage;
use ui_extractor::{
    format_ms, rasterize_svg_icons, render_annotation, run_cases, resolve_models_dir, Bounds, ExtractConfig, IconConfig,
    IconPack, IconRasterColor, LayoutConfig, OcrConfig, RasterizeSvgOptions,
    DEFAULT_ICON_INDEX_PACK, DEFAULT_OCR_PACK, ExtractEngine,
};

#[derive(Parser)]
#[command(name = "ui-extractor", about = "Extract UI trees and text from screenshots")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract UI tree from a single screenshot
    Extract {
        /// Input screenshot path
        #[arg(short, long)]
        input: PathBuf,

        /// Output JSON path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Also write annotated PNG next to JSON output
        #[arg(long)]
        annotate: bool,

        /// Skip OCR; layout tree only
        #[arg(long)]
        layout_only: bool,

        /// Skip icon recognition
        #[arg(long)]
        no_icon: bool,

        /// Manifest pack root (`{models_dir}/{pack_id}/`)
        #[arg(long, default_value = "models")]
        models_dir: PathBuf,

        /// OCR model pack id
        #[arg(long, default_value = DEFAULT_OCR_PACK)]
        ocr_pack: String,

        /// Icon index pack id
        #[arg(long, default_value = DEFAULT_ICON_INDEX_PACK)]
        icon_index_pack: String,

        /// Minimum contour area in pixels
        #[arg(long, default_value_t = 100)]
        min_area: i64,

        /// OCR input long-edge limit in pixels (0 = full resolution)
        #[arg(long, default_value_t = 960)]
        ocr_max_side: u32,

        /// Write intermediate pipeline images to pipeline/ next to output or input
        #[arg(long)]
        dump_pipeline: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,

        #[command(flatten)]
        icon: IconExtractArgs,
    },
    /// Process all cases under tests/cases (input image -> output.json + annotated.png + skeleton.html)
    Cases {
        /// Directory containing case subfolders
        #[arg(long, default_value = "tests/cases")]
        dir: PathBuf,

        /// Skip OCR; layout tree only
        #[arg(long)]
        layout_only: bool,

        /// Skip icon recognition
        #[arg(long)]
        no_icon: bool,

        /// Manifest pack root
        #[arg(long, default_value = "models")]
        models_dir: PathBuf,

        /// OCR model pack id
        #[arg(long, default_value = DEFAULT_OCR_PACK)]
        ocr_pack: String,

        /// Icon index pack id
        #[arg(long, default_value = DEFAULT_ICON_INDEX_PACK)]
        icon_index_pack: String,

        /// Minimum contour area in pixels
        #[arg(long, default_value_t = 100)]
        min_area: i64,

        /// OCR input long-edge limit in pixels (0 = full resolution)
        #[arg(long, default_value_t = 960)]
        ocr_max_side: u32,

        #[command(flatten)]
        icon: IconExtractArgs,
    },
    /// Icon library utilities (rasterize, match, search)
    Icon {
        #[command(subcommand)]
        command: IconCommand,
    },
}

#[derive(Subcommand)]
enum IconCommand {
    /// Rasterize SVG icons to PNG templates
    RasterizeSvg {
        /// Input SVG directory
        #[arg(long, default_value = "assets/svg")]
        svg_dir: PathBuf,

        /// Output PNG directory
        #[arg(long, default_value = "assets/icons")]
        out_dir: PathBuf,

        /// Output edge length in pixels
        #[arg(long, default_value_t = 48)]
        size: u32,

        /// Icon color on transparent background
        #[arg(long, value_enum, default_value_t = RasterColor::Black)]
        color: RasterColor,

        /// Parallel worker threads (default: CPU count)
        #[arg(long)]
        jobs: Option<usize>,

        /// Skip PNGs that already exist
        #[arg(long)]
        skip_existing: bool,
    },
    /// Match a screenshot crop against the icon library
    Match {
        /// Input image path
        #[arg(short, long)]
        input: PathBuf,

        #[command(flatten)]
        region: RegionArgs,

        #[command(flatten)]
        pack: IconPackArgs,
    },
    /// Top-k cosine search against the icon library
    Search {
        /// Input image path
        #[arg(short, long)]
        input: PathBuf,

        /// Number of hits to return
        #[arg(long, default_value_t = 5)]
        top_k: usize,

        #[command(flatten)]
        region: RegionArgs,

        #[command(flatten)]
        pack: IconPackArgs,
    },
}

#[derive(Args, Clone)]
struct IconExtractArgs {
    /// Minimum cosine similarity to accept an icon match (0–1)
    #[arg(long, default_value_t = 0.72)]
    min_cosine: f64,
}

impl IconExtractArgs {
    fn to_config(&self) -> IconConfig {
        IconConfig {
            min_cosine: self.min_cosine,
            ..IconConfig::default()
        }
    }
}

#[derive(Args, Clone)]
struct IconPackArgs {
    /// Manifest pack root
    #[arg(long, default_value = "models")]
    models_dir: PathBuf,

    /// Icon index pack id
    #[arg(long, default_value = DEFAULT_ICON_INDEX_PACK)]
    icon_index_pack: String,

    /// Minimum cosine similarity to accept a match (0–1)
    #[arg(long, default_value_t = 0.72)]
    min_cosine: f64,

    /// Template edge length in pixels for query preprocessing
    #[arg(long, default_value_t = 48)]
    template_size: u32,
}

impl IconPackArgs {
    fn load(&self) -> anyhow::Result<IconPack> {
        let config = ExtractConfig {
            models_dir: resolve_models_dir(Some(&self.models_dir)),
            icon_index_pack: self.icon_index_pack.clone(),
            icon: IconConfig {
                template_size: self.template_size,
                min_cosine: self.min_cosine,
                ..IconConfig::default()
            },
            run_icon: true,
            ..ExtractConfig::default()
        };
        let registry = ui_extractor::infer::Registry::open(&config.models_dir, config.runtime.clone())?;
        Ok(IconPack::from_registry(
            &registry,
            &config.icon_index_pack,
            self.template_size,
            config.icon,
        )?)
    }
}

#[derive(Args, Clone, Default)]
struct RegionArgs {
    #[arg(long)]
    x: Option<i32>,
    #[arg(long)]
    y: Option<i32>,
    #[arg(long)]
    width: Option<i32>,
    #[arg(long)]
    height: Option<i32>,
}

impl RegionArgs {
    fn parse(&self) -> anyhow::Result<Option<Bounds>> {
        match (self.x, self.y, self.width, self.height) {
            (None, None, None, None) => Ok(None),
            (Some(x), Some(y), Some(width), Some(height)) => {
                Ok(Some(Bounds::new(x, y, width, height)))
            }
            _ => anyhow::bail!("region requires --x, --y, --width, and --height together"),
        }
    }
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
}

#[derive(Clone, ValueEnum)]
enum RasterColor {
    Black,
    White,
}

impl From<RasterColor> for IconRasterColor {
    fn from(value: RasterColor) -> Self {
        match value {
            RasterColor::Black => IconRasterColor::Black,
            RasterColor::White => IconRasterColor::White,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract {
            input,
            output,
            annotate,
            layout_only,
            no_icon,
            models_dir,
            ocr_pack,
            icon_index_pack,
            min_area,
            ocr_max_side,
            dump_pipeline,
            format,
            icon,
        } => run_extract(
            input,
            output,
            annotate,
            layout_only,
            no_icon,
            models_dir,
            ocr_pack,
            icon_index_pack,
            min_area,
            ocr_max_side,
            dump_pipeline,
            format,
            icon,
        ),
        Command::Cases {
            dir,
            layout_only,
            no_icon,
            models_dir,
            ocr_pack,
            icon_index_pack,
            min_area,
            ocr_max_side,
            icon,
        } => run_cases_cmd(
            dir,
            layout_only,
            no_icon,
            models_dir,
            ocr_pack,
            icon_index_pack,
            min_area,
            ocr_max_side,
            icon,
        ),
        Command::Icon { command } => match command {
            IconCommand::RasterizeSvg {
                svg_dir,
                out_dir,
                size,
                color,
                jobs,
                skip_existing,
            } => run_icon_rasterize_svg(svg_dir, out_dir, size, color, jobs, skip_existing),
            IconCommand::Match { input, region, pack } => run_icon_match(input, region, pack),
            IconCommand::Search {
                input,
                top_k,
                region,
                pack,
            } => run_icon_search(input, top_k, region, pack),
        },
    }
}

fn build_config(
    layout_only: bool,
    no_icon: bool,
    models_dir: PathBuf,
    ocr_pack: String,
    icon_index_pack: String,
    min_area: i64,
    ocr_max_side: u32,
    pipeline_dump_dir: Option<PathBuf>,
    icon: IconExtractArgs,
) -> ExtractConfig {
    ExtractConfig {
        models_dir: resolve_models_dir(Some(&models_dir)),
        ocr_pack,
        icon_index_pack,
        layout: LayoutConfig {
            min_area,
            ..LayoutConfig::default()
        },
        ocr: OcrConfig {
            max_side: ocr_max_side,
            ..OcrConfig::default()
        },
        icon: icon.to_config(),
        run_ocr: !layout_only,
        run_icon: !no_icon,
        pipeline_dump_dir,
        ..ExtractConfig::default()
    }
}

fn run_extract(
    input: PathBuf,
    output: Option<PathBuf>,
    annotate: bool,
    layout_only: bool,
    no_icon: bool,
    models_dir: PathBuf,
    ocr_pack: String,
    icon_index_pack: String,
    min_area: i64,
    ocr_max_side: u32,
    dump_pipeline: bool,
    format: OutputFormat,
    icon: IconExtractArgs,
) -> anyhow::Result<()> {
    let pipeline_dump_dir = if dump_pipeline {
        Some(pipeline_dump_dir_for_extract(&input, output.as_ref()))
    } else {
        None
    };
    let config = build_config(
        layout_only,
        no_icon,
        models_dir,
        ocr_pack,
        icon_index_pack,
        min_area,
        ocr_max_side,
        pipeline_dump_dir,
        icon,
    );
    let mut engine = ExtractEngine::open(config)?;
    let (result, _) = engine.extract_from_path(&input)?;

    let json = match format {
        OutputFormat::Json => serde_json::to_string(&result)?,
        OutputFormat::Pretty => serde_json::to_string_pretty(&result)?,
    };

    if let Some(path) = output {
        std::fs::write(&path, &json)?;
        if annotate {
            let img = image::open(&input)?;
            let annotated = render_annotation(&img, &result);
            let png_path = path.with_extension("png");
            annotated.save(&png_path)?;
            eprintln!("wrote {}", png_path.display());
        }
        if dump_pipeline {
            eprintln!(
                "wrote pipeline stages under {}",
                pipeline_dump_dir_for_extract(&input, Some(&path)).display()
            );
        }
        eprintln!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

fn run_cases_cmd(
    dir: PathBuf,
    layout_only: bool,
    no_icon: bool,
    models_dir: PathBuf,
    ocr_pack: String,
    icon_index_pack: String,
    min_area: i64,
    ocr_max_side: u32,
    icon: IconExtractArgs,
) -> anyhow::Result<()> {
    let batch_start = std::time::Instant::now();
    let config = build_config(
        layout_only,
        no_icon,
        models_dir,
        ocr_pack,
        icon_index_pack,
        min_area,
        ocr_max_side,
        None,
        icon,
    );
    let summary = run_cases(&dir, &config)?;

    let batch_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "processed {} case(s) in {} | sum {}",
        summary.count,
        format_ms(batch_ms),
        summary.totals.format_stages()
    );

    Ok(())
}

fn run_icon_rasterize_svg(
    svg_dir: PathBuf,
    out_dir: PathBuf,
    size: u32,
    color: RasterColor,
    jobs: Option<usize>,
    skip_existing: bool,
) -> anyhow::Result<()> {
    rasterize_svg_icons(&RasterizeSvgOptions {
        svg_dir,
        out_dir,
        size,
        color: color.into(),
        jobs,
        skip_existing,
    })
}

fn run_icon_match(input: PathBuf, region: RegionArgs, pack: IconPackArgs) -> anyhow::Result<()> {
    let bounds = region.parse()?;
    let mut pack = pack.load()?;
    let img = image::open(&input)?;

    let hit = match bounds {
        Some(bounds) => pack.match_region(&img, &bounds),
        None => pack.match_image(&img)?,
    };

    let json = match hit {
        Some(hit) => serde_json::to_string_pretty(&hit)?,
        None => "null".to_string(),
    };
    println!("{json}");
    Ok(())
}

fn run_icon_search(
    input: PathBuf,
    top_k: usize,
    region: RegionArgs,
    pack: IconPackArgs,
) -> anyhow::Result<()> {
    let bounds = region.parse()?;
    let mut pack = pack.load()?;
    let img = image::open(&input)?;

    let embedding = match bounds {
        Some(bounds) => {
            let gray = img.to_luma8();
            let crop = crop_gray(&gray, &bounds)
                .ok_or_else(|| anyhow::anyhow!("invalid crop region for {}", input.display()))?;
            pack.embed_query_gray(&crop)?
        }
        None => pack.embed_query_image(&img)?,
    };

    let hits = pack.search_embedding(&embedding, top_k);
    println!("{}", serde_json::to_string_pretty(&hits)?);
    Ok(())
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

fn pipeline_dump_dir_for_extract(input: &PathBuf, output: Option<&PathBuf>) -> PathBuf {
    if let Some(out) = output {
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.join("pipeline");
            }
        }
    }
    input
        .parent()
        .map(|p| p.join("pipeline"))
        .unwrap_or_else(|| PathBuf::from("pipeline"))
}
