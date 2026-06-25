use image::{GrayImage, Rgb, RgbImage};

pub use crate::infer::{EMBED_DIM, INPUT_SIZE};

/// Render a grayscale screenshot crop as 256×256 RGB (white background, black ink).
pub fn icon_crop_to_rgb256(gray_crop: &GrayImage, mask_size: u32) -> RgbImage {
    let (mask, _) = normalize_query_mask(gray_crop, mask_size);
    mask_to_rgb256(&mask, mask_size)
}

/// Render a normalized ink mask as 256×256 RGB (white background, black ink).
pub fn mask_to_rgb256(mask: &[u8], mask_size: u32) -> RgbImage {
    let mut src = RgbImage::from_fn(mask_size, mask_size, |x, y| {
        let idx = (y * mask_size + x) as usize;
        if mask.get(idx).copied().unwrap_or(0) > 0 {
            Rgb([0, 0, 0])
        } else {
            Rgb([255, 255, 255])
        }
    });

    if mask_size != INPUT_SIZE {
        src = image::imageops::resize(
            &src,
            INPUT_SIZE,
            INPUT_SIZE,
            image::imageops::FilterType::Triangle,
        );
    }
    src
}

/// Resize a grayscale crop and derive an ink mask, trying both polarities.
pub fn normalize_query_mask(gray_crop: &GrayImage, size: u32) -> (Vec<u8>, bool) {
    let resized = image::imageops::resize(
        gray_crop,
        size,
        size,
        image::imageops::FilterType::Triangle,
    );

    let dark = ink_mask_from_gray(&resized, true);
    let light = ink_mask_from_gray(&resized, false);
    let dark_ink = dark.iter().filter(|&&v| v > 0).count();
    let light_ink = light.iter().filter(|&&v| v > 0).count();

    let total = (size * size) as usize;
    let dark_ok = ink_ratio_ok(dark_ink, total);
    let light_ok = ink_ratio_ok(light_ink, total);

    match (dark_ok, light_ok) {
        (true, true) => {
            if dark_ink <= light_ink {
                (dark, true)
            } else {
                (light, false)
            }
        }
        (true, false) => (dark, true),
        (false, true) => (light, false),
        (false, false) => {
            if dark_ink.abs_diff(light_ink) <= 8 {
                (dark, true)
            } else if dark_ink < light_ink {
                (dark, true)
            } else {
                (light, false)
            }
        }
    }
}

fn ink_ratio_ok(ink: usize, total: usize) -> bool {
    if total == 0 {
        return false;
    }
    let ratio = ink as f64 / total as f64;
    (0.03..=0.72).contains(&ratio)
}

fn ink_mask_from_gray(img: &GrayImage, dark_icon: bool) -> Vec<u8> {
    let bg = estimate_background(img);
    let threshold = adaptive_threshold(img, bg);

    img.pixels()
        .map(|p| {
            let v = p.0[0];
            let ink = if dark_icon {
                v.saturating_add(threshold) < bg
            } else {
                v.saturating_sub(threshold) > bg
            };
            ink as u8
        })
        .collect()
}

fn estimate_background(img: &GrayImage) -> u8 {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 255;
    }

    let mut samples = Vec::new();
    for x in 0..w {
        samples.push(img.get_pixel(x, 0).0[0]);
        samples.push(img.get_pixel(x, h - 1).0[0]);
    }
    for y in 1..h.saturating_sub(1) {
        samples.push(img.get_pixel(0, y).0[0]);
        samples.push(img.get_pixel(w - 1, y).0[0]);
    }

    if samples.is_empty() {
        return 128;
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn adaptive_threshold(img: &GrayImage, bg: u8) -> u8 {
    let values: Vec<u8> = img.pixels().map(|p| p.0[0]).collect();
    let mut diffs: Vec<u8> = values
        .iter()
        .map(|v| v.abs_diff(bg))
        .filter(|d| *d > 0)
        .collect();
    if diffs.is_empty() {
        return 24;
    }
    diffs.sort_unstable();
    let median = diffs[diffs.len() / 2];
    median.clamp(12, 48)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn mask_renders_black_square() {
        let mut img = GrayImage::from_pixel(32, 32, Luma([255]));
        for y in 8..24 {
            for x in 8..24 {
                img.put_pixel(x, y, Luma([0]));
            }
        }
        let rgb = icon_crop_to_rgb256(&img, 16);
        assert_eq!(rgb.dimensions(), (INPUT_SIZE, INPUT_SIZE));
        let center = rgb.get_pixel(INPUT_SIZE / 2, INPUT_SIZE / 2).0;
        assert_eq!(center, [0, 0, 0]);
    }

    #[test]
    fn normalize_filled_square() {
        let mut img = GrayImage::from_pixel(32, 32, Luma([255]));
        for y in 8..24 {
            for x in 8..24 {
                img.put_pixel(x, y, Luma([0]));
            }
        }
        let (mask, dark) = normalize_query_mask(&img, 16);
        assert!(dark);
        assert!(mask.iter().filter(|&&v| v > 0).count() > 20);
    }
}
