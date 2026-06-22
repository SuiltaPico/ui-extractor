use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use image::RgbImage;
use ort::session::Session;
use ort::value::Tensor;

use crate::error::{ExtractError, Result};

use super::preprocess::{l2_normalize, EMBED_DIM, INPUT_SIZE};

const INDEX_MAGIC: &[u8; 4] = b"MCL2";
const INDEX_VERSION: u32 = 1;

/// MobileCLIP2-S0 vision encoder (ONNX).
pub struct IconEmbedder {
    session: Session,
    input_name: String,
    output_name: String,
}

impl IconEmbedder {
    pub fn load(model_path: &Path) -> Result<Self> {
        if !model_path.is_file() {
            return Err(ExtractError::Image(format!(
                "MobileCLIP2 vision model not found: {} (run scripts/download_mobileclip2.ps1)",
                model_path.display()
            )));
        }

        let session = Session::builder()
            .map_err(|e| ExtractError::Image(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let input_name = session
            .inputs()
            .first()
            .ok_or_else(|| ExtractError::Image("ONNX model has no inputs".into()))?
            .name()
            .to_string();
        let output_name = session
            .outputs()
            .first()
            .ok_or_else(|| ExtractError::Image("ONNX model has no outputs".into()))?
            .name()
            .to_string();

        Ok(Self {
            session,
            input_name,
            output_name,
        })
    }

    pub fn embed_rgb256(&mut self, rgb: &RgbImage) -> Result<Vec<f32>> {
        let tensor = super::preprocess::rgb256_to_nchw(rgb);
        self.embed_nchw(&tensor)
    }

    pub fn embed_nchw(&mut self, nchw: &[f32]) -> Result<Vec<f32>> {
        let expected = 3 * INPUT_SIZE as usize * INPUT_SIZE as usize;
        if nchw.len() != expected {
            return Err(ExtractError::Image(format!(
                "expected {expected} floats for NCHW input, got {}",
                nchw.len()
            )));
        }

        let input = Tensor::from_array((
            [1i64, 3, INPUT_SIZE as i64, INPUT_SIZE as i64],
            nchw.to_vec(),
        ))
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![self.input_name.as_str() => input])
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let (_shape, data) = outputs[self.output_name.as_str()]
            .try_extract_tensor::<f32>()
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        let mut embedding = data.to_vec();
        if embedding.len() > EMBED_DIM {
            embedding.truncate(EMBED_DIM);
        }
        if embedding.len() < EMBED_DIM {
            return Err(ExtractError::Image(format!(
                "embedding dim {} < expected {EMBED_DIM}",
                embedding.len()
            )));
        }
        l2_normalize(&mut embedding);
        Ok(embedding)
    }
}

/// Precomputed L2-normalized MDI template embeddings.
#[derive(Debug, Clone)]
pub struct EmbeddingIndex {
    pub dim: u32,
    pub names: Vec<String>,
    pub vectors: Vec<f32>,
}

impl EmbeddingIndex {
    pub fn count(&self) -> usize {
        self.names.len()
    }

    pub fn vector(&self, index: usize) -> &[f32] {
        let dim = self.dim as usize;
        let start = index * dim;
        &self.vectors[start..start + dim]
    }

    /// Brute-force cosine search (vectors are L2-normalized).
    pub fn best_match(&self, query: &[f32]) -> Option<(usize, f64)> {
        if query.len() != self.dim as usize || self.names.is_empty() {
            return None;
        }

        let mut best_idx = 0usize;
        let mut best_score = f64::NEG_INFINITY;
        for (i, _name) in self.names.iter().enumerate() {
            let score = super::preprocess::cosine(query, self.vector(i));
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }
        Some((best_idx, best_score))
    }

    /// Return top-k by cosine similarity (descending).
    pub fn top_k(&self, query: &[f32], k: usize) -> Vec<(usize, f64)> {
        if query.len() != self.dim as usize || self.names.is_empty() {
            return vec![];
        }

        let mut scores: Vec<(usize, f64)> = self
            .names
            .iter()
            .enumerate()
            .map(|(i, _)| (i, super::preprocess::cosine(query, self.vector(i))))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k.min(scores.len()));
        scores
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| ExtractError::Image(e.to_string()))?;
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        if &magic != INDEX_MAGIC {
            return Err(ExtractError::Image(format!(
                "invalid embedding index magic in {}",
                path.display()
            )));
        }

        let version = read_u32(&mut reader)?;
        if version != INDEX_VERSION {
            return Err(ExtractError::Image(format!(
                "unsupported embedding index version {version} in {}",
                path.display()
            )));
        }

        let dim = read_u32(&mut reader)?;
        let count = read_u32(&mut reader)?;
        if dim as usize != EMBED_DIM {
            return Err(ExtractError::Image(format!(
                "embedding dim {dim} != expected {EMBED_DIM}"
            )));
        }

        let mut names = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let name_len = read_u16(&mut reader)? as usize;
            let mut name_bytes = vec![0u8; name_len];
            reader
                .read_exact(&mut name_bytes)
                .map_err(|e| ExtractError::Image(e.to_string()))?;
            let name = String::from_utf8(name_bytes).map_err(|e| ExtractError::Image(e.to_string()))?;
            names.push(name);
        }

        let vector_count = count as usize * dim as usize;
        let mut vectors = vec![0f32; vector_count];
        reader
            .read_exact(bytemuck_cast_mut(&mut vectors))
            .map_err(|e| ExtractError::Image(e.to_string()))?;

        Ok(Self { dim, names, vectors })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ExtractError::Image(e.to_string()))?;
        }
        let mut file = File::create(path).map_err(|e| ExtractError::Image(e.to_string()))?;

        file.write_all(INDEX_MAGIC)
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        write_u32(&mut file, INDEX_VERSION)?;
        write_u32(&mut file, self.dim)?;
        write_u32(&mut file, self.names.len() as u32)?;

        for name in &self.names {
            let bytes = name.as_bytes();
            if bytes.len() > u16::MAX as usize {
                return Err(ExtractError::Image(format!("name too long: {name}")));
            }
            write_u16(&mut file, bytes.len() as u16)?;
            file.write_all(bytes)
                .map_err(|e| ExtractError::Image(e.to_string()))?;
        }

        file.write_all(bytemuck_cast(&self.vectors))
            .map_err(|e| ExtractError::Image(e.to_string()))?;
        Ok(())
    }
}

fn read_u16(reader: &mut impl Read) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader
        .read_exact(&mut buf)
        .map_err(|e| ExtractError::Image(e.to_string()))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(reader: &mut impl Read) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| ExtractError::Image(e.to_string()))?;
    Ok(u32::from_le_bytes(buf))
}

fn write_u16(writer: &mut impl Write, value: u16) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|e| ExtractError::Image(e.to_string()))
}

fn write_u32(writer: &mut impl Write, value: u32) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|e| ExtractError::Image(e.to_string()))
}

fn bytemuck_cast(slice: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * std::mem::size_of::<f32>(),
        )
    }
}

fn bytemuck_cast_mut(slice: &mut [f32]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            slice.as_mut_ptr() as *mut u8,
            slice.len() * std::mem::size_of::<f32>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_roundtrip() {
        let index = EmbeddingIndex {
            dim: EMBED_DIM as u32,
            names: vec!["home".into(), "menu".into()],
            vectors: {
                let mut v = vec![0f32; EMBED_DIM * 2];
                v[0] = 1.0;
                v[EMBED_DIM] = 1.0;
                v
            },
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("embeddings.bin");
        index.save(&path).unwrap();
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.names, index.names);
        assert_eq!(loaded.vectors, index.vectors);
    }

    #[test]
    fn best_match_picks_highest_cosine() {
        let mut a = vec![0f32; EMBED_DIM];
        a[0] = 1.0;
        let mut b = vec![0f32; EMBED_DIM];
        b[1] = 1.0;
        let query = a.clone();
        let index = EmbeddingIndex {
            dim: EMBED_DIM as u32,
            names: vec!["a".into(), "b".into()],
            vectors: [a, b].concat(),
        };
        let (idx, score) = index.best_match(&query).unwrap();
        assert_eq!(idx, 0);
        assert!((score - 1.0).abs() < 1e-6);
    }
}
