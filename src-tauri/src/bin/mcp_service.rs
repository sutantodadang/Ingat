#![allow(clippy::expect_used)]

/// Unified Backend Service for Multi-Client Access
///
/// This binary implements a long-running HTTP service that all Ingat clients
/// can connect to simultaneously. It solves the sled database exclusive lock
/// problem by maintaining a single process with one database connection, while
/// serving multiple clients.
///
/// # Architecture
///
/// ```text
/// ┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
/// │  Tauri UI   │────▶│                  │◀────│  VS Code    │
/// └─────────────┘     │  Backend Service │     │ (mcp-stdio) │
///                     │                  │     └─────────────┘
/// ┌─────────────┐     │  (Single Process │     ┌─────────────┐
/// │    Zed      │────▶│   One DB Lock)   │◀────│  Cursor     │
/// │(mcp-bridge) │     │                  │     │ (mcp-stdio) │
/// └─────────────┘     └──────────────────┘     └─────────────┘
/// ```
///
/// # Usage
///
/// 1. Start the service:
///    ```bash
///    mcp-service --port 3200
///    ```
///
/// 2. Configure clients to connect to http://localhost:3200
///
/// # Endpoints
///
/// - `POST /api/contexts` - Save a context
/// - `GET /api/contexts` - List contexts
/// - `POST /api/search` - Search contexts
/// - `GET /api/stats` - Get statistics
/// - `POST /import` - Import team memory wire entries (idempotent)
/// - `GET /export` - Export memories as wire entries (?scope=team&repository=<repo>)
/// - `GET /sse` - MCP SSE transport
/// - `POST /message` - MCP message endpoint
/// - `POST /mcp-stdio` - MCP stdio-over-HTTP transport
///
/// # Environment Variables
///
/// - `INGAT_LOG`: Set logging level (trace, debug, info, warn, error)
/// - `INGAT_DATA_DIR`: Override data directory location
/// - `INGAT_SERVICE_PORT`: Default port (default: 3200)
/// - `INGAT_SERVICE_HOST`: Bind address (default: 127.0.0.1)
///

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    routing::{get, post},
    Json, Router,
};

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use ingat_lib::application::{
    services::ContextApi,
    ImportResponse, IngestContextRequest, SearchRequest, SearchResponse, WireMemoryEntry,
};

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use ingat_lib::domain::{ContextSummary, MemoryScope};

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use serde::Serialize;

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use std::{net::SocketAddr, sync::Arc};

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use tokio::sync::RwLock;

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use tracing::{error, info};

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ============================================================================
// Data Types
// ============================================================================

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
#[derive(Clone)]
struct AppState {
    service: Arc<RwLock<Arc<dyn ContextApi>>>,
    data_dir: std::path::PathBuf,
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
// Use the existing DTO from application layer
// SaveContextRequest is IngestContextRequest
#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
// SearchRequest is already imported from application layer
#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
// SearchResponse is already imported from application layer
#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
#[derive(Debug, Serialize)]
struct StatsResponse {
    total_contexts: usize,
    data_dir: String,
    version: String,
    uptime_seconds: u64,
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ingat-backend"
    }))
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn save_context(
    State(state): State<AppState>,
    Json(payload): Json<IngestContextRequest>,
) -> Result<Json<ContextSummary>, (StatusCode, Json<ErrorResponse>)> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    match service.ingest(payload) {
        Ok(summary) => {
            info!("Context saved: {}", summary.id);
            Ok(Json(summary))
        }
        Err(e) => {
            error!("Failed to save context: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "SAVE_FAILED".to_string(),
                }),
            ))
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn list_contexts(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<ContextSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    let limit = params.get("limit").and_then(|s| s.parse().ok());

    let project = params.get("project").cloned();

    match service.history(project, limit) {
        Ok(response) => Ok(Json(response.items)),
        Err(e) => {
            error!("Failed to list contexts: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "LIST_FAILED".to_string(),
                }),
            ))
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    match service.projects() {
        Ok(projects) => Ok(Json(projects)),
        Err(e) => {
            error!("Failed to list projects: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "PROJECTS_FAILED".to_string(),
                }),
            ))
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn search_contexts(
    State(state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    match service.search(payload) {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Search failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "SEARCH_FAILED".to_string(),
                }),
            ))
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, StatusCode> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    // Use history with large limit to count contexts
    match service.history(None, Some(10000)) {
        Ok(response) => Ok(Json(StatsResponse {
            total_contexts: response.items.len(),
            data_dir: state.data_dir.display().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0, // TODO: track service start time
        })),
        Err(e) => {
            error!("Failed to get stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn import_memories(
    State(state): State<AppState>,
    Json(entries): Json<Vec<WireMemoryEntry>>,
) -> Result<Json<ImportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = state.service.read().await;
    let service = Arc::clone(&service);

    match service.import_memories(entries) {
        Ok(response) => {
            info!(
                "Imported {} memories ({} skipped)",
                response.imported, response.skipped
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to import memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "IMPORT_FAILED".to_string(),
                }),
            ))
        }
    }
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn export_memories(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<WireMemoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let scope = match params.get("scope").map(String::as_str) {
        None => None,
        Some("team") => Some(MemoryScope::Team),
        Some("personal") => Some(MemoryScope::Personal),
        Some(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("invalid scope '{other}': expected 'team' or 'personal'"),
                    code: "INVALID_SCOPE".to_string(),
                }),
            ))
        }
    };
    let repository = params.get("repository").cloned();

    let service = state.service.read().await;
    let service = Arc::clone(&service);

    match service.export_memories(scope, repository) {
        Ok(entries) => Ok(Json(entries)),
        Err(e) => {
            error!("Failed to export memories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "EXPORT_FAILED".to_string(),
                }),
            ))
        }
    }
}

// ============================================================================
// MCP SSE Handler (for Zed, Claude Desktop)
// ============================================================================

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn mcp_sse_handler(
    State(_state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>> {
    info!("MCP SSE client connected");

    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default().data("connected"));

        // TODO: Implement full MCP SSE protocol
        // This is a placeholder that keeps the connection alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            yield Ok(Event::default().event("ping").data("keepalive"));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ============================================================================
// MCP Message Handler (for stdio-over-HTTP)
// ============================================================================

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn mcp_message_handler(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _body: String,
) -> Response {
    info!("MCP message received");

    // TODO: Implement full MCP JSON-RPC protocol
    // This is a placeholder response
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "status": "not_implemented",
            "message": "MCP protocol implementation in progress"
        }
    });

    Json(response).into_response()
}

// ============================================================================
// Service Setup and Main
// ============================================================================

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
async fn run_service() -> anyhow::Result<()> {
    // Initialize tracing with color and formatting suitable for service logs
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,ingat_lib=debug")),
        )
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .init();

    info!(
        "Starting Ingat Backend Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Build environment (same as main app setup)
    info!("Initializing application environment...");

    let app_handles = tokio::task::spawn_blocking(|| ingat_lib::build_environment())
        .await
        .expect("Failed to spawn initialization task")
        .expect("Failed to initialize application");

    info!("Data directory: {}", app_handles.data_dir.display());

    let state = AppState {
        service: Arc::new(RwLock::new(app_handles.service)),
        data_dir: app_handles.data_dir,
    };

    info!("Application initialized successfully");

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // REST API
        .route("/api/contexts", post(save_context).get(list_contexts))
        .route("/api/search", post(search_contexts))
        .route("/api/projects", get(list_projects))
        .route("/api/stats", get(get_stats))
        // Team memory sync (issue #10)
        .route("/import", post(import_memories))
        .route("/export", get(export_memories))
        // MCP endpoints
        .route("/sse", get(mcp_sse_handler))
        .route("/message", post(mcp_message_handler))
        .with_state(state);

    // Determine bind address
    let host = std::env::var("INGAT_SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("INGAT_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3200);

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid bind address");

    info!("🚀 Ingat Backend Service listening on http://{}", addr);
    info!("📊 Health check: http://{}/health", addr);
    info!("🔌 MCP SSE endpoint: http://{}/sse", addr);
    info!("💾 REST API: http://{}/api/*", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.expect("Server failed");

    Ok(())
}

#[cfg(all(feature = "mcp-server", feature = "tauri-plugin"))]
#[tokio::main]
async fn main() {
    if let Err(err) = run_service().await {
        eprintln!("[ingat::mcp-service] Service failed: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(not(all(feature = "mcp-server", feature = "tauri-plugin")))]
fn main() {
    eprintln!("[ingat::mcp-service] Build with required features to enable the service.");
    eprintln!(
        "Example: cargo build --release --bin mcp-service --features mcp-server,tauri-plugin"
    );
    std::process::exit(1);
}
