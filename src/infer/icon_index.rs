use crate::infer::error::{InferError, Result};
use crate::infer::ffi;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IconMatch {
    pub name: String,
    pub score: f64,
}

/// Icon index loaded via infer-core FFI (`infer_icon_index_*`).
#[derive(Debug)]
pub struct IconIndex {
    pub(crate) handle: *mut std::ffi::c_void,
}

impl IconIndex {
    pub fn match_embedding(&self, query: &[f32], min_cosine: f64) -> Result<Option<IconMatch>> {
        let json = ffi::icon_index_match_embedding(self.handle, query, min_cosine as f32)?;
        parse_match_json(&json)
    }

    pub fn match_embeddings_batch(
        &self,
        queries: &[&[f32]],
        min_cosine: f64,
    ) -> Result<Vec<Option<IconMatch>>> {
        if queries.is_empty() {
            return Ok(vec![]);
        }
        let dim = queries[0].len();
        if dim == 0 {
            return Err(InferError::IconIndex("empty embedding".into()));
        }
        if queries.iter().any(|q| q.len() != dim) {
            return Err(InferError::IconIndex(
                "batch embeddings have inconsistent dims".into(),
            ));
        }
        let flat: Vec<f32> = queries.iter().flat_map(|q| q.iter().copied()).collect();
        let json = ffi::icon_index_match_embeddings_batch(
            self.handle,
            &flat,
            dim,
            min_cosine as f32,
        )?;
        parse_match_batch_json(&json)
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<IconMatch>> {
        let json = ffi::icon_index_search(self.handle, query, top_k.max(1))?;
        parse_search_json(&json)
    }
}

impl Drop for IconIndex {
    fn drop(&mut self) {
        ffi::icon_index_destroy(self.handle);
    }
}

fn parse_match_json(json: &str) -> Result<Option<IconMatch>> {
    let trimmed = json.trim();
    if trimmed == "null" {
        return Ok(None);
    }
    let hit: IconMatch = serde_json::from_str(trimmed)?;
    Ok(Some(hit))
}

fn parse_match_batch_json(json: &str) -> Result<Vec<Option<IconMatch>>> {
    let hits: Vec<Option<IconMatch>> = serde_json::from_str(json)?;
    Ok(hits)
}

fn parse_search_json(json: &str) -> Result<Vec<IconMatch>> {
    serde_json::from_str(json).map_err(InferError::from)
}
