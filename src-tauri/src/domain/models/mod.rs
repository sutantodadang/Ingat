use chrono::{DateTime, Utc};
#[cfg(feature = "mcp-server")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Upper bound to keep tag arrays compact for storage and filtering.
pub const MAX_TAGS: usize = 12;

/// Core record representing a stored context chunk, embedding, and its metadata.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[cfg_attr(feature = "mcp-server", schemars(rename_all = "camelCase"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRecord {
    #[cfg_attr(feature = "mcp-server", schemars(with = "String"))]
    pub id: Uuid,
    pub project: String,
    pub ide: String,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    pub kind: ContextKind,
    pub embedding: ContextEmbedding,
    pub created_at: DateTime<Utc>,
}

impl ContextRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: impl Into<String>,
        ide: impl Into<String>,
        file_path: Option<impl Into<String>>,
        language: Option<impl Into<String>>,
        summary: impl Into<String>,
        body: impl Into<String>,
        tags: impl IntoIterator<Item = impl Into<String>>,
        kind: ContextKind,
        embedding: ContextEmbedding,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project: sanitize_project(project),
            ide: sanitize_single_line(ide),
            file_path: file_path.map(|p| p.into()),
            language: language.map(|l| l.into()),
            summary: summary.into(),
            body: body.into(),
            tags: normalize_tags(tags),
            kind,
            embedding,
            created_at: Utc::now(),
        }
    }

    pub fn matches_filters(&self, filters: &QueryFilters) -> bool {
        if let Some(project) = &filters.project {
            if &self.project != project {
                return false;
            }
        }
        if let Some(kind) = &filters.kind {
            if &self.kind != kind {
                return false;
            }
        }
        if let Some(tag) = &filters.tag {
            if !self.tags.iter().any(|t| t == tag) {
                return false;
            }
        }
        if let Some(ide) = &filters.ide {
            if &self.ide != ide {
                return false;
            }
        }
        true
    }

    pub fn as_summary(&self) -> ContextSummary {
        ContextSummary {
            id: self.id,
            project: self.project.clone(),
            summary: self.summary.clone(),
            kind: self.kind.clone(),
            created_at: self.created_at,
            tags: self.tags.clone(),
        }
    }
}

/// Lightweight projection returned to the UI for history listings.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[cfg_attr(feature = "mcp-server", schemars(rename_all = "camelCase"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    #[cfg_attr(feature = "mcp-server", schemars(with = "String"))]
    pub id: Uuid,
    pub project: String,
    pub summary: String,
    pub kind: ContextKind,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Input for retrieval requests originating from the UI or MCP clients.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQuery {
    pub prompt: String,
    pub filters: QueryFilters,
    pub limit: usize,
}

/// Supported filters for narrowing search results.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    pub project: Option<String>,
    pub kind: Option<ContextKind>,
    pub tag: Option<String>,
    pub ide: Option<String>,
}

#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextKind {
    CodeSnippet,
    FixHistory,
    ProjectSummary,
    Discussion,
    ToolLog,
    Other(String),
}

impl Default for ContextKind {
    fn default() -> Self {
        ContextKind::Other("unspecified".into())
    }
}

/// Vector representation of a context chunk.
#[cfg_attr(feature = "mcp-server", derive(JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEmbedding {
    pub model: String,
    pub vector: Vec<f32>,
}

impl ContextEmbedding {
    pub fn new(model: impl Into<String>, vector: Vec<f32>) -> Self {
        Self {
            model: model.into(),
            vector,
        }
    }

    pub fn dims(&self) -> usize {
        self.vector.len()
    }
}

fn sanitize_project(input: impl Into<String>) -> String {
    sanitize_single_line(input).replace(['\\', '/', ':'], "-")
}

fn sanitize_single_line(input: impl Into<String>) -> String {
    input
        .into()
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn normalize_tags(tags: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    tags.into_iter()
        .filter_map(|tag| {
            let normalized = tag.into().trim().to_lowercase().replace(' ', "-");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
        .take(MAX_TAGS)
        .collect()
}
