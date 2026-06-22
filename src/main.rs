use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use ui_extractor::{extract_from_path, run_cases, ExtractConfig, IconConfig, LayoutConfig, OcrConfig};

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

        /// Skip icon recognition (MDI template matching)
        #[arg(long)]
        no_icon: bool,

        /// MDI rasterized PNG directory for icon matching
        #[arg(long, default_value = "assets/mdi/png-48-black")]
        mdi_dir: PathBuf,

        /// Precomputed MDI embedding index (embed-mdi output)
        #[arg(long, default_value = "assets/mdi/embeddings.bin")]
        embedding_index: PathBuf,

        /// MobileCLIP2-S0 vision ONNX model
        #[arg(long, default_value = "models/mobileclip2-s0-vision.onnx")]
        icon_model: PathBuf,

        /// Minimum cosine similarity to accept an icon match (0–1)
        #[arg(long, default_value_t = 0.72)]
        icon_min_cosine: f64,

        /// PP-OCR model directory (det/rec ONNX + dict)
        #[arg(long, default_value = "models")]
        model_dir: PathBuf,

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
    },
    /// Process all cases under tests/cases (input image -> output.json + annotated.png + skeleton.html)
    Cases {
        /// Directory containing case subfolders
        #[arg(long, default_value = "tests/cases")]
        dir: PathBuf,

        /// Skip OCR; layout tree only
        #[arg(long)]
        layout_only: bool,

        /// Skip icon recognition (MDI template matching)
        #[arg(long)]
        no_icon: bool,

        /// MDI rasterized PNG directory for icon matching
        #[arg(long, default_value = "assets/mdi/png-48-black")]
        mdi_dir: PathBuf,

        /// Precomputed MDI embedding index (embed-mdi output)
        #[arg(long, default_value = "assets/mdi/embeddings.bin")]
        embedding_index: PathBuf,

        /// MobileCLIP2-S0 vision ONNX model
        #[arg(long, default_value = "models/mobileclip2-s0-vision.onnx")]
        icon_model: PathBuf,

        /// Minimum cosine similarity to accept an icon match (0–1)
        #[arg(long, default_value_t = 0.72)]
        icon_min_cosine: f64,

        /// PP-OCR model directory (det/rec ONNX + dict)
        #[arg(long, default_value = "models")]
        model_dir: PathBuf,

        /// Minimum contour area in pixels
        #[arg(long, default_value_t = 100)]
        min_area: i64,

        /// OCR input long-edge limit in pixels (0 = full resolution)
        #[arg(long, default_value_t = 960)]
        ocr_max_side: u32,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
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
            mdi_dir,
            embedding_index,
            icon_model,
            icon_min_cosine,
            model_dir,
            min_area,
            ocr_max_side,
            dump_pipeline,
            format,
        } => run_extract(
            input,
            output,
            annotate,
            layout_only,
            no_icon,
            mdi_dir,
            embedding_index,
            icon_model,
            icon_min_cosine,
            model_dir,
            min_area,
            ocr_max_side,
            dump_pipeline,
            format,
        ),
        Command::Cases {
            dir,
            layout_only,
            no_icon,
            mdi_dir,
            embedding_index,
            icon_model,
            icon_min_cosine,
            model_dir,
            min_area,
            ocr_max_side,
        } => run_cases_cmd(
            dir,
            layout_only,
            no_icon,
            mdi_dir,
            embedding_index,
            icon_model,
            icon_min_cosine,
            model_dir,
            min_area,
            ocr_max_side,
        ),
    }
}

fn build_config(
    layout_only: bool,
    no_icon: bool,
    mdi_dir: PathBuf,
    embedding_index: PathBuf,
    icon_model: PathBuf,
    icon_min_cosine: f64,
    model_dir: PathBuf,
    min_area: i64,
    ocr_max_side: u32,
    pipeline_dump_dir: Option<PathBuf>,
) -> ExtractConfig {
    ExtractConfig {
        layout: LayoutConfig {
            min_area,
            ..LayoutConfig::default()
        },
        ocr: OcrConfig {
            model_dir,
            max_side: ocr_max_side,
            ..OcrConfig::default()
        },
        icon: IconConfig {
            mdi_png_dir: mdi_dir,
            embedding_index,
            vision_model: icon_model,
            min_cosine: icon_min_cosine,
            ..IconConfig::default()
        },
        run_ocr: !layout_only,
        run_icon: !no_icon,
        pipeline_dump_dir,
    }
}

fn run_extract(
    input: PathBuf,
    output: Option<PathBuf>,
    annotate: bool,
    layout_only: bool,
    no_icon: bool,
    mdi_dir: PathBuf,
    embedding_index: PathBuf,
    icon_model: PathBuf,
    icon_min_cosine: f64,
    model_dir: PathBuf,
    min_area: i64,
    ocr_max_side: u32,
    dump_pipeline: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let pipeline_dump_dir = if dump_pipeline {
        Some(pipeline_dump_dir_for_extract(&input, output.as_ref()))
    } else {
        None
    };
    let config = build_config(
        layout_only,
        no_icon,
        mdi_dir,
        embedding_index,
        icon_model,
        icon_min_cosine,
        model_dir,
        min_area,
        ocr_max_side,
        pipeline_dump_dir,
    );
    let result = extract_from_path(&input, &config)?;

    let json = match format {
        OutputFormat::Json => serde_json::to_string(&result)?,
        OutputFormat::Pretty => serde_json::to_string_pretty(&result)?,
    };

    if let Some(path) = output {
        std::fs::write(&path, &json)?;
        if annotate {
            let img = image::open(&input)?;
            let annotated = ui_extractor::render_annotation(&img, &result);
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
    mdi_dir: PathBuf,
    embedding_index: PathBuf,
    icon_model: PathBuf,
    icon_min_cosine: f64,
    model_dir: PathBuf,
    min_area: i64,
    ocr_max_side: u32,
) -> anyhow::Result<()> {
    let batch_start = std::time::Instant::now();
    let config = build_config(
        layout_only,
        no_icon,
        mdi_dir,
        embedding_index,
        icon_model,
        icon_min_cosine,
        model_dir,
        min_area,
        ocr_max_side,
        None,
    );
    let summary = run_cases(&dir, &config)?;

    let batch_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "processed {} case(s) in {} | sum {}",
        summary.count,
        ui_extractor::format_ms(batch_ms),
        summary.totals.format_stages()
    );

    Ok(())
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
