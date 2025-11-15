use std::str::FromStr;

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use parking_lot::Mutex;

use crate::{application::services::EmbeddingEngine, domain::DomainError};

/// Embedding engine backed by `fastembed`'s `TextEmbedding`.
///
/// The engine keeps a single `TextEmbedding` instance behind a `Mutex`, which
/// allows us to reuse the loaded model without cloning heavyweight resources.
pub struct FastEmbedEngine {
    model_label: String,
    dimensions: usize,
    inner: Mutex<TextEmbedding>,
}

impl FastEmbedEngine {
    /// Create a new engine for the given model (for example `BAAI/bge-small-en-v1.5`).
    pub fn try_new(model_name: impl AsRef<str>) -> Result<Self, DomainError> {
        let label = model_name.as_ref().trim();
        if label.is_empty() {
            return Err(DomainError::validation(
                "fastembed model name cannot be empty",
            ));
        }

        let embedding_model = EmbeddingModel::from_str(label).map_err(|err| {
            DomainError::other(format!("failed to parse fastembed model `{label}`: {err}"))
        })?;

        let model_info = TextEmbedding::get_model_info(&embedding_model).map_err(|err| {
            DomainError::other(format!(
                "unable to read metadata for fastembed model `{label}`: {err}"
            ))
        })?;

        let init_options = TextInitOptions::new(embedding_model.clone());
        let text_embedding = TextEmbedding::try_new(init_options).map_err(|err| {
            DomainError::other(format!(
                "failed to initialise fastembed model `{label}`: {err}"
            ))
        })?;

        Ok(Self {
            model_label: label.to_string(),
            dimensions: model_info.dim,
            inner: Mutex::new(text_embedding),
        })
    }
}

impl EmbeddingEngine for FastEmbedEngine {
    fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>, DomainError> {
        if !model.eq_ignore_ascii_case(&self.model_label) {
            return Err(DomainError::embedding(format!(
                "engine initialised for `{}` but `{}` requested",
                self.model_label, model
            )));
        }

        if text.trim().is_empty() {
            return Err(DomainError::validation("text payload cannot be empty"));
        }

        let mut embedder = self.inner.lock();
        let embeddings = embedder
            .embed(vec![text], None)
            .map_err(|err| DomainError::other(format!("fastembed inference failed: {err}")))?;
        let vector = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::other("fastembed returned no embedding"))?;

        if vector.len() != self.dimensions {
            return Err(DomainError::embedding(format!(
                "unexpected embedding dimension (expected {}, got {})",
                self.dimensions,
                vector.len()
            )));
        }

        Ok(vector)
    }

    fn dims(&self, _model: &str) -> Option<usize> {
        Some(self.dimensions)
    }
}
