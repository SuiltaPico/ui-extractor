use std::path::Path;

use image::DynamicImage;

use crate::{
    error::Result,
    icon::{attach_icons_with_pack, IconPack},
    pipeline::{extract_from_image_timed, ExtractConfig, ExtractTimings},
    types::ExtractResult,
};

/// Stateful extractor: loads icon models once, reuses them across extractions.
pub struct ExtractEngine {
    config: ExtractConfig,
    icon_pack: Option<IconPack>,
}

impl ExtractEngine {
    /// Create an engine and eagerly load icon resources when `config.run_icon` is true.
    pub fn new(config: ExtractConfig) -> Result<Self> {
        let mut engine = Self {
            config: config.clone(),
            icon_pack: None,
        };
        if config.run_icon {
            engine.reload_icon_pack()?;
        }
        Ok(engine)
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

    /// Reload icon embeddings from paths in `config.icon`.
    pub fn reload_icon_pack(&mut self) -> Result<()> {
        if !self.config.run_icon {
            self.icon_pack = None;
            return Ok(());
        }

        let icon = &self.config.icon;
        self.icon_pack = Some(IconPack::load(
            &icon.embedding_index,
            &icon.vision_model,
            icon.template_size,
            icon.into(),
        )?);
        Ok(())
    }

    pub fn extract_from_path(
        &mut self,
        path: &Path,
    ) -> Result<(ExtractResult, ExtractTimings)> {
        let img =
            image::open(path).map_err(|_| crate::error::ExtractError::ImageRead(path.display().to_string()))?;
        self.extract_from_image(&img)
    }

    pub fn extract_from_bytes(&mut self, bytes: &[u8]) -> Result<(ExtractResult, ExtractTimings)> {
        let img = image::load_from_memory(bytes).map_err(|e| crate::error::ExtractError::Image(e.to_string()))?;
        self.extract_from_image(&img)
    }

    pub fn extract_from_image(
        &mut self,
        img: &DynamicImage,
    ) -> Result<(ExtractResult, ExtractTimings)> {
        let mut pipeline_config = self.config.clone();
        pipeline_config.run_icon = false;
        let (mut result, mut timings) = extract_from_image_timed(img, &pipeline_config)?;

        if self.config.run_icon {
            if let Some(pack) = self.icon_pack.as_mut() {
                let stats = attach_icons_with_pack(
                    &mut result.root,
                    img,
                    pack,
                    &self.config.icon,
                );
                timings.icon = stats;
            }
        }

        Ok((result, timings))
    }
}
