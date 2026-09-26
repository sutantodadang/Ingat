use sha2::{Digest, Sha256};

use crate::{
    application::services::EmbeddingEngine,
    domain::{ContextEmbedding, DomainError},
};

/// Stable, process-independent deterministic embedding engine.
///
/// Unlike [`super::SimpleEmbedEngine`] (which uses `AHasher` with process-local
/// seeds), this engine derives each token slot from a SHA-256 digest, so
/// persisted vectors and freshly computed query vectors are identical across
/// independent OS processes and restarts. It is hash retrieval, not a learned
/// model.
///
/// Slot derivation: SHA-256 the token, take the first eight digest bytes as a
/// little-endian `u64`, then `slot = value % dimensions`. Tokenisation, counts
/// and L2 normalisation match [`super::SimpleEmbedEngine`].
pub struct Sha256EmbedEngine {
    model_name: String,
    dimensions: usize,
}

impl Sha256EmbedEngine {
    pub fn try_new(model_name: impl Into<String>, dimensions: usize) -> Result<Self, DomainError> {
        if dimensions == 0 {
            return Err(DomainError::validation(
                "embedding dimensions must be greater than zero",
            ));
        }
        Ok(Self {
            model_name: model_name.into(),
            dimensions: dimensions.clamp(8, 4096),
        })
    }

    pub fn new(model_name: impl Into<String>, dimensions: usize) -> Self {
        Self::try_new(model_name, dimensions).expect("valid sha256 embedder configuration")
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    fn tokenize<'a>(&self, text: &'a str) -> impl Iterator<Item = &'a str> {
        text.split(|c: char| c.is_ascii_whitespace() || c.is_ascii_punctuation())
            .filter(move |token| !token.is_empty())
    }

    fn slot(&self, token: &str) -> usize {
        let digest = Sha256::digest(token.as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        (u64::from_le_bytes(bytes) % self.dimensions as u64) as usize
    }

    fn embed_internal(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; self.dimensions];
        for token in self.tokenize(text) {
            vector[self.slot(token)] += 1.0;
        }

        // L2 normalize to keep scores in [-1, 1]
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }

        vector
    }

    pub fn embed_payload(&self, text: &str) -> Result<ContextEmbedding, DomainError> {
        if text.trim().is_empty() {
            return Err(DomainError::validation("text payload cannot be empty"));
        }
        Ok(ContextEmbedding::new(
            &self.model_name,
            self.embed_internal(text),
        ))
    }
}

impl EmbeddingEngine for Sha256EmbedEngine {
    fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>, DomainError> {
        if !model.eq_ignore_ascii_case(&self.model_name) {
            return Err(DomainError::embedding(format!(
                "engine initialised for `{}` but `{}` requested",
                self.model_name, model
            )));
        }
        if text.trim().is_empty() {
            return Err(DomainError::validation("text payload cannot be empty"));
        }
        Ok(self.embed_internal(text))
    }

    fn dims(&self, _model: &str) -> Option<usize> {
        Some(self.dimensions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "ingat/simple-sha256-v1";

    #[test]
    fn deterministic_across_engine_instances() {
        let fixtures = [
            "hello world",
            "Halo dunia",
            "日本語のテキスト",
            "emoji 🚀 test",
        ];
        for text in fixtures {
            let a = Sha256EmbedEngine::try_new(MODEL, 256).unwrap();
            let b = Sha256EmbedEngine::try_new(MODEL, 256).unwrap();
            assert_eq!(a.embed(MODEL, text).unwrap(), b.embed(MODEL, text).unwrap());
        }
    }

    #[test]
    fn matches_manual_slot_derivation() {
        let engine = Sha256EmbedEngine::try_new(MODEL, 256).unwrap();
        let vector = engine.embed(MODEL, "hello").unwrap();
        let digest = Sha256::digest(b"hello");
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        let slot = (u64::from_le_bytes(bytes) % 256) as usize;
        assert_eq!(vector[slot], 1.0);
        assert_eq!(vector.iter().filter(|v| **v != 0.0).count(), 1);
    }
}
