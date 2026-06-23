use std::path::Path;

use crate::error::Result;

use super::embedding::EmbeddingIndex;

/// Loaded embedding index for cosine icon matching.
#[derive(Debug)]
pub struct IconLibrary {
    pub embeddings: EmbeddingIndex,
}

impl IconLibrary {
    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self {
            embeddings: EmbeddingIndex::load(path)?,
        })
    }

    pub fn from_index(embeddings: EmbeddingIndex) -> Self {
        Self { embeddings }
    }

    /// Best cosine match above `min_cosine`.
    pub fn best_match(
        &self,
        query_embedding: &[f32],
        min_cosine: f64,
    ) -> Option<(String, f64)> {
        let (idx, cosine) = self.embeddings.best_match(query_embedding)?;
        if cosine < min_cosine {
            return None;
        }
        Some((self.embeddings.names[idx].clone(), cosine))
    }
}
