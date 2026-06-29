use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use ui_extractor::{
    format_ms, render_annotation, run_cases, resolve_models_dir, ExtractConfig, IconConfig,
    LayoutConfig, OcrConfig, RuntimeConfig, DEFAULT_ICON_INDEX_PACK, DEFAULT_OCR_PACK,
    ExtractEngine,
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

        /// Runtime config JSON file or inline JSON (ORT/MNN execution providers)
        #[arg(long)]
        runtime_config: Option<String>,

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

        /// Runtime config JSON file or inline JSON
        #[arg(long)]
        runtime_config: Option<String>,

        /// Minimum contour area in pixels
        #[arg(long, default_value_t = 100)]
        min_area: i64,

        /// OCR input long-edge limit in pixels (0 = full resolution)
        #[arg(long, default_value_t = 960)]
        ocr_max_side: u32,

        #[command(flatten)]
        icon: IconExtractArgs,
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
            models_dir,
            ocr_pack,
            icon_index_pack,
            min_area,
            ocr_max_side,
            dump_pipeline,
            format,
            icon,
            runtime_config,
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
            runtime_config,
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
            runtime_config,
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
            runtime_config,
        ),
    }
}

fn parse_runtime_config(raw: Option<&str>) -> anyhow::Result<RuntimeConfig> {
    let Some(text) = raw else {
        return Ok(RuntimeConfig::default());
    };
    let path = PathBuf::from(text);
    let json = if path.is_file() {
        std::fs::read_to_string(path)?
    } else {
        text.to_string()
    };
    RuntimeConfig::from_json(&json).map_err(|e| anyhow::anyhow!("runtime config: {e}"))
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
    runtime_config: Option<String>,
) -> anyhow::Result<ExtractConfig> {
    Ok(ExtractConfig {
        models_dir: resolve_models_dir(Some(&models_dir)),
        runtime: parse_runtime_config(runtime_config.as_deref())?,
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
    })
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
    runtime_config: Option<String>,
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
        runtime_config,
    )?;
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
    runtime_config: Option<String>,
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
        runtime_config,
    )?;
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
