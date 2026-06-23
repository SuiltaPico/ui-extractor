use std::ffi::CString;
use std::marker::PhantomData;

use crate::ffi::{self, NcnnExtractor};
use crate::Mat;

pub struct Extractor<'a> {
    ptr: NcnnExtractor,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Extractor<'a> {
    pub(crate) fn from_ptr(ptr: NcnnExtractor) -> Self {
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn input(&mut self, name: &str, mat: &Mat) -> anyhow::Result<()> {
        let c_str = CString::new(name)?;
        if unsafe { ffi::ncnn_extractor_input(self.ptr, c_str.as_ptr(), mat.ptr()) } != 0 {
            anyhow::bail!("Error setting input for layer `{name}`");
        }
        Ok(())
    }

    pub fn extract(&mut self, name: &str, mat: &mut Mat) -> anyhow::Result<()> {
        let c_str = CString::new(name)?;
        if unsafe { ffi::ncnn_extractor_extract(self.ptr, c_str.as_ptr(), mat.mut_ptr()) } != 0 {
            anyhow::bail!("Error running extract on layer `{name}`");
        }
        Ok(())
    }
}

impl Drop for Extractor<'_> {
    fn drop(&mut self) {
        unsafe { ffi::ncnn_extractor_destroy(self.ptr) };
    }
}
