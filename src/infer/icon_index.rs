use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use crate::infer::embed::{l2_normalize, EMBED_DIM};
use crate::infer::error::{InferError, Result};
use crate::infer::manifest::Manifest;

const INDEX_MAGIC: &[u8; 4] = b"MCL2";
const INDEX_VERSION_F32: u32 = 1;
const INDEX_VERSION_I8: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStorageFormat {
    F32,
    Int8,
}

impl IndexStorageFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "f32" | "fp32" => Ok(Self::F32),
            "int8" | "i8" => Ok(Self::Int8),
            other => Err(InferError::IconIndex(format!(
                "unsupported index format: {other} (expected f32 or int8)"
            ))),
        }
    }

    pub fn index_format_label(self) -> &'static str {
        match self {
            Self::F32 => "mcl2-v1",
            Self::Int8 => "mcl2-v2",
        }
    }
}

/// Precomputed L2-normalized icon template embeddings (stored as int8 + per-vector scale).
#[derive(Debug, Clone)]
pub struct EmbeddingIndex {
    pub dim: u32,
    pub names: Vec<String>,
    codes: Vec<i8>,
    scales: Vec<f32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IconMatch {
    pub name: String,
    pub score: f64,
}

/// Loaded icon index for cosine matching.
#[derive(Debug)]
pub struct IconIndex {
    index: EmbeddingIndex,
    path: PathBuf,
}

impl IconIndex {
    pub fn from_manifest(pack_dir: &Path, manifest: &Manifest) -> Result<Self> {
        let path = manifest.file_path(pack_dir, "index")?;
        let index = EmbeddingIndex::load(&path)?;
        Ok(Self { index, path })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let index = EmbeddingIndex::load(path)?;
        Ok(Self {
            index,
            path: path.to_path_buf(),
        })
    }

    pub fn embedding_index(&self) -> &EmbeddingIndex {
        &self.index
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn match_embedding(&self, query: &[f32], min_cosine: f64) -> Option<IconMatch> {
        let (idx, score) = self.index.best_match(query)?;
        if score < min_cosine {
            return None;
        }
        Some(IconMatch {
            name: self.index.names[idx].clone(),
            score,
        })
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<IconMatch> {
        self.index
            .top_k(query, top_k.max(1))
            .into_iter()
            .map(|(idx, score)| IconMatch {
                name: self.index.names[idx].clone(),
                score,
            })
            .collect()
    }
}

impl EmbeddingIndex {
    pub fn count(&self) -> usize {
        self.names.len()
    }

    pub fn from_float_vectors(dim: u32, names: Vec<String>, vectors: Vec<f32>) -> Result<Self> {
        let dim_usize = dim as usize;
        if names.is_empty() {
            return Err(InferError::IconIndex(
                "embedding index has no vectors".into(),
            ));
        }
        if vectors.len() != names.len() * dim_usize {
            return Err(InferError::IconIndex(format!(
                "vector byte count {} != {} names × dim {dim_usize}",
                vectors.len(),
                names.len()
            )));
        }

        let mut codes = Vec::with_capacity(vectors.len());
        let mut scales = Vec::with_capacity(names.len());
        for chunk in vectors.chunks(dim_usize) {
            let (chunk_codes, scale) = quantize_unit_vector(chunk);
            codes.extend(chunk_codes);
            scales.push(scale);
        }

        Ok(Self {
            dim,
            names,
            codes,
            scales,
        })
    }

    pub fn vector_f32(&self, index: usize) -> Vec<f32> {
        let dim = self.dim as usize;
        let start = index * dim;
        let mut out = dequantize_vector(&self.codes[start..start + dim], self.scales[index]);
        l2_normalize(&mut out);
        out
    }

    pub fn best_match(&self, query: &[f32]) -> Option<(usize, f64)> {
        if query.len() != self.dim as usize || self.names.is_empty() {
            return None;
        }

        let mut best_idx = 0usize;
        let mut best_score = f64::NEG_INFINITY;
        let dim = self.dim as usize;
        for (i, _name) in self.names.iter().enumerate() {
            let start = i * dim;
            let score = cosine_query_i8(query, &self.codes[start..start + dim], self.scales[i]);
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }
        Some((best_idx, best_score))
    }

    pub fn top_k(&self, query: &[f32], k: usize) -> Vec<(usize, f64)> {
        if query.len() != self.dim as usize || self.names.is_empty() {
            return vec![];
        }

        let dim = self.dim as usize;
        let mut scores: Vec<(usize, f64)> = self
            .names
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let start = i * dim;
                (
                    i,
                    cosine_query_i8(query, &self.codes[start..start + dim], self.scales[i]),
                )
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k.min(scores.len()));
        scores
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| InferError::IconIndex(e.to_string()))?;
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|e| InferError::IconIndex(e.to_string()))?;
        if &magic != INDEX_MAGIC {
            return Err(InferError::IconIndex(format!(
                "invalid embedding index magic in {}",
                path.display()
            )));
        }

        let version = read_u32(&mut reader)?;
        let dim = read_u32(&mut reader)?;
        let count = read_u32(&mut reader)?;
        if dim as usize != EMBED_DIM {
            return Err(InferError::IconIndex(format!(
                "embedding dim {dim} != expected {EMBED_DIM}"
            )));
        }

        let mut names = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let name_len = read_u16(&mut reader)? as usize;
            let mut name_bytes = vec![0u8; name_len];
            reader
                .read_exact(&mut name_bytes)
                .map_err(|e| InferError::IconIndex(e.to_string()))?;
            let name =
                String::from_utf8(name_bytes).map_err(|e| InferError::IconIndex(e.to_string()))?;
            names.push(name);
        }

        match version {
            INDEX_VERSION_F32 => {
                let vector_count = count as usize * dim as usize;
                let mut vectors = vec![0f32; vector_count];
                reader
                    .read_exact(bytemuck_cast_mut(&mut vectors))
                    .map_err(|e| InferError::IconIndex(e.to_string()))?;
                Self::from_float_vectors(dim, names, vectors)
            }
            INDEX_VERSION_I8 => {
                let dim_usize = dim as usize;
                let mut codes = vec![0i8; count as usize * dim_usize];
                let mut scales = vec![0f32; count as usize];
                for scale in &mut scales {
                    *scale = read_f32(&mut reader)?;
                }
                reader
                    .read_exact(bytemuck_cast_i8_mut(&mut codes))
                    .map_err(|e| InferError::IconIndex(e.to_string()))?;
                Ok(Self {
                    dim,
                    names,
                    codes,
                    scales,
                })
            }
            other => Err(InferError::IconIndex(format!(
                "unsupported embedding index version {other} in {}",
                path.display()
            ))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.save_as(path, IndexStorageFormat::Int8)
    }

    pub fn save_as(&self, path: &Path, format: IndexStorageFormat) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path).map_err(InferError::Io)?;

        let version = match format {
            IndexStorageFormat::Int8 => INDEX_VERSION_I8,
            IndexStorageFormat::F32 => INDEX_VERSION_F32,
        };

        file.write_all(INDEX_MAGIC).map_err(InferError::Io)?;
        write_u32(&mut file, version)?;
        write_u32(&mut file, self.dim)?;
        write_u32(&mut file, self.names.len() as u32)?;

        for name in &self.names {
            let bytes = name.as_bytes();
            if bytes.len() > u16::MAX as usize {
                return Err(InferError::IconIndex(format!("name too long: {name}")));
            }
            write_u16(&mut file, bytes.len() as u16)?;
            file.write_all(bytes).map_err(InferError::Io)?;
        }

        match format {
            IndexStorageFormat::Int8 => {
                for &scale in &self.scales {
                    write_f32(&mut file, scale)?;
                }
                file.write_all(bytemuck_cast_i8(&self.codes))
                    .map_err(InferError::Io)?;
            }
            IndexStorageFormat::F32 => {
                let mut vectors = Vec::with_capacity(self.names.len() * self.dim as usize);
                for i in 0..self.names.len() {
                    vectors.extend(self.vector_f32(i));
                }
                file.write_all(bytemuck_cast_mut(&mut vectors))
                    .map_err(InferError::Io)?;
            }
        }
        Ok(())
    }
}

fn quantize_unit_vector(v: &[f32]) -> (Vec<i8>, f32) {
    let scale = v
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, f32::max)
        .max(f32::EPSILON);
    let codes = v
        .iter()
        .map(|x| (x / scale * 127.0).round().clamp(-127.0, 127.0) as i8)
        .collect();
    (codes, scale)
}

fn dequantize_vector(codes: &[i8], scale: f32) -> Vec<f32> {
    let inv = scale / 127.0;
    codes.iter().map(|&c| c as f32 * inv).collect()
}

fn cosine_query_i8(query: &[f32], codes: &[i8], scale: f32) -> f64 {
    let inv = scale as f64 / 127.0;
    let mut dot = 0.0f64;
    let mut norm_sq = 0.0f64;
    for (&q, &c) in query.iter().zip(codes.iter()) {
        let v = c as f64 * inv;
        dot += q as f64 * v;
        norm_sq += v * v;
    }
    if norm_sq <= 1e-20 {
        return 0.0;
    }
    dot / norm_sq.sqrt()
}

fn read_u16(reader: &mut impl Read) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader
        .read_exact(&mut buf)
        .map_err(|e| InferError::IconIndex(e.to_string()))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(reader: &mut impl Read) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| InferError::IconIndex(e.to_string()))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32(reader: &mut impl Read) -> Result<f32> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| InferError::IconIndex(e.to_string()))?;
    Ok(f32::from_le_bytes(buf))
}

fn write_u16(writer: &mut impl Write, value: u16) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(InferError::Io)
}

fn write_u32(writer: &mut impl Write, value: u32) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(InferError::Io)
}

fn write_f32(writer: &mut impl Write, value: f32) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(InferError::Io)
}

fn bytemuck_cast_mut(slice: &mut [f32]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            slice.as_mut_ptr() as *mut u8,
            slice.len() * std::mem::size_of::<f32>(),
        )
    }
}

fn bytemuck_cast_i8(slice: &[i8]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * std::mem::size_of::<i8>(),
        )
    }
}

fn bytemuck_cast_i8_mut(slice: &mut [i8]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            slice.as_mut_ptr() as *mut u8,
            slice.len() * std::mem::size_of::<i8>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::embed::cosine;

    fn unit_vector(seed: f32) -> Vec<f32> {
        let mut v: Vec<f32> = (0..EMBED_DIM)
            .map(|i| ((i as f32 + 1.0) * seed).sin())
            .collect();
        l2_normalize(&mut v);
        v
    }

    #[test]
    fn index_roundtrip_f32() {
        let a = unit_vector(0.3);
        let b = unit_vector(0.7);
        let index = EmbeddingIndex::from_float_vectors(
            EMBED_DIM as u32,
            vec!["home".into(), "menu".into()],
            [a, b].concat(),
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("embeddings.bin");
        index.save_as(&path, IndexStorageFormat::F32).unwrap();
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.names, index.names);
        let (idx, score) = loaded.best_match(&index.vector_f32(0)).unwrap();
        assert_eq!(idx, 0);
        assert!(score > 0.99);
    }

    #[test]
    fn index_roundtrip_i8() {
        let a = unit_vector(0.3);
        let b = unit_vector(0.7);
        let index = EmbeddingIndex::from_float_vectors(
            EMBED_DIM as u32,
            vec!["home".into(), "menu".into()],
            [a, b].concat(),
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("embeddings.bin");
        index.save(&path).unwrap();
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.names, index.names);
        assert_eq!(loaded.scales, index.scales);
        assert_eq!(loaded.codes, index.codes);
    }

    #[test]
    fn best_match_picks_highest_cosine() {
        let mut a = vec![0f32; EMBED_DIM];
        a[0] = 1.0;
        let mut b = vec![0f32; EMBED_DIM];
        b[1] = 1.0;
        let query = a.clone();
        let index = EmbeddingIndex::from_float_vectors(
            EMBED_DIM as u32,
            vec!["a".into(), "b".into()],
            [a, b].concat(),
        )
        .unwrap();
        let (idx, score) = index.best_match(&query).unwrap();
        assert_eq!(idx, 0);
        assert!(score > 0.99);
    }

    #[test]
    fn int8_preserves_top1_on_random_unit_vectors() {
        let count = 256;
        let mut names = Vec::with_capacity(count);
        let mut vectors = Vec::with_capacity(count * EMBED_DIM);
        for i in 0..count {
            names.push(format!("icon-{i}"));
            vectors.extend(unit_vector(i as f32 * 0.17 + 0.01));
        }

        let query = unit_vector(42.0);
        let f32_scores: Vec<f64> = vectors
            .chunks(EMBED_DIM)
            .map(|v| cosine(&query, v))
            .collect();
        let f32_best = f32_scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let index = EmbeddingIndex::from_float_vectors(EMBED_DIM as u32, names, vectors).unwrap();
        let (i8_best, _) = index.best_match(&query).unwrap();
        assert_eq!(f32_best, i8_best, "int8 changed top-1 match");
    }
}
