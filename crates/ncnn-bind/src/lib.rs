//! Minimal ncnn C API bindings for ui-extractor.
//!
//! Hand-written FFI for the ~20 functions we use — no bindgen, no vendored ncnnrs.

mod extractor;
mod ffi;
mod mat;
mod net;

pub use extractor::Extractor;
pub use mat::{Mat, MatPixelType};
pub use net::Net;
