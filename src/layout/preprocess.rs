use image::GrayImage;
use imageproc::distance_transform::Norm;
use imageproc::morphology::{dilate, erode};

/// Gaussian blur before edge detection.
pub fn blur_for_edges(src: &GrayImage) -> GrayImage {
    gaussian_blur_3x3(src)
}

/// Canny edge map (not yet thickened).
pub fn canny_edges(blurred: &GrayImage) -> GrayImage {
    imageproc::edges::canny(blurred, 50.0, 100.0)
}

/// Thicken edge pixels so contour tracing can close gaps.
pub fn dilate_edges(edges: &GrayImage, radius: u8) -> GrayImage {
    dilate(edges, Norm::LInf, radius)
}

/// Grayscale + Canny edge detection + Dilation tuned for UI borders.
#[allow(dead_code)]
pub fn preprocess_for_contours(src: &GrayImage) -> GrayImage {
    let blurred = blur_for_edges(src);
    let edges = canny_edges(&blurred);
    dilate_edges(&edges, 1)
}

/// Morphological close to connect broken UI borders.
pub fn close_gaps(src: &GrayImage, kernel_size: u32) -> GrayImage {
    if kernel_size <= 1 {
        return src.clone();
    }
    let k = kernel_size as u8;
    erode(&dilate(src, Norm::LInf, k), Norm::LInf, k)
}

fn gaussian_blur_3x3(src: &GrayImage) -> GrayImage {
    let kernel: &[f32; 9] = &[
        1.0 / 16.0,
        2.0 / 16.0,
        1.0 / 16.0,
        2.0 / 16.0,
        4.0 / 16.0,
        2.0 / 16.0,
        1.0 / 16.0,
        2.0 / 16.0,
        1.0 / 16.0,
    ];
    imageproc::filter::filter3x3(src, kernel)
}

pub fn to_gray(src: &image::DynamicImage) -> GrayImage {
    src.to_luma8()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn preprocess_keeps_dimensions() {
        let img = GrayImage::from_pixel(32, 32, Luma([128]));
        let out = preprocess_for_contours(&img);
        assert_eq!(out.dimensions(), (32, 32));
    }
}
