pub mod preprocess;

pub const INPUT_SIZE: u32 = 256;
pub const EMBED_DIM: usize = 512;

use image::RgbImage;

use crate::infer::error::{InferError, Result};
use crate::infer::ffi;

/// Convert RGB 256×256 to NCHW float tensor in [0, 1].
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

pub fn l2_normalize(v: &mut [f32]) -> f32 {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v {
            *x /= norm;
        }
    }
    norm
}

pub fn finalize_embedding(mut embedding: Vec<f32>) -> Result<Vec<f32>> {
    if embedding.len() > EMBED_DIM {
        embedding.truncate(EMBED_DIM);
    }
    if embedding.len() < EMBED_DIM {
        return Err(InferError::Embed(format!(
            "embedding dim {} < expected {EMBED_DIM}",
            embedding.len()
        )));
    }
    l2_normalize(&mut embedding);
    Ok(embedding)
}

pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum()
}

pub struct EmbedEngine {
    pub(crate) handle: *mut std::ffi::c_void,
}

impl EmbedEngine {
    pub fn embed_rgb256(&mut self, rgb: &RgbImage) -> Result<Vec<f32>> {
        ffi::embed_rgb256(self.handle, rgb.as_raw())
    }

    pub fn embed_rgb256_batch(&mut self, images: &[RgbImage]) -> Result<Vec<Vec<f32>>> {
        let slices: Vec<&[u8]> = images.iter().map(|img| img.as_raw().as_slice()).collect();
        ffi::embed_rgb256_batch(self.handle, &slices)
    }
}

impl Drop for EmbedEngine {
    fn drop(&mut self) {
        let handle = self.handle;
        if handle.is_null() {
            return;
        }
        self.handle = std::ptr::null_mut();
        ffi::embed_engine_destroy(handle);
    }
}
