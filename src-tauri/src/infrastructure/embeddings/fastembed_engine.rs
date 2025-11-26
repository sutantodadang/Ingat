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

        let embedding_model = Self::parse_model(label)?;

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

    fn parse_model(s: &str) -> Result<EmbeddingModel, DomainError> {
        match s {
            "BAAI/bge-small-en-v1.5" | "bge-small-en-v1.5" | "BGESmallENV15" => {
                Ok(EmbeddingModel::BGESmallENV15)
            }
            "sentence-transformers/all-MiniLM-L6-v2" | "all-MiniLM-L6-v2" | "AllMiniLML6V2" => {
                Ok(EmbeddingModel::AllMiniLML6V2)
            }
            "mixedbread-ai/mxbai-embed-large-v1" | "mxbai-embed-large-v1" | "MxbaiEmbedLargeV1" => {
                Ok(EmbeddingModel::MxbaiEmbedLargeV1)
            }
            "Qdrant/clip-ViT-B-32-text" | "clip-ViT-B-32-text" | "ClipVitB32" => {
                Ok(EmbeddingModel::ClipVitB32)
            }
            "BAAI/bge-large-en-v1.5" | "bge-large-en-v1.5" | "BGELargeENV15" => {
                Ok(EmbeddingModel::BGELargeENV15)
            }
            "BAAI/bge-small-zh-v1.5" | "bge-small-zh-v1.5" | "BGESmallZHV15" => {
                Ok(EmbeddingModel::BGESmallZHV15)
            }
            "BAAI/bge-large-zh-v1.5" | "bge-large-zh-v1.5" | "BGELargeZHV15" => {
                Ok(EmbeddingModel::BGELargeZHV15)
            }
            "BAAI/bge-base-en-v1.5" | "bge-base-en-v1.5" | "BGEBaseENV15" => {
                Ok(EmbeddingModel::BGEBaseENV15)
            }
            "sentence-transformers/all-MiniLM-L12-v2" | "all-MiniLM-L12-v2" | "AllMiniLML12V2" => {
                Ok(EmbeddingModel::AllMiniLML12V2)
            }
            "sentence-transformers/paraphrase-multilingual-mpnet-base-v2"
            | "paraphrase-multilingual-mpnet-base-v2"
            | "ParaphraseMLMpnetBaseV2" => Ok(EmbeddingModel::ParaphraseMLMpnetBaseV2),
            "lightonai/ModernBERT-embed-large"
            | "ModernBERT-embed-large"
            | "ModernBertEmbedLarge" => Ok(EmbeddingModel::ModernBertEmbedLarge),
            "nomic-ai/nomic-embed-text-v1" | "nomic-embed-text-v1" | "NomicEmbedTextV1" => {
                Ok(EmbeddingModel::NomicEmbedTextV1)
            }
            "nomic-ai/nomic-embed-text-v1.5" | "nomic-embed-text-v1.5" | "NomicEmbedTextV15" => {
                Ok(EmbeddingModel::NomicEmbedTextV15)
            }
            "intfloat/multilingual-e5-small" | "multilingual-e5-small" | "MultilingualE5Small" => {
                Ok(EmbeddingModel::MultilingualE5Small)
            }
            "intfloat/multilingual-e5-base" | "multilingual-e5-base" | "MultilingualE5Base" => {
                Ok(EmbeddingModel::MultilingualE5Base)
            }
            "intfloat/multilingual-e5-large" | "multilingual-e5-large" | "MultilingualE5Large" => {
                Ok(EmbeddingModel::MultilingualE5Large)
            }
            "Alibaba-NLP/gte-base-en-v1.5" | "gte-base-en-v1.5" | "GTEBaseENV15" => {
                Ok(EmbeddingModel::GTEBaseENV15)
            }
            "Alibaba-NLP/gte-large-en-v1.5" | "gte-large-en-v1.5" | "GTELargeENV15" => {
                Ok(EmbeddingModel::GTELargeENV15)
            }
            _ => Err(DomainError::other(format!(
                "Unknown embedding model: {}",
                s
            ))),
        }
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
