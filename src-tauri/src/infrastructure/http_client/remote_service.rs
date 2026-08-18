//! Remote context service implementation that proxies operations to mcp-service via HTTP.

use std::collections::BTreeSet;

use ureq::Agent;

use crate::application::dtos::{
    HealthStatusResponse, ImportResponse, IngestContextRequest, SearchRequest, SearchResponse,
    SummaryListResponse, WireMemoryEntry,
};
use crate::application::services::ContextApi;
use crate::domain::{ContextSummary, DomainError, MemoryScope};

use super::{get_service_url, handle_http_error};

pub struct RemoteContextClient {
    base_url: String,
    agent: Agent,
}

impl RemoteContextClient {
    pub fn new(host: &str, port: u16) -> Self {
        let base_url = get_service_url(host, port);
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build();
        Self { base_url, agent }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/{}", self.base_url, path)
    }
}

impl ContextApi for RemoteContextClient {
    fn ingest(&self, payload: IngestContextRequest) -> Result<ContextSummary, DomainError> {
        let url = self.api_url("contexts");

        let response = self
            .agent
            .post(&url)
            .send_json(serde_json::to_value(payload).map_err(|e| {
                DomainError::storage(format!("failed to serialize ingest request: {e}"))
            })?)
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        response
            .into_json::<ContextSummary>()
            .map_err(|e| DomainError::storage(format!("failed to parse ingest response: {e}")))
    }

    fn search(&self, request: SearchRequest) -> Result<SearchResponse, DomainError> {
        let url = self.api_url("search");

        let response = self
            .agent
            .post(&url)
            .send_json(serde_json::to_value(request).map_err(|e| {
                DomainError::storage(format!("failed to serialize search request: {e}"))
            })?)
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        response
            .into_json::<SearchResponse>()
            .map_err(|e| DomainError::storage(format!("failed to parse search response: {e}")))
    }

    fn history(
        &self,
        project: Option<String>,
        limit: Option<usize>,
    ) -> Result<SummaryListResponse, DomainError> {
        let mut url = self.api_url("contexts");

        let mut params = Vec::new();
        if let Some(project) = project.as_deref() {
            params.push(format!("project={}", urlencoding::encode(project)));
        }
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        let items = response
            .into_json::<Vec<ContextSummary>>()
            .map_err(|e| DomainError::storage(format!("failed to parse history response: {e}")))?;

        Ok(SummaryListResponse { items })
    }

    fn projects(&self) -> Result<Vec<String>, DomainError> {
        // Prefer dedicated endpoint if available.
        let url = self.api_url("projects");
        let response = self.agent.get(&url).call();

        match response {
            Ok(ok) => ok.into_json::<Vec<String>>().map_err(|e| {
                DomainError::storage(format!("failed to parse projects response: {e}"))
            }),
            Err(ureq::Error::Status(404, _)) => {
                // Back-compat fallback: derive projects from the latest contexts.
                let summaries = self.history(None, Some(10_000))?.items;
                let mut unique = BTreeSet::new();
                for summary in summaries {
                    unique.insert(summary.project);
                }
                Ok(unique.into_iter().collect())
            }
            Err(e) => Err(DomainError::storage(handle_http_error(e).to_string())),
        }
    }

    fn health(&self) -> Result<HealthStatusResponse, DomainError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        if response.status() != 200 {
            return Err(DomainError::storage("remote service is not healthy"));
        }

        Ok(HealthStatusResponse {
            ok: true,
            message: "ready".into(),
            details: Some("remote mode".into()),
        })
    }

    fn import_memories(&self, entries: Vec<WireMemoryEntry>) -> Result<ImportResponse, DomainError> {
        let url = format!("{}/import", self.base_url);

        let response = self
            .agent
            .post(&url)
            .send_json(serde_json::to_value(entries).map_err(|e| {
                DomainError::storage(format!("failed to serialize import request: {e}"))
            })?)
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        response
            .into_json::<ImportResponse>()
            .map_err(|e| DomainError::storage(format!("failed to parse import response: {e}")))
    }

    fn export_memories(
        &self,
        scope: Option<MemoryScope>,
        repository: Option<String>,
    ) -> Result<Vec<WireMemoryEntry>, DomainError> {
        let mut url = format!("{}/export", self.base_url);

        let mut params = Vec::new();
        if let Some(scope) = scope {
            let value = match scope {
                MemoryScope::Team => "team",
                MemoryScope::Personal => "personal",
            };
            params.push(format!("scope={value}"));
        }
        if let Some(repository) = repository.as_deref() {
            params.push(format!("repository={}", urlencoding::encode(repository)));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| DomainError::storage(handle_http_error(e).to_string()))?;

        response
            .into_json::<Vec<WireMemoryEntry>>()
            .map_err(|e| DomainError::storage(format!("failed to parse export response: {e}")))
    }
}
