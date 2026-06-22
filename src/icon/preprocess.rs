use image::{DynamicImage, GrayImage, Rgb, RgbImage};

use super::library::normalize_query_mask;

pub const INPUT_SIZE: u32 = 256;
pub const EMBED_DIM: usize = 512;

/// Render a grayscale screenshot crop as 256×256 RGB (white background, black ink).
pub fn icon_crop_to_rgb256(gray_crop: &GrayImage, mask_size: u32) -> RgbImage {
    let (mask, _) = normalize_query_mask(gray_crop, mask_size);
    mask_to_rgb256(&mask, mask_size)
}

/// Render an MDI PNG (RGBA + alpha) as 256×256 RGB for embedding index build.
pub fn mdi_png_to_rgb256(img: &DynamicImage, mask_size: u32) -> RgbImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut flat = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            flat.put_pixel(x, y, composite_icon_pixel(rgba.get_pixel(x, y)));
        }
    }

    let mut resized = if w == mask_size && h == mask_size {
        flat
    } else {
        image::imageops::resize(
            &flat,
            mask_size,
            mask_size,
            image::imageops::FilterType::Triangle,
        )
    };

    if mask_size != INPUT_SIZE {
        resized = image::imageops::resize(
            &resized,
            INPUT_SIZE,
            INPUT_SIZE,
            image::imageops::FilterType::Triangle,
        );
    }
    resized
}

fn composite_icon_pixel(pixel: &image::Rgba<u8>) -> Rgb<u8> {
    let alpha = pixel[3] as f32 / 255.0;
    let blend = |channel: u8| {
        (channel as f32 * alpha + 255.0 * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8
    };
    Rgb([blend(pixel[0]), blend(pixel[1]), blend(pixel[2])])
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

/// Convert RGB 256×256 to NCHW float tensor in [0, 1] (CLIP rescale, mean=0 std=1).
pub fn rgb256_to_nchw(rgb: &RgbImage) -> Vec<f32> {
    debug_assert_eq!(rgb.dimensions(), (INPUT_SIZE, INPUT_SIZE));
    let mut out = vec![0.0f32; 3 * INPUT_SIZE as usize * INPUT_SIZE as usize];
    let plane = (INPUT_SIZE * INPUT_SIZE) as usize;
    for y in 0..INPUT_SIZE {
        for x in 0..INPUT_SIZE {
            let pixel = rgb.get_pixel(x, y);
            let idx = (y * INPUT_SIZE + x) as usize;
            out[idx] = pixel[0] as f32 / 255.0;
            out[plane + idx] = pixel[1] as f32 / 255.0;
            out[2 * plane + idx] = pixel[2] as f32 / 255.0;
        }
    }
    out
}

/// L2-normalize a vector in place; returns the original L2 norm.
pub fn l2_normalize(v: &mut [f32]) -> f32 {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v {
            *x /= norm;
        }
    }
    norm
}

/// Cosine similarity between two L2-normalized vectors.
pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn rgb256_tensor_range() {
        let rgb = RgbImage::from_pixel(INPUT_SIZE, INPUT_SIZE, Rgb([128, 64, 32]));
        let tensor = rgb256_to_nchw(&rgb);
        assert!((tensor[0] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(tensor.len(), 3 * INPUT_SIZE as usize * INPUT_SIZE as usize);
    }

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
}
