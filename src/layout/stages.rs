use std::path::Path;

use image::{GrayImage, Luma};
use imageproc::contours::Contour;
use imageproc::drawing::draw_line_segment_mut;
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use serde::Serialize;

use crate::{
    error::{ExtractError, Result},
    types::UiElement,
};

#[derive(Debug, Clone, Serialize)]
pub struct ContourFilterStats {
    pub total: usize,
    pub kept: usize,
    pub dropped_hole: usize,
    pub dropped_area: usize,
    pub dropped_thin: usize,
}

/// Intermediate images produced by the layout pipeline.
#[derive(Debug, Clone)]
pub struct LayoutStages {
    pub width: u32,
    pub height: u32,
    pub gray: GrayImage,
    pub blurred: GrayImage,
    pub edges: GrayImage,
    pub dilated: GrayImage,
    pub closed: GrayImage,
    pub contours_all: GrayImage,
    pub contours_kept: GrayImage,
    pub contours_dropped: GrayImage,
    pub filter_stats: ContourFilterStats,
    pub layout_before_prune: UiElement,
}

#[derive(Serialize)]
struct StageManifestEntry {
    file: &'static str,
    description: &'static str,
}

const MANIFEST: &[StageManifestEntry] = &[
    StageManifestEntry {
        file: "01_gray.png",
        description: "Grayscale input",
    },
    StageManifestEntry {
        file: "02_blurred.png",
        description: "Gaussian blur (3x3) before edge detection",
    },
    StageManifestEntry {
        file: "03_canny.png",
        description: "Canny edge map (raw edges, not yet thickened)",
    },
    StageManifestEntry {
        file: "04_dilated.png",
        description: "Dilated edges fed to contour finder",
    },
    StageManifestEntry {
        file: "05_closed.png",
        description: "Morphological close to bridge broken borders",
    },
    StageManifestEntry {
        file: "06_contours_all.png",
        description: "All contour paths (white on black), including holes and tiny regions",
    },
    StageManifestEntry {
        file: "07_contours_kept.png",
        description: "Contour paths that passed area / hole / size filters (actual shape, not AABB)",
    },
    StageManifestEntry {
        file: "07_contours_dropped.png",
        description: "Contours removed between stage 06 and 07 (red = rejected)",
    },
    StageManifestEntry {
        file: "filter_stats.json",
        description: "Counts of kept vs rejected contours by reason",
    },
    StageManifestEntry {
        file: "08_layout_before_prune.png",
        description: "Axis-aligned boxes from hierarchy before redundant-container pruning",
    },
    StageManifestEntry {
        file: "manifest.json",
        description: "Index of pipeline stage files",
    },
];

impl LayoutStages {
    pub fn render_contours(
        contours: &[Contour<u32>],
        width: u32,
        height: u32,
        keep: Option<&[bool]>,
    ) -> GrayImage {
        let mut img = GrayImage::from_pixel(width, height, Luma([0]));
        for (idx, contour) in contours.iter().enumerate() {
            let include = keep.map(|mask| mask.get(idx).copied().unwrap_or(false)).unwrap_or(true);
            if include {
                draw_contour_path(&mut img, contour);
            }
        }
        thicken_lines(&img, 1)
    }

    pub fn write_to_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(|e| ExtractError::Image(e.to_string()))?;

        save_gray(dir.join("01_gray.png"), &self.gray)?;
        save_gray(dir.join("02_blurred.png"), &self.blurred)?;
        save_gray(dir.join("03_canny.png"), &self.edges)?;
        save_gray(dir.join("04_dilated.png"), &self.dilated)?;
        save_gray(dir.join("05_closed.png"), &self.closed)?;
        save_gray(dir.join("06_contours_all.png"), &self.contours_all)?;
        save_gray(dir.join("07_contours_kept.png"), &self.contours_kept)?;
        save_gray(dir.join("07_contours_dropped.png"), &self.contours_dropped)?;

        let stats = serde_json::to_string_pretty(&self.filter_stats)
            .map_err(|e| ExtractError::Image(format!("filter stats encode failed: {e}")))?;
        std::fs::write(dir.join("filter_stats.json"), stats)
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let manifest = serde_json::to_string_pretty(MANIFEST)
            .map_err(|e| ExtractError::Image(format!("manifest encode failed: {e}")))?;
        std::fs::write(dir.join("manifest.json"), manifest)
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        Ok(())
    }

    pub fn write_layout_before_prune(
        &self,
        dir: &Path,
        source: &image::DynamicImage,
    ) -> Result<()> {
        let annotated =
            crate::annotate::render_layout_annotation(source, &self.layout_before_prune);
        annotated
            .save(dir.join("08_layout_before_prune.png"))
            .map_err(|e| ExtractError::Image(format!("failed to write layout preview: {e}")))?;
        Ok(())
    }
}

fn draw_contour_path(img: &mut GrayImage, contour: &Contour<u32>) {
    let points = &contour.points;
    if points.is_empty() {
        return;
    }

    for window in points.windows(2) {
        let a = window[0];
        let b = window[1];
        draw_line_segment_mut(
            img,
            (a.x as f32, a.y as f32),
            (b.x as f32, b.y as f32),
            Luma([255]),
        );
    }

    let first = points[0];
    let last = points[points.len() - 1];
    draw_line_segment_mut(
        img,
        (last.x as f32, last.y as f32),
        (first.x as f32, first.y as f32),
        Luma([255]),
    );
}

fn thicken_lines(img: &GrayImage, radius: u8) -> GrayImage {
    if radius == 0 {
        return img.clone();
    }
    dilate(img, Norm::LInf, radius)
}

fn save_gray(path: impl AsRef<Path>, img: &GrayImage) -> Result<()> {
    img.save(path.as_ref()).map_err(|e| {
        ExtractError::Image(format!(
            "failed to write {}: {e}",
            path.as_ref().display()
        ))
    })
}
