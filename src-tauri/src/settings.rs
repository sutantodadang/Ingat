use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// Default filename used to persist configuration within the data directory.
const CONFIG_FILENAME: &str = "config.json";

/// Declarative list of embedding backends compiled into the binary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub enum EmbeddingBackend {
    /// Lightweight deterministic hash embedder (always available).
    Simple {
        #[serde(default = "default_simple_model")]
        model: String,
        #[serde(default = "default_simple_dim")]
        dimensions: usize,
    },
    /// High-quality semantic embeddings powered by FastEmbed (feature gated).
    #[cfg(feature = "fastembed-engine")]
    FastEmbed { model: String },
}

impl EmbeddingBackend {
    pub fn id(&self) -> &'static str {
        match self {
            EmbeddingBackend::Simple { .. } => "simple",
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed { .. } => "fastembed",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EmbeddingBackend::Simple { .. } => "Deterministic Hash (offline)",
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed { .. } => "FastEmbed (semantic)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EmbeddingBackend::Simple { .. } => {
                "Small, deterministic vectors suitable for quick local testing."
            }
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed { .. } => {
                "High-quality semantic embeddings via fastembed/ONNX runtime."
            }
        }
    }

    pub fn is_feature_gated(&self) -> bool {
        #[cfg(feature = "fastembed-engine")]
        {
            return matches!(self, EmbeddingBackend::FastEmbed { .. });
        }

        #[cfg(not(feature = "fastembed-engine"))]
        {
            false
        }
    }

    pub fn model_name(&self) -> &str {
        match self {
            EmbeddingBackend::Simple { model, .. } => model,
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed { model } => model,
        }
    }

    pub fn expected_dimensions(&self) -> Option<usize> {
        match self {
            EmbeddingBackend::Simple { dimensions, .. } => Some(*dimensions),
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed { .. } => None,
        }
    }

    pub fn with_default_model(id: &str) -> Option<Self> {
        match id {
            "simple" => Some(EmbeddingBackend::Simple {
                model: default_simple_model(),
                dimensions: default_simple_dim(),
            }),
            #[cfg(feature = "fastembed-engine")]
            "fastembed" => Some(EmbeddingBackend::FastEmbed {
                model: default_fastembed_model(),
            }),
            _ => None,
        }
    }
}

impl Default for EmbeddingBackend {
    fn default() -> Self {
        #[cfg(feature = "fastembed-engine")]
        {
            EmbeddingBackend::FastEmbed {
                model: default_fastembed_model(),
            }
        }
        #[cfg(not(feature = "fastembed-engine"))]
        {
            EmbeddingBackend::Simple {
                model: default_simple_model(),
                dimensions: default_simple_dim(),
            }
        }
    }
}

/// Complete persisted configuration payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub embedding: EmbeddingBackend,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            embedding: EmbeddingBackend::default(),
        }
    }
}

/// Thread-safe manager responsible for loading and persisting `AppConfig`.
pub struct ConfigManager {
    path: PathBuf,
    state: RwLock<AppConfig>,
}

impl ConfigManager {
    /// Create a manager rooted at `data_dir`. The JSON file will be located at
    /// `<data_dir>/config.json`.
    pub fn load(data_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = data_dir.as_ref().join(CONFIG_FILENAME);
        let config = if path.exists() {
            fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<AppConfig>(&bytes).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        };

        Ok(Self {
            path,
            state: RwLock::new(config),
        })
    }

    /// Snapshot of the current configuration.
    pub fn current(&self) -> AppConfig {
        self.state.read().expect("config poisoned").clone()
    }

    /// Update the active embedding backend and persist to disk.
    pub fn set_backend(&self, backend: EmbeddingBackend) -> std::io::Result<AppConfig> {
        {
            let mut guard = self.state.write().expect("config poisoned");
            guard.embedding = backend;
            self.persist_locked(&guard)?;
        }
        Ok(self.current())
    }

    /// Ensure the backing directory exists and write the JSON payload.
    fn persist_locked(&self, config: &AppConfig) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = serde_json::to_vec_pretty(config)?;
        fs::write(&self.path, payload)
    }
}

pub fn available_backends() -> Vec<EmbeddingBackend> {
    #[cfg(feature = "fastembed-engine")]
    {
        vec![
            EmbeddingBackend::FastEmbed {
                model: default_fastembed_model(),
            },
            EmbeddingBackend::Simple {
                model: default_simple_model(),
                dimensions: default_simple_dim(),
            },
        ]
    }
    #[cfg(not(feature = "fastembed-engine"))]
    {
        vec![EmbeddingBackend::default()]
    }
}

const fn default_simple_dim() -> usize {
    256
}

fn default_simple_model() -> String {
    "ingat/simple-hash".to_string()
}

#[cfg(feature = "fastembed-engine")]
fn default_fastembed_model() -> String {
    "BAAI/bge-small-en-v1.5".to_string()
}
