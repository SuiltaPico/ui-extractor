use std::path::{Path, PathBuf};

use ncnn_bind::{Mat, Net};

use crate::error::{ExtractError, Result};

/// Resolve `.ncnn.bin` from a `.ncnn.param` path (pnnx naming).
pub fn bin_path(param_path: &Path) -> PathBuf {
    let name = param_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if let Some(stem) = name.strip_suffix(".ncnn.param") {
        param_path.with_file_name(format!("{stem}.ncnn.bin"))
    } else if let Some(stem) = name.strip_suffix(".param") {
        param_path.with_file_name(format!("{stem}.bin"))
    } else {
        param_path.with_extension("bin")
    }
}

pub fn load_net(param_path: &Path) -> Result<Net> {
    if !param_path.is_file() {
        return Err(ExtractError::Image(format!(
            "ncnn param not found: {}",
            param_path.display()
        )));
    }
    let bin_path = bin_path(param_path);
    if !bin_path.is_file() {
        return Err(ExtractError::Image(format!(
            "ncnn bin not found: {}",
            bin_path.display()
        )));
    }

    let mut net = Net::new();
    net.load_param(param_path.to_str().unwrap_or(""))
        .map_err(|e| ExtractError::Image(format!("ncnn load_param: {e}")))?;
    net.load_model(bin_path.to_str().unwrap_or(""))
        .map_err(|e| ExtractError::Image(format!("ncnn load_model: {e}")))?;
    Ok(net)
}

/// Read float tensor data from an ncnn Mat (elemsize must be 4).
pub fn mat_to_vec_f32(mat: &Mat) -> Result<Vec<f32>> {
    if mat.elemsize() != 4 {
        return Err(ExtractError::Image(format!(
            "expected f32 ncnn mat, elemsize={}",
            mat.elemsize()
        )));
    }
    let count = (mat.w() * mat.h() * mat.d().max(1) * mat.c()) as usize;
    if count == 0 {
        return Ok(vec![]);
    }
    let ptr = mat.data() as *const f32;
    Ok(unsafe { std::slice::from_raw_parts(ptr, count).to_vec() })
}

/// NCHW `[3,H,W]` in [0,1] → HWC float vec for ncnn `Mat::new_external_3d(w,h,3)`.
pub fn nchw01_to_hwc(nchw: &[f32], width: u32, height: u32) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let mut hwc = vec![0f32; 3 * w * h];
    for c in 0..3 {
        for y in 0..h {
            for x in 0..w {
                hwc[(y * w + x) * 3 + c] = nchw[c * h * w + y * w + x];
            }
        }
    }
    hwc
}

/// Build 3-channel ncnn Mat from HWC float data.
pub fn mat_from_hwc(hwc: &[f32], width: i32, height: i32) -> Result<Mat> {
    let expected = (width * height * 3) as usize;
    if hwc.len() != expected {
        return Err(ExtractError::Image(format!(
            "expected {expected} HWC floats, got {}",
            hwc.len()
        )));
    }
    let mat = unsafe {
        Mat::new_external_3d(
            width,
            height,
            3,
            hwc.as_ptr() as *mut _,
            None,
        )
    };
    Ok(mat)
}
