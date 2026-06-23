use std::ffi::CString;

use crate::extractor::Extractor;
use crate::ffi::{self, NcnnNet};

pub struct Net {
    ptr: NcnnNet,
}

unsafe impl Send for Net {}
unsafe impl Sync for Net {}

impl Net {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { ffi::ncnn_net_create() },
        }
    }

    pub fn load_param(&mut self, path: &str) -> anyhow::Result<()> {
        let c_str = CString::new(path)?;
        if unsafe { ffi::ncnn_net_load_param(self.ptr, c_str.as_ptr()) } != 0 {
            anyhow::bail!("Error loading params {path}");
        }
        Ok(())
    }

    pub fn load_model(&mut self, path: &str) -> anyhow::Result<()> {
        let c_str = CString::new(path)?;
        if unsafe { ffi::ncnn_net_load_model(self.ptr, c_str.as_ptr()) } != 0 {
            anyhow::bail!("Error loading model {path}");
        }
        Ok(())
    }

    pub fn create_extractor(&self) -> Extractor<'_> {
        let ptr = unsafe { ffi::ncnn_extractor_create(self.ptr) };
        Extractor::from_ptr(ptr)
    }
}

impl Drop for Net {
    fn drop(&mut self) {
        unsafe { ffi::ncnn_net_destroy(self.ptr) };
    }
}
