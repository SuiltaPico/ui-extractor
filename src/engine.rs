use std::path::Path;

use image::DynamicImage;
use crate::infer::{OcrEngine, Registry};

use crate::{
    error::Result,
    icon::{attach_icons_with_pack, IconPack},
    pipeline::{extract_from_image_timed_with_engine, ExtractConfig, ExtractTimings},
    types::ExtractResult,
};

/// Stateful extractor: loads models once via infer-core registry, reuses across extractions.
pub struct ExtractEngine {
    registry: Registry,
    config: ExtractConfig,
    ocr: Option<OcrEngine>,
    icon_pack: Option<IconPack>,
}

impl ExtractEngine {
    /// `registry == None` → open an owned registry; `Some` → borrow external handle.
    pub fn new(config: ExtractConfig, registry: Option<Registry>) -> Result<Self> {
        let registry = match registry {
            Some(reg) => reg,
            None => Registry::open(&config.models_dir, config.runtime.clone())
                .map_err(|e| crate::error::ExtractError::Ocr(e.to_string()))?,
        };

        let mut engine = Self {
            registry,
            config: config.clone(),
            ocr: None,
            icon_pack: None,
        };
        if config.run_icon {
            engine.reload_icon_pack()?;
        }
        Ok(engine)
    }

    /// Standalone mode: create and own an infer-core registry.
    pub fn open(config: ExtractConfig) -> Result<Self> {
        Self::new(config, None)
    }

    /// Use an existing infer-core registry (owned or borrowed).
    pub fn from_registry(registry: Registry, config: ExtractConfig) -> Result<Self> {
        Self::new(config, Some(registry))
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn config(&self) -> &ExtractConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ExtractConfig {
        &mut self.config
    }

    pub fn icon_pack(&self) -> Option<&IconPack> {
        self.icon_pack.as_ref()
    }

    pub fn icon_pack_mut(&mut self) -> Option<&mut IconPack> {
        self.icon_pack.as_mut()
    }

    /// Reload icon index + embed pack from registry.
    pub fn reload_icon_pack(&mut self) -> Result<()> {
        if !self.config.run_icon {
            self.icon_pack = None;
            return Ok(());
        }

        let icon = &self.config.icon;
        self.icon_pack = Some(IconPack::from_registry(
            &self.registry,
            &self.config.icon_index_pack,
            icon.template_size,
            icon.clone(),
        )?);
        Ok(())
    }

    pub fn extract_from_path(
        &mut self,
        path: &Path,
    ) -> Result<(ExtractResult, ExtractTimings)> {
        let img = image::open(path)
            .map_err(|_| crate::error::ExtractError::ImageRead(path.display().to_string()))?;
        self.extract_from_image(&img)
    }

    pub fn extract_from_bytes(&mut self, bytes: &[u8]) -> Result<(ExtractResult, ExtractTimings)> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| crate::error::ExtractError::Image(e.to_string()))?;
        self.extract_from_image(&img)
    }

    pub fn extract_from_image(
        &mut self,
        img: &DynamicImage,
    ) -> Result<(ExtractResult, ExtractTimings)> {
        let mut pipeline_config = self.config.clone();
        pipeline_config.run_icon = false;

        if pipeline_config.run_ocr && self.ocr.is_none() {
            self.ocr = Some(crate::ocr::load_ocr_engine(
                &self.registry,
                &self.config.ocr_pack,
                &self.config.ocr,
            )?);
        }

        let registry = &self.registry;
        let ocr_ref = self.ocr.as_ref();

        let (mut result, mut timings) = extract_from_image_timed_with_engine(
            img,
            &pipeline_config,
            registry,
            ocr_ref,
            None,
        )?;

        if self.config.run_icon {
            if let Some(pack) = self.icon_pack.as_mut() {
                let stats = attach_icons_with_pack(
                    &mut result.root,
                    img,
                    pack,
                    &pack.match_config(),
                );
                timings.icon = stats;
            }
        }

        Ok((result, timings))
    }
}
