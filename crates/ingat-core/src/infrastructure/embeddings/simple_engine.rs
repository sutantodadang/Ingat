use ahash::AHasher;
use std::hash::{Hash, Hasher};

use crate::{
    application::services::EmbeddingEngine,
    domain::{ContextEmbedding, DomainError},
};

/// A lightweight, deterministic embedding engine that hashes tokens into a fixed-size vector.
/// This is not meant for production-grade semantic search, but it keeps the application functional
/// without downloading external models or shipping native dependencies.
pub struct SimpleEmbedEngine {
    model_name: String,
    dimensions: usize,
}

impl SimpleEmbedEngine {
    pub fn try_new(model_name: impl Into<String>, dimensions: usize) -> Result<Self, DomainError> {
        if dimensions == 0 {
            return Err(DomainError::validation(
                "embedding dimensions must be greater than zero",
            ));
        }
        let dims = dimensions.clamp(8, 4096);
        Ok(Self {
            model_name: model_name.into(),
            dimensions: dims,
        })
    }

    pub fn new(model_name: impl Into<String>, dimensions: usize) -> Self {
        Self::try_new(model_name, dimensions).expect("valid simple embedder configuration")
    }

    fn tokenize<'a>(&self, text: &'a str) -> impl Iterator<Item = &'a str> {
        text.split(|c: char| c.is_ascii_whitespace() || c.is_ascii_punctuation())
            .filter(move |token| !token.is_empty())
    }

    fn hash_token(&self, token: &str) -> usize {
        let mut hasher = AHasher::default();
        token.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn embed_internal(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; self.dimensions];
        let tokens: Vec<&str> = self.tokenize(text).collect();
        if tokens.is_empty() {
            return vector;
        }

        for token in tokens {
            let hash = self.hash_token(token);
            let idx = hash % self.dimensions;
            vector[idx] += 1.0;
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

impl Default for SimpleEmbedEngine {
    fn default() -> Self {
        Self::try_new("ingat/simple-hash", 256)
            .expect("default simple embedder configuration is valid")
    }
}

impl EmbeddingEngine for SimpleEmbedEngine {
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
