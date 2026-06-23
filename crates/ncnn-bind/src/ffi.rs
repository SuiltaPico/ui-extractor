//! Hand-written ncnn C API declarations used by ui-extractor.
//! Only the subset we call is listed here — no bindgen, no full `c_api.h`.

use std::os::raw::{c_char, c_int, c_void};

pub type NcnnMat = *mut c_void;
pub type NcnnNet = *mut c_void;
pub type NcnnExtractor = *mut c_void;
pub type NcnnAllocator = *mut c_void;

pub const NCNN_MAT_PIXEL_RGB: c_int = 1;
pub const NCNN_MAT_PIXEL_BGR: c_int = 2;
pub const NCNN_MAT_PIXEL_GRAY: c_int = 3;
pub const NCNN_MAT_PIXEL_RGBA: c_int = 4;
pub const NCNN_MAT_PIXEL_BGRA: c_int = 5;

extern "C" {
    pub fn ncnn_mat_create() -> NcnnMat;
    pub fn ncnn_mat_create_2d(w: c_int, h: c_int, allocator: NcnnAllocator) -> NcnnMat;
    pub fn ncnn_mat_create_external_3d(
        w: c_int,
        h: c_int,
        c: c_int,
        data: *mut c_void,
        allocator: NcnnAllocator,
    ) -> NcnnMat;
    pub fn ncnn_mat_destroy(mat: NcnnMat);
    pub fn ncnn_mat_fill_float(mat: NcnnMat, v: f32);
    pub fn ncnn_mat_get_w(mat: NcnnMat) -> c_int;
    pub fn ncnn_mat_get_h(mat: NcnnMat) -> c_int;
    pub fn ncnn_mat_get_d(mat: NcnnMat) -> c_int;
    pub fn ncnn_mat_get_c(mat: NcnnMat) -> c_int;
    pub fn ncnn_mat_get_elemsize(mat: NcnnMat) -> usize;
    pub fn ncnn_mat_get_data(mat: NcnnMat) -> *mut c_void;
    pub fn ncnn_mat_from_pixels(
        pixels: *const u8,
        pixel_type: c_int,
        w: c_int,
        h: c_int,
        stride: c_int,
        allocator: NcnnAllocator,
    ) -> NcnnMat;
    pub fn ncnn_mat_substract_mean_normalize(
        mat: NcnnMat,
        mean_vals: *const f32,
        norm_vals: *const f32,
    );

    pub fn ncnn_net_create() -> NcnnNet;
    pub fn ncnn_net_destroy(net: NcnnNet);
    pub fn ncnn_net_load_param(net: NcnnNet, path: *const c_char) -> c_int;
    pub fn ncnn_net_load_model(net: NcnnNet, path: *const c_char) -> c_int;

    pub fn ncnn_extractor_create(net: NcnnNet) -> NcnnExtractor;
    pub fn ncnn_extractor_destroy(ex: NcnnExtractor);
    pub fn ncnn_extractor_input(ex: NcnnExtractor, name: *const c_char, mat: NcnnMat) -> c_int;
    pub fn ncnn_extractor_extract(ex: NcnnExtractor, name: *const c_char, mat: *mut NcnnMat)
        -> c_int;
}
