//! Remote vector store implementation that proxies operations to mcp-service via HTTP.

use uuid::Uuid;

use crate::application::services::VectorStore;
use crate::domain::{ContextEmbedding, ContextRecord, ContextSummary, DomainError, QueryFilters};

use super::get_service_url;

/// Vector store implementation that proxies all operations to a remote mcp-service
pub struct RemoteVectorStore {
    base_url: String,
    agent: ureq::Agent,
}

impl RemoteVectorStore {
    /// Create a new remote vector store client
    pub fn new(host: &str, port: u16) -> Self {
        let base_url = get_service_url(host, port);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build();

        Self { base_url, agent }
    }

    /// Get the API endpoint URL
    fn api_url(&self, path: &str) -> String {
        format!("{}/api/{}", self.base_url, path)
    }
}

impl VectorStore for RemoteVectorStore {
    fn persist(&self, record: &ContextRecord) -> Result<(), DomainError> {
        let url = self.api_url("contexts");

        // Convert ContextRecord to IngestContextRequest format
        let request_body = serde_json::json!({
            "project": record.project,
            "ide": record.ide,
            "file_path": record.file_path,
            "language": record.language,
            "summary": record.summary,
            "body": record.body,
            "tags": record.tags,
            "kind": record.kind,
        });

        self.agent
            .post(&url)
            .send_json(request_body)
            .map_err(|e| DomainError::storage(format!("Failed to save context: {}", e)))?;

        Ok(())
    }

    fn search(
        &self,
        embedding: &ContextEmbedding,
        limit: usize,
        filters: &QueryFilters,
    ) -> Result<Vec<(ContextRecord, f32)>, DomainError> {
        let url = self.api_url("search");

        let request_body = serde_json::json!({
            "prompt": "",
            "embedding": &embedding.vector,
            "limit": limit,
            "project": filters.project,
            "kind": filters.kind,
        });

        let response = self
            .agent
            .post(&url)
            .send_json(request_body)
            .map_err(|e| DomainError::storage(format!("Search failed: {}", e)))?;

        // Parse SearchResponse
        let search_response: serde_json::Value = response
            .into_json()
            .map_err(|e| DomainError::storage(format!("Failed to parse search response: {}", e)))?;

        let results = search_response["results"]
            .as_array()
            .ok_or_else(|| DomainError::storage("Invalid search response format"))?;

        let records: Vec<(ContextRecord, f32)> = results
            .iter()
            .filter_map(|item| {
                let score = item["score"].as_f64().unwrap_or(0.0) as f32;
                // Extract the record fields from the result
                let record = ContextRecord {
                    id: item["id"].as_str().and_then(|s| Uuid::parse_str(s).ok())?,
                    project: item["project"].as_str().unwrap_or("").to_string(),
                    ide: item
                        .get("ide")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    file_path: item
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    language: item
                        .get("language")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    summary: item["summary"].as_str().unwrap_or("").to_string(),
                    body: item["body"].as_str().unwrap_or("").to_string(),
                    tags: Vec::new(),
                    kind: serde_json::from_value(item["kind"].clone()).ok()?,
                    embedding: ContextEmbedding::new("remote", Vec::new()),
                    created_at: serde_json::from_value(item["created_at"].clone()).ok()?,
                };
                Some((record, score))
            })
            .collect();

        Ok(records)
    }

    fn recent(
        &self,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextSummary>, DomainError> {
        let mut url = self.api_url("contexts");

        // Build query parameters
        let mut params = Vec::new();
        if let Some(proj) = project {
            params.push(format!("project={}", urlencoding::encode(proj)));
        }
        params.push(format!("limit={}", limit));

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| DomainError::storage(format!("Failed to list contexts: {}", e)))?;

        let summaries: Vec<ContextSummary> = response
            .into_json()
            .map_err(|e| DomainError::storage(format!("Failed to parse list response: {}", e)))?;

        Ok(summaries)
    }

    fn projects(&self) -> Result<Vec<String>, DomainError> {
        // TODO: Implement projects endpoint on mcp-service
        // For now, return empty list
        Ok(Vec::new())
    }

    fn ping(&self) -> Result<(), DomainError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| DomainError::storage(format!("Health check failed: {}", e)))?;

        if response.status() == 200 {
            Ok(())
        } else {
            Err(DomainError::storage("Remote service is not healthy"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_store_creation() {
        let store = RemoteVectorStore::new("localhost", 3200);
        assert!(store.api_url("test").contains("localhost:3200"));
    }
}
