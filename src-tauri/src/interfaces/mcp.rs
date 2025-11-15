use std::{env, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context as AnyhowContext, Result};
use parking_lot::RwLock;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_router,
    transport::sse_server::{SseServer, SseServerConfig},
    ErrorData as McpError, ServerHandler,
};
use serde_json::json;
use tokio::task;
use tokio_util::sync::CancellationToken;

use crate::{
    application::{
        dtos::{IngestContextRequest, SearchRequest},
        ContextService,
    },
    domain::DomainError,
};

// Re-export for use in binaries
pub use IngatMcpServer as IngatMcpStdioServer;

const ENV_BIND_ADDR: &str = "INGAT_MCP_BIND_ADDR";
const ENV_SSE_PATH: &str = "INGAT_MCP_SSE_PATH";
const ENV_POST_PATH: &str = "INGAT_MCP_POST_PATH";
const ENV_KEEP_ALIVE_SECS: &str = "INGAT_MCP_KEEP_ALIVE_SECS";

/// Static metadata describing the active MCP endpoints.
#[derive(Debug, Clone)]
pub struct McpEndpointMetadata {
    pub bind_addr: SocketAddr,
    pub sse_path: String,
    pub post_path: String,
}

impl McpEndpointMetadata {
    pub fn post_url(&self) -> String {
        format!("http://{}{}", self.bind_addr, self.post_path)
    }

    pub fn sse_url(&self) -> String {
        format!("http://{}{}", self.bind_addr, self.sse_path)
    }
}

/// Runtime configuration for the embedded MCP SSE server.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub bind_addr: SocketAddr,
    pub sse_path: String,
    pub post_path: String,
    pub keep_alive: Duration,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:5210".parse().expect("loopback socket"),
            sse_path: "/sse".into(),
            post_path: "/message".into(),
            keep_alive: Duration::from_secs(30),
        }
    }
}

impl McpServerConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(raw) = env::var(ENV_BIND_ADDR) {
            if let Ok(addr) = SocketAddr::from_str(&raw) {
                cfg.bind_addr = addr;
            }
        }

        if let Ok(path) = env::var(ENV_SSE_PATH) {
            cfg.sse_path = normalize_path(&path);
        }

        if let Ok(path) = env::var(ENV_POST_PATH) {
            cfg.post_path = normalize_path(&path);
        }

        if let Ok(raw) = env::var(ENV_KEEP_ALIVE_SECS) {
            if let Ok(seconds) = raw.parse::<u64>() {
                cfg.keep_alive = Duration::from_secs(seconds.max(5));
            }
        }

        cfg
    }

    fn into_pair(self, cancel_token: CancellationToken) -> (SseServerConfig, McpEndpointMetadata) {
        (
            SseServerConfig {
                bind: self.bind_addr,
                sse_path: self.sse_path.clone(),
                post_path: self.post_path.clone(),
                ct: cancel_token,
                sse_keep_alive: Some(self.keep_alive),
            },
            McpEndpointMetadata {
                bind_addr: self.bind_addr,
                sse_path: self.sse_path,
                post_path: self.post_path,
            },
        )
    }
}

fn normalize_path(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "/".into();
    }
    if trimmed.starts_with('/') {
        trimmed.into()
    } else {
        format!("/{}", trimmed)
    }
}

/// Handle to the background MCP server. Dropping the handle shuts it down.
#[derive(Clone)]
pub struct McpServerHandle {
    root_token: CancellationToken,
    worker_token: CancellationToken,
    metadata: Arc<McpEndpointMetadata>,
}

impl McpServerHandle {
    pub fn shutdown(&self) {
        self.worker_token.cancel();
        self.root_token.cancel();
    }

    pub fn metadata(&self) -> &McpEndpointMetadata {
        &self.metadata
    }
}

impl Drop for McpServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Boot an MCP SSE server that mirrors Ingat's ingest/search capabilities.
pub async fn spawn_mcp_server(
    service_cell: Arc<RwLock<Arc<ContextService>>>,
    config: McpServerConfig,
) -> Result<McpServerHandle> {
    let root_token = CancellationToken::new();
    let (sse_config, metadata) = config.into_pair(root_token.clone());

    let sse_server = SseServer::serve_with_config(sse_config)
        .await
        .context("failed to bind MCP SSE listener")?;

    let worker_token =
        sse_server.with_service(move || IngatMcpServer::new(Arc::clone(&service_cell)));

    Ok(McpServerHandle {
        root_token,
        worker_token,
        metadata: Arc::new(metadata),
    })
}

pub struct McpRuntime {
    handle: McpServerHandle,
}

impl McpRuntime {
    pub async fn start(
        service_cell: Arc<RwLock<Arc<ContextService>>>,
        config: Option<McpServerConfig>,
    ) -> Result<Self> {
        let cfg = config.unwrap_or_else(McpServerConfig::from_env);
        let handle = spawn_mcp_server(service_cell, cfg).await?;
        Ok(Self { handle })
    }

    pub fn metadata(&self) -> &McpEndpointMetadata {
        self.handle.metadata()
    }

    pub fn shutdown(self) {
        self.handle.shutdown();
    }
}

#[derive(Clone)]
pub struct IngatMcpServer {
    service_cell: Arc<RwLock<Arc<ContextService>>>,
    tool_router: ToolRouter<Self>,
}

impl IngatMcpServer {
    pub fn new(service_cell: Arc<RwLock<Arc<ContextService>>>) -> Self {
        Self {
            service_cell,
            tool_router: Self::tool_router(),
        }
    }

    fn current_service(&self) -> Arc<ContextService> {
        Arc::clone(&self.service_cell.read())
    }

    async fn ingest(&self, payload: IngestContextRequest) -> Result<CallToolResult, McpError> {
        let service = self.current_service();
        let summary = task::spawn_blocking(move || service.ingest(payload))
            .await
            .map_err(|err| internal_error(err.to_string()))?
            .map_err(map_domain_error)?;

        let value = serde_json::to_value(summary).map_err(|err| internal_error(err.to_string()))?;
        Ok(CallToolResult::structured(value))
    }

    async fn search(&self, payload: SearchRequest) -> Result<CallToolResult, McpError> {
        let service = self.current_service();
        let response = task::spawn_blocking(move || service.search(payload))
            .await
            .map_err(|err| internal_error(err.to_string()))?
            .map_err(map_domain_error)?;

        let value =
            serde_json::to_value(response).map_err(|err| internal_error(err.to_string()))?;
        Ok(CallToolResult::structured(value))
    }
}

#[tool_router]
impl IngatMcpServer {
    #[tool(
        name = "ingest_context",
        description = "Persist a new Ingat context item."
    )]
    async fn ingest_context(
        &self,
        Parameters(payload): Parameters<IngestContextRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.ingest(payload).await
    }

    #[tool(
        name = "search_contexts",
        description = "Execute a semantic search across stored contexts."
    )]
    async fn search_contexts(
        &self,
        Parameters(payload): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.search(payload).await
    }
}

impl ServerHandler for IngatMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "ingat-mcp".into(),
                title: Some("Ingat Context Bridge".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                r#"🧠 Ingat - Your AI's Long-Term Memory

AUTOMATIC BEHAVIORS YOU MUST FOLLOW:

1. 🔍 ALWAYS SEARCH FIRST:
   - Before answering ANY coding question, call search_contexts
   - Check if we've solved this before
   - Use relevant context in your answer
   - Example: User asks "How to handle API errors?" → First: search_contexts("API error handling")

2. 💾 ALWAYS SAVE SOLUTIONS:
   - After fixing bugs → ingest_context (kind: "FixHistory")
   - After implementing features → ingest_context (kind: "CodeSnippet")
   - After architectural decisions → ingest_context (kind: "Discussion")
   - After discoveries → ingest_context (kind: "Other")

3. 🏷️ USE GOOD TAGS:
   - Include: file paths, languages, problem types
   - Example: ["api", "error-handling", "typescript", "bug-fix"]
   - Max 12 tags, make them searchable

4. 📝 WRITE CLEAR SUMMARIES:
   - Summary: Brief description (max 640 chars)
   - Body: Full details including code (max 16,000 chars)
   - Make summaries searchable for future queries

5. 🤖 BE PROACTIVE:
   - Say: "Let me check Ingat for similar solutions..."
   - Say: "I'll save this for future reference..."
   - Say: "Found in Ingat: [relevant context]..."

6. 🎯 SEARCH TRIGGERS:
   - User reports bug/error
   - User asks "how to" questions
   - Starting new features
   - Discussing architecture
   - Requesting examples

7. 💡 SAVE TRIGGERS:
   - Successfully fixed a bug
   - Implemented a feature
   - Made architectural decision
   - Discovered useful pattern
   - User says "thanks" after solution

WORKFLOW:
1. User asks question → search_contexts first
2. Provide answer using found context + your knowledge
3. User confirms success → ingest_context to save
4. Build knowledge base over time

Treat Ingat as your long-term memory. Always search before answering, always save after solving."#
                    .into(),
            ),
        }
    }
}

fn map_domain_error(err: DomainError) -> McpError {
    match err {
        DomainError::Validation(msg) | DomainError::LimitExceeded(msg) => {
            McpError::invalid_params("invalid request", Some(json!({ "detail": msg })))
        }
        DomainError::NotFound(msg) => {
            McpError::resource_not_found("not found", Some(json!({ "detail": msg })))
        }
        DomainError::Embedding(msg) | DomainError::Storage(msg) | DomainError::Other(msg) => {
            McpError::internal_error("internal error", Some(json!({ "detail": msg })))
        }
    }
}

fn internal_error(message: impl Into<String>) -> McpError {
    McpError::internal_error(
        "internal MCP server error",
        Some(json!({ "detail": message.into() })),
    )
}

/// Run MCP server using stdio transport (stdin/stdout).
/// This is compatible with VS Code, Cursor, Windsurf, and other process-spawning MCP clients.
pub async fn run_mcp_stdio_server(service_cell: Arc<RwLock<Arc<ContextService>>>) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tracing::{debug, error, info};

    info!(target: "ingat::mcp", "Starting MCP stdio server...");

    let server = IngatMcpServer::new(service_cell);
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF - client closed connection
                info!(target: "memorust::mcp", "Client closed stdio connection");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!(target: "memorust::mcp", "Received: {}", trimmed);

                // Parse and handle JSON-RPC request
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(request) => {
                        // Handle the request using rmcp's handler
                        let response = handle_jsonrpc_request(&server, request).await;

                        // Write response to stdout
                        let response_json = serde_json::to_string(&response)
                            .unwrap_or_else(|e| format!(r#"{{"jsonrpc":"2.0","error":{{"code":-32603,"message":"Failed to serialize response: {}"}}}}"#, e));

                        if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                            error!(target: "memorust::mcp", "Failed to write response: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.write_all(b"\n").await {
                            error!(target: "memorust::mcp", "Failed to write newline: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.flush().await {
                            error!(target: "memorust::mcp", "Failed to flush stdout: {}", e);
                            break;
                        }

                        debug!(target: "memorust::mcp", "Sent: {}", response_json);
                    }
                    Err(e) => {
                        error!(target: "memorust::mcp", "Failed to parse JSON-RPC request: {}", e);
                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32700,
                                "message": format!("Parse error: {}", e)
                            }
                        });
                        let _ = stdout
                            .write_all(serde_json::to_string(&error_response).unwrap().as_bytes())
                            .await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
            }
            Err(e) => {
                error!(target: "memorust::mcp", "Failed to read from stdin: {}", e);
                break;
            }
        }
    }

    info!(target: "memorust::mcp", "MCP stdio server terminated");
    Ok(())
}

/// Handle JSON-RPC requests for MCP
async fn handle_jsonrpc_request(
    server: &IngatMcpServer,
    request: serde_json::Value,
) -> serde_json::Value {
    // Extract request fields
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "initialize" => {
            let info = server.get_info();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": info.protocol_version,
                    "capabilities": info.capabilities,
                    "serverInfo": info.server_info,
                    "instructions": info.instructions
                }
            })
        }
        "tools/list" => {
            let tools = server.tool_router.list_all();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            let params = request.get("params");
            match params {
                Some(p) => {
                    let tool_name = p.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = p.get("arguments").cloned().unwrap_or(json!({}));

                    // Call the appropriate tool method directly
                    let result = match tool_name {
                        "ingest_context" => {
                            match serde_json::from_value::<IngestContextRequest>(arguments) {
                                Ok(req) => server.ingest(req).await,
                                Err(e) => Err(McpError::invalid_params(
                                    "Invalid ingest_context arguments",
                                    Some(json!({"detail": e.to_string()})),
                                )),
                            }
                        }
                        "search_contexts" => {
                            match serde_json::from_value::<SearchRequest>(arguments) {
                                Ok(req) => server.search(req).await,
                                Err(e) => Err(McpError::invalid_params(
                                    "Invalid search_contexts arguments",
                                    Some(json!({"detail": e.to_string()})),
                                )),
                            }
                        }
                        _ => Err(McpError::invalid_params(
                            format!("Unknown tool: {}", tool_name),
                            None,
                        )),
                    };

                    match result {
                        Ok(result) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": result
                        }),
                        Err(e) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": e.code,
                                "message": e.message,
                                "data": e.data
                            }
                        }),
                    }
                }
                None => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params"
                    }
                }),
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        }),
    }
}
