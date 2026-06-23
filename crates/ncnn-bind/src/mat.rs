use std::os::raw::c_void;

use crate::ffi::{
    self, NcnnMat, NCNN_MAT_PIXEL_BGR, NCNN_MAT_PIXEL_BGRA, NCNN_MAT_PIXEL_GRAY,
    NCNN_MAT_PIXEL_RGB, NCNN_MAT_PIXEL_RGBA,
};

const PIXEL_CONVERT_SHIFT: u32 = 16;

#[derive(Clone, Copy, Debug)]
pub enum MatPixelType {
    BGR,
    BGRA,
    Gray,
    RGB,
    RGBA,
    RgbToBgr,
    RgbToGray,
    RgbToRgba,
    RgbToBgra,
    BgrToRgb,
    BgrToGray,
    BgrToRgba,
    BgrToBgra,
    GrayToRgb,
    GrayToBgr,
    GrayToRgba,
    GrayToBgra,
    RgbaToRgb,
    RgbaToBgr,
    RgbaToGray,
    RgbaToBgra,
    BgraToRgb,
    BgraToBgr,
    BgraToGray,
    BgraToRgba,
}

impl MatPixelType {
    fn to_int(self) -> i32 {
        match self {
            Self::BGR => NCNN_MAT_PIXEL_BGR,
            Self::BGRA => NCNN_MAT_PIXEL_BGRA,
            Self::Gray => NCNN_MAT_PIXEL_GRAY,
            Self::RGB => NCNN_MAT_PIXEL_RGB,
            Self::RGBA => NCNN_MAT_PIXEL_RGBA,
            Self::RgbToBgr => NCNN_MAT_PIXEL_RGB | (NCNN_MAT_PIXEL_BGR << PIXEL_CONVERT_SHIFT),
            Self::RgbToGray => NCNN_MAT_PIXEL_RGB | (NCNN_MAT_PIXEL_GRAY << PIXEL_CONVERT_SHIFT),
            Self::RgbToRgba => NCNN_MAT_PIXEL_RGB | (NCNN_MAT_PIXEL_RGBA << PIXEL_CONVERT_SHIFT),
            Self::RgbToBgra => NCNN_MAT_PIXEL_RGB | (NCNN_MAT_PIXEL_BGRA << PIXEL_CONVERT_SHIFT),
            Self::BgrToRgb => NCNN_MAT_PIXEL_BGR | (NCNN_MAT_PIXEL_RGB << PIXEL_CONVERT_SHIFT),
            Self::BgrToGray => NCNN_MAT_PIXEL_BGR | (NCNN_MAT_PIXEL_GRAY << PIXEL_CONVERT_SHIFT),
            Self::BgrToRgba => NCNN_MAT_PIXEL_BGR | (NCNN_MAT_PIXEL_RGBA << PIXEL_CONVERT_SHIFT),
            Self::BgrToBgra => NCNN_MAT_PIXEL_BGR | (NCNN_MAT_PIXEL_BGRA << PIXEL_CONVERT_SHIFT),
            Self::GrayToRgb => NCNN_MAT_PIXEL_GRAY | (NCNN_MAT_PIXEL_RGB << PIXEL_CONVERT_SHIFT),
            Self::GrayToBgr => NCNN_MAT_PIXEL_GRAY | (NCNN_MAT_PIXEL_BGR << PIXEL_CONVERT_SHIFT),
            Self::GrayToRgba => NCNN_MAT_PIXEL_GRAY | (NCNN_MAT_PIXEL_RGBA << PIXEL_CONVERT_SHIFT),
            Self::GrayToBgra => NCNN_MAT_PIXEL_GRAY | (NCNN_MAT_PIXEL_BGRA << PIXEL_CONVERT_SHIFT),
            Self::RgbaToRgb => NCNN_MAT_PIXEL_RGBA | (NCNN_MAT_PIXEL_RGB << PIXEL_CONVERT_SHIFT),
            Self::RgbaToBgr => NCNN_MAT_PIXEL_RGBA | (NCNN_MAT_PIXEL_BGR << PIXEL_CONVERT_SHIFT),
            Self::RgbaToGray => NCNN_MAT_PIXEL_RGBA | (NCNN_MAT_PIXEL_GRAY << PIXEL_CONVERT_SHIFT),
            Self::RgbaToBgra => NCNN_MAT_PIXEL_RGBA | (NCNN_MAT_PIXEL_BGRA << PIXEL_CONVERT_SHIFT),
            Self::BgraToRgb => NCNN_MAT_PIXEL_BGRA | (NCNN_MAT_PIXEL_RGB << PIXEL_CONVERT_SHIFT),
            Self::BgraToBgr => NCNN_MAT_PIXEL_BGRA | (NCNN_MAT_PIXEL_BGR << PIXEL_CONVERT_SHIFT),
            Self::BgraToGray => NCNN_MAT_PIXEL_BGRA | (NCNN_MAT_PIXEL_GRAY << PIXEL_CONVERT_SHIFT),
            Self::BgraToRgba => NCNN_MAT_PIXEL_BGRA | (NCNN_MAT_PIXEL_RGBA << PIXEL_CONVERT_SHIFT),
        }
    }

    fn stride(self) -> i32 {
        match self {
            Self::BGR | Self::BgrToBgra | Self::BgrToGray | Self::BgrToRgb | Self::BgrToRgba => 3,
            Self::BGRA
            | Self::BgraToBgr
            | Self::BgraToGray
            | Self::BgraToRgb
            | Self::BgraToRgba => 4,
            Self::Gray
            | Self::GrayToBgr
            | Self::GrayToBgra
            | Self::GrayToRgb
            | Self::GrayToRgba => 1,
            Self::RGB | Self::RgbToBgr | Self::RgbToBgra | Self::RgbToGray | Self::RgbToRgba => 3,
            Self::RGBA
            | Self::RgbaToBgr
            | Self::RgbaToBgra
            | Self::RgbaToGray
            | Self::RgbaToRgb => 4,
        }
    }
}

pub struct Mat {
    ptr: NcnnMat,
}

unsafe impl Send for Mat {}

impl Mat {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { ffi::ncnn_mat_create() },
        }
    }

    pub fn new_2d(w: i32, h: i32, _alloc: Option<&()>) -> Self {
        Self {
            ptr: unsafe { ffi::ncnn_mat_create_2d(w, h, std::ptr::null_mut()) },
        }
    }

    /// # Safety
    /// `data` must remain valid and unaliased for the lifetime of this `Mat`.
    pub unsafe fn new_external_3d(
        w: i32,
        h: i32,
        c: i32,
        data: *mut c_void,
        _alloc: Option<&()>,
    ) -> Self {
        Self {
            ptr: ffi::ncnn_mat_create_external_3d(w, h, c, data, std::ptr::null_mut()),
        }
    }

    pub fn from_pixels(
        data: &[u8],
        pixel_type: MatPixelType,
        width: i32,
        height: i32,
        _alloc: Option<&()>,
    ) -> anyhow::Result<Self> {
        let len = width * height * pixel_type.stride();
        if data.len() != len as usize {
            anyhow::bail!("Expected data length {len}, provided {}", data.len());
        }

        Ok(Self {
            ptr: unsafe {
                ffi::ncnn_mat_from_pixels(
                    data.as_ptr(),
                    pixel_type.to_int(),
                    width,
                    height,
                    width * pixel_type.stride(),
                    std::ptr::null_mut(),
                )
            },
        })
    }

    pub fn substract_mean_normalize(&mut self, mean_vals: &[f32], norm_vals: &[f32]) {
        let channels = self.c() as usize;
        assert_eq!(mean_vals.len(), channels);
        assert_eq!(norm_vals.len(), channels);
        unsafe {
            ffi::ncnn_mat_substract_mean_normalize(
                self.ptr,
                mean_vals.as_ptr(),
                norm_vals.as_ptr(),
            );
        }
    }

    pub fn fill(&mut self, value: f32) {
        unsafe { ffi::ncnn_mat_fill_float(self.ptr, value) };
    }

    pub fn w(&self) -> i32 {
        unsafe { ffi::ncnn_mat_get_w(self.ptr) }
    }

    pub fn h(&self) -> i32 {
        unsafe { ffi::ncnn_mat_get_h(self.ptr) }
    }

    pub fn d(&self) -> i32 {
        unsafe { ffi::ncnn_mat_get_d(self.ptr) }
    }

    pub fn c(&self) -> i32 {
        unsafe { ffi::ncnn_mat_get_c(self.ptr) }
    }

    pub fn elemsize(&self) -> u64 {
        unsafe { ffi::ncnn_mat_get_elemsize(self.ptr) as u64 }
    }

    pub fn data(&self) -> *mut c_void {
        unsafe { ffi::ncnn_mat_get_data(self.ptr) }
    }

    pub(crate) fn ptr(&self) -> NcnnMat {
        self.ptr
    }

    pub(crate) fn mut_ptr(&mut self) -> *mut NcnnMat {
        &mut self.ptr
    }
}

impl Default for Mat {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Mat {
    fn drop(&mut self) {
        unsafe { ffi::ncnn_mat_destroy(self.ptr) };
    }
}