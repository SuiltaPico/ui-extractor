use std::path::{Path, PathBuf};
use std::time::Instant;

use image::DynamicImage;

use crate::{
    annotate::render_annotation,
    error::{ExtractError, Result},
    pipeline::{extract_from_image_timed, ExtractConfig, ExtractTimings},
    skeleton::write_skeleton_html,
    types::ExtractResult,
};

pub struct CaseOutputs {
    pub case_dir: PathBuf,
    pub json_path: PathBuf,
    pub annotated_path: PathBuf,
    pub skeleton_path: PathBuf,
    pub pipeline_dir: PathBuf,
    pub timings: CaseTimings,
}

#[derive(Debug, Clone, Default)]
pub struct CaseBatchSummary {
    pub count: usize,
    pub totals: CaseTimings,
}

#[derive(Debug, Clone, Default)]
pub struct CaseTimings {
    pub load_ms: f64,
    pub extract: ExtractTimings,
    pub write_json_ms: f64,
    pub write_annotation_ms: f64,
    pub total_ms: f64,
}

impl CaseTimings {
    pub fn format_stages(&self) -> String {
        let e = &self.extract;
        let mut parts = vec![
            format!("load {}", format_ms(self.load_ms)),
            format!("gray {}", format_ms(e.gray_ms)),
            format!("layout {}", format_ms(e.layout_ms)),
        ];

        if e.pipeline_dump_ms > 0.0 {
            parts.push(format!("pipeline_dump {}", format_ms(e.pipeline_dump_ms)));
        }

        if e.ocr_total_ms() > 0.0 {
            if e.parallel_ms > 0.0 {
                parts.push(format!("parallel_wall {}", format_ms(e.parallel_ms)));
            }
            if e.ocr.init_ms > 0.0 {
                parts.push(format!("ocr_init {}", format_ms(e.ocr.init_ms)));
            }
            parts.push(format!("ocr {}", format_ms(e.ocr.predict_ms)));
        }

        if e.attach_words_ms > 0.0 {
            parts.push(format!("attach {}", format_ms(e.attach_words_ms)));
        }

        if e.icon.timings.load_ms > 0.0 || e.icon.timings.match_ms > 0.0 {
            parts.push(format!(
                "icon {}+{}",
                format_ms(e.icon.timings.load_ms),
                format_ms(e.icon.timings.match_ms)
            ));
        }

        parts.push(format!("json {}", format_ms(self.write_json_ms)));
        parts.push(format!("annotate {}", format_ms(self.write_annotation_ms)));
        parts.push(format!("total {}", format_ms(self.total_ms)));

        parts.join(" | ")
    }

    pub fn accumulate(&mut self, other: &Self) {
        self.load_ms += other.load_ms;
        self.extract.gray_ms += other.extract.gray_ms;
        self.extract.layout_ms += other.extract.layout_ms;
        self.extract.pipeline_dump_ms += other.extract.pipeline_dump_ms;
        self.extract.parallel_ms += other.extract.parallel_ms;
        self.extract.ocr.init_ms += other.extract.ocr.init_ms;
        self.extract.ocr.predict_ms += other.extract.ocr.predict_ms;
        self.extract.attach_words_ms += other.extract.attach_words_ms;
        self.extract.icon.timings.load_ms += other.extract.icon.timings.load_ms;
        self.extract.icon.timings.match_ms += other.extract.icon.timings.match_ms;
        self.write_json_ms += other.write_json_ms;
        self.write_annotation_ms += other.write_annotation_ms;
        self.total_ms += other.total_ms;
    }
}

const INPUT_NAMES: &[&str] = &[
    "input.png",
    "input.jpg",
    "input.jpeg",
    "input.webp",
    "input.gif",
];

/// Process every subdirectory of `cases_dir` serially, printing each case as it completes.
pub fn run_cases(cases_dir: &Path, config: &ExtractConfig) -> Result<CaseBatchSummary> {
    if !cases_dir.is_dir() {
        return Err(ExtractError::Image(format!(
            "cases directory not found: {}",
            cases_dir.display()
        )));
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(cases_dir)
        .map_err(|e| ExtractError::Image(e.to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    let jobs: Vec<(PathBuf, PathBuf)> = entries
        .into_iter()
        .filter_map(|case_dir| {
            find_case_input(&case_dir).map(|input| (case_dir, input))
        })
        .collect();

    if jobs.is_empty() {
        return Err(ExtractError::Image(format!(
            "no cases with input.(png|jpg|jpeg|webp|gif) under {}",
            cases_dir.display()
        )));
    }

    let mut summary = CaseBatchSummary::default();
    for (case_dir, input) in jobs {
        let out = process_case(&case_dir, &input, config)?;
        eprintln!(
            "{} -> {} + {} + {} + {}",
            out.case_dir.display(),
            out.json_path.display(),
            out.annotated_path.display(),
            out.skeleton_path.display(),
            out.pipeline_dir.display()
        );
        eprintln!("  {}", out.timings.format_stages());
        summary.totals.accumulate(&out.timings);
        summary.count += 1;
    }

    Ok(summary)
}

fn find_case_input(case_dir: &Path) -> Option<PathBuf> {
    INPUT_NAMES
        .iter()
        .map(|name| case_dir.join(name))
        .find(|path| path.is_file())
}

pub fn process_case(case_dir: &Path, input: &Path, config: &ExtractConfig) -> Result<CaseOutputs> {
    let total_start = Instant::now();
    let mut timings = CaseTimings::default();

    let load_start = Instant::now();
    let img = image::open(input).map_err(|_| ExtractError::ImageRead(input.display().to_string()))?;
    timings.load_ms = ms_since(load_start);

    let pipeline_dir = case_dir.join("pipeline");
    let mut config = config.clone();
    config.pipeline_dump_dir = Some(pipeline_dir.clone());

    let (result, extract_timings) = extract_from_image_timed(&img, &config)?;
    timings.extract = extract_timings;

    let json_path = case_dir.join("output.json");
    let annotated_path = case_dir.join("annotated.png");
    let skeleton_path = case_dir.join("skeleton.html");

    let json_start = Instant::now();
    write_json(&json_path, &result)?;
    write_skeleton_html(&skeleton_path, &result)?;
    timings.write_json_ms = ms_since(json_start);

    let annotate_start = Instant::now();
    write_annotation(&img, &result, &annotated_path)?;
    timings.write_annotation_ms = ms_since(annotate_start);

    timings.total_ms = ms_since(total_start);

    Ok(CaseOutputs {
        case_dir: case_dir.to_path_buf(),
        json_path,
        annotated_path,
        skeleton_path,
        pipeline_dir,
        timings,
    })
}

fn write_json(path: &Path, result: &ExtractResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| ExtractError::Image(format!("json encode failed: {e}")))?;
    std::fs::write(path, json).map_err(|e| ExtractError::Image(e.to_string()))
}

fn write_annotation(source: &DynamicImage, result: &ExtractResult, path: &Path) -> Result<()> {
    let annotated = render_annotation(source, result);
    annotated
        .save(path)
        .map_err(|e| ExtractError::Image(format!("failed to write {}: {e}", path.display())))
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

pub fn format_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else if ms >= 10.0 {
        format!("{:.0}ms", ms)
    } else {
        format!("{:.1}ms", ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn find_case_input_supports_jpg() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("input.jpg"), b"fake").unwrap();
        let found = find_case_input(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "input.jpg");
    }

    #[test]
    fn format_ms_uses_seconds_for_large_values() {
        assert_eq!(format_ms(1500.0), "1.50s");
        assert_eq!(format_ms(42.0), "42ms");
    }
}

