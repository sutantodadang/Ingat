use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use tauri::State;

pub mod application;
pub mod domain;
pub mod infrastructure;
#[cfg(feature = "mcp-server")]
pub mod interfaces;
pub mod service_manager;
pub mod settings;

use application::services::{EmbeddingEngine as EmbeddingEngineTrait, VectorStore};
use application::{
    ContextService, EmbeddingBackendListResponse, EmbeddingBackendOption, HealthStatusResponse,
    IngestContextRequest, SearchRequest, SearchResponse, SummaryListResponse,
    UpdateEmbeddingBackendRequest,
};
use domain::{ContextSummary, DomainError};
#[cfg(feature = "fastembed-engine")]
use infrastructure::FastEmbedEngine;

use infrastructure::{
    check_service_availability, RemoteVectorStore, SimpleEmbedEngine, SledVectorStore,
};

#[cfg(feature = "mcp-server")]
use interfaces::mcp::{McpEndpointMetadata, McpRuntime, McpServerConfig};

use service_manager::ServiceManager;
use settings::{available_backends, ConfigManager, EmbeddingBackend};
#[cfg(feature = "mcp-server")]
use tracing::info;

/// Global state shared with Tauri commands.
struct AppState {
    service: Arc<RwLock<Arc<ContextService>>>,
    store: Arc<dyn VectorStore>,
    config: Arc<ConfigManager>,
    service_manager: Arc<ServiceManager>,
}

impl AppState {
    fn new(handles: AppHandles, service_manager: Arc<ServiceManager>) -> Self {
        Self {
            service: Arc::new(RwLock::new(handles.service)),
            store: handles.store,
            config: handles.config,
            service_manager,
        }
    }

    fn service(&self) -> Arc<ContextService> {
        Arc::clone(&self.service.read())
    }

    fn service_cell(&self) -> Arc<RwLock<Arc<ContextService>>> {
        Arc::clone(&self.service)
    }

    fn store(&self) -> Arc<dyn VectorStore> {
        Arc::clone(&self.store)
    }

    fn config(&self) -> Arc<ConfigManager> {
        Arc::clone(&self.config)
    }
}

pub struct AppHandles {
    pub service: Arc<ContextService>,
    pub store: Arc<dyn VectorStore>,
    pub config: Arc<ConfigManager>,
    pub data_dir: std::path::PathBuf,
}

#[tauri::command]
async fn ingest_context(
    state: State<'_, AppState>,
    payload: IngestContextRequest,
) -> Result<ContextSummary, String> {
    let service = state.service();
    tauri::async_runtime::spawn_blocking(move || service.ingest(payload))
        .await
        .map_err(|err| err.to_string())?
        .map_err(map_domain_error)
}

#[tauri::command]
async fn search_contexts(
    state: State<'_, AppState>,
    payload: SearchRequest,
) -> Result<SearchResponse, String> {
    let service = state.service();
    tauri::async_runtime::spawn_blocking(move || service.search(payload))
        .await
        .map_err(|err| err.to_string())?
        .map_err(map_domain_error)
}

#[tauri::command]
async fn recent_contexts(
    state: State<'_, AppState>,
    project: Option<String>,
    limit: Option<usize>,
) -> Result<SummaryListResponse, String> {
    let service = state.service();
    tauri::async_runtime::spawn_blocking(move || service.history(project, limit))
        .await
        .map_err(|err| err.to_string())?
        .map_err(map_domain_error)
}

#[tauri::command]
async fn list_projects(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let service = state.service();
    tauri::async_runtime::spawn_blocking(move || service.projects())
        .await
        .map_err(|err| err.to_string())?
        .map_err(map_domain_error)
}

#[tauri::command]
async fn health(state: State<'_, AppState>) -> Result<HealthStatusResponse, String> {
    let service = state.service();
    tauri::async_runtime::spawn_blocking(move || service.health())
        .await
        .map_err(|err| err.to_string())?
        .map_err(map_domain_error)
}

#[tauri::command]
async fn embedding_backends(
    state: State<'_, AppState>,
) -> Result<EmbeddingBackendListResponse, String> {
    let active = state.config().current().embedding;
    let service = state.service();
    Ok(build_backend_response(active, service))
}

#[tauri::command]
async fn set_embedding_backend(
    state: State<'_, AppState>,
    payload: UpdateEmbeddingBackendRequest,
) -> Result<EmbeddingBackendListResponse, String> {
    let service_cell = state.service_cell();
    let store = state.store();
    let config = state.config();

    tauri::async_runtime::spawn_blocking(move || -> Result<EmbeddingBackendListResponse> {
        let base_backend = EmbeddingBackend::with_default_model(&payload.backend_id)
            .ok_or_else(|| anyhow!(format!("unknown backend '{}'", payload.backend_id)))?;
        let backend = apply_model_override(base_backend, payload.model_override);

        let (embedder, service_config) = init_embedder(&backend)?;
        let new_service = Arc::new(ContextService::new(
            embedder,
            Arc::clone(&store),
            service_config,
        ));

        let updated = config.set_backend(backend).map_err(|err| anyhow!(err))?;

        {
            let mut guard = service_cell.write();
            *guard = Arc::clone(&new_service);
        }

        Ok(build_backend_response(updated.embedding, new_service))
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

/// Entry point invoked from `main.rs`.
pub fn run() {
    #[cfg(feature = "mcp-server")]
    init_tracing();

    if let Err(err) = try_run() {
        eprintln!("[ingat] startup failed: {err:?}");
    }
}

fn try_run() -> Result<()> {
    // Initialize the service manager
    let service_manager = Arc::new(ServiceManager::new());

    // Check if service is already running before attempting to start
    let host = std::env::var("INGAT_SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("INGAT_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3200);

    if !check_service_availability(&host, port) {
        eprintln!("[ingat] No running mcp-service detected");
        eprintln!("[ingat] Attempting to start mcp-service as a detached background process...");

        // Attempt to start the mcp-service as a detached background process
        if let Err(e) = service_manager.start() {
            eprintln!(
                "[ingat] Warning: Could not auto-start mcp-service: {}",
                e
            );
            eprintln!("[ingat] Continuing in local database mode");
        } else {
            // Give the service a moment to start
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    } else {
        eprintln!("[ingat] mcp-service is already running - will use remote mode");
    }

    let handles = build_environment().context("failed to bootstrap Ingat environment")?;
    let app_state = AppState::new(handles, Arc::clone(&service_manager));

    #[cfg(feature = "mcp-server")]
    let mcp_runtime = {
        let service_cell = app_state.service_cell();

        let runtime = tauri::async_runtime::block_on(McpRuntime::start(service_cell, None))
            .context("failed to start MCP runtime")?;

        log_mcp_startup(runtime.metadata());
        runtime
    };

    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            ingest_context,
            search_contexts,
            recent_contexts,
            list_projects,
            health,
            embedding_backends,
            set_embedding_backend,
            service_status,
            start_service,
            stop_service
        ])
        .run(tauri::generate_context!())
        .map_err(|err| anyhow!(err));

    #[cfg(feature = "mcp-server")]
    {
        mcp_runtime.shutdown();
    }

    // Service runs detached, so it persists after UI closes
    // This is intentional - it allows the service to serve multiple clients

    run_result?;

    Ok(())
}

#[tauri::command]
async fn service_status(state: State<'_, AppState>) -> Result<ServiceStatusResponse, String> {
    let manager = &state.service_manager;

    Ok(ServiceStatusResponse {
        is_running: manager.is_running(),
        service_url: manager.service_url(),
    })
}

#[tauri::command]
async fn start_service(state: State<'_, AppState>) -> Result<String, String> {
    let manager = &state.service_manager;

    manager.start().map_err(|e| e.to_string())?;

    Ok(format!("Service started at {}", manager.service_url()))
}

#[tauri::command]
async fn stop_service(state: State<'_, AppState>) -> Result<String, String> {
    let manager = &state.service_manager;

    manager.stop();

    Ok("Service stopped".to_string())
}

#[derive(serde::Serialize)]
struct ServiceStatusResponse {
    is_running: bool,
    service_url: String,
}

#[cfg(feature = "mcp-server")]
pub async fn run_mcp_bridge(config: Option<McpServerConfig>) -> Result<()> {
    init_tracing();

    let handles = build_environment().context("failed to bootstrap Ingat environment")?;
    let service_cell = Arc::new(RwLock::new(handles.service));

    let runtime = McpRuntime::start(service_cell, config)
        .await
        .context("failed to start MCP runtime")?;

    log_mcp_startup(runtime.metadata());
    info!(
        target: "memorust::mcp",
        "Standalone bridge running. Press Ctrl+C to exit."
    );

    tokio::signal::ctrl_c()
        .await
        .context("failed to listen for shutdown signal")?;

    runtime.shutdown();
    Ok(())
}

/// Run MCP server using stdio transport (stdin/stdout).
/// This is designed for VS Code, Cursor, Windsurf, and other IDEs that spawn MCP processes.
#[cfg(feature = "mcp-server")]
pub async fn run_mcp_stdio() -> Result<()> {
    init_tracing();

    let handles = build_environment().context("failed to bootstrap Ingat environment")?;
    let service_cell = Arc::new(RwLock::new(handles.service));

    info!(
        target: "memorust::mcp",
        "Starting MCP stdio server (stdin/stdout transport)..."
    );

    interfaces::mcp::run_mcp_stdio_server(service_cell)
        .await
        .context("MCP stdio server failed")?;

    Ok(())
}

#[cfg(feature = "mcp-server")]
fn log_mcp_startup(metadata: &McpEndpointMetadata) {
    let sse_url = metadata.sse_url();
    let post_url = metadata.post_url();
    info!(
        target: "memorust::mcp",
        bind = %metadata.bind_addr,
        sse = %sse_url,
        post = %post_url,
        "MCP runtime listening"
    );
}

#[cfg(feature = "mcp-server")]
fn init_tracing() {
    init_tracing_with_writer(std::io::stderr);
}

#[cfg(feature = "mcp-server")]
fn init_tracing_with_writer<W>(make_writer: fn() -> W)
where
    W: std::io::Write + Send + Sync + 'static,
{
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();

    let _ = INIT.get_or_init(|| {
        let filter = std::env::var("INGAT_LOG").unwrap_or_else(|_| "info".into());
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_writer(make_writer)
            .compact()
            .try_init();
    });
}

pub fn build_environment() -> Result<AppHandles> {
    // Check if mcp-service is running
    let host = std::env::var("INGAT_SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("INGAT_SERVICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3200);

    eprintln!("[ingat] Checking for mcp-service at {}:{}...", host, port);

    if check_service_availability(&host, port) {
        eprintln!(
            "[ingat] ✓ Detected running mcp-service at {}:{}",
            host, port
        );
        eprintln!("[ingat] ✓ Using REMOTE MODE - all operations will proxy to the service");
        eprintln!("[ingat] ✓ No local database lock will be acquired");
        return build_environment_remote(&host, port);
    }

    eprintln!("[ingat] ✗ No mcp-service detected at {}:{}", host, port);
    eprintln!("[ingat] → Using LOCAL MODE - will open database directly");
    eprintln!("[ingat] → This may conflict if mcp-service starts later");
    build_environment_local()
}

/// Build environment using local database
fn build_environment_local() -> Result<AppHandles> {
    let data_dir = resolve_data_dir()?;

    let config = Arc::new(ConfigManager::load(&data_dir).context("failed to load config file")?);
    let active_config = config.current();

    let store_path = data_dir.join("store");
    std::fs::create_dir_all(&store_path).context("failed to create store directory")?;
    let store_impl = SledVectorStore::open(&store_path)
        .map_err(|err| anyhow!(err.to_string()))
        .context("failed to open embedded store")?;
    let store: Arc<dyn VectorStore> = Arc::new(store_impl);

    let (embedder, service_config) = init_embedder(&active_config.embedding)
        .context("failed to initialise embedding backend")?;
    let service = Arc::new(ContextService::new(
        embedder,
        Arc::clone(&store),
        service_config,
    ));

    Ok(AppHandles {
        service,
        store,
        config,
        data_dir,
    })
}

/// Build environment using remote mcp-service
fn build_environment_remote(host: &str, port: u16) -> Result<AppHandles> {
    let data_dir = resolve_data_dir()?;

    let config = Arc::new(ConfigManager::load(&data_dir).context("failed to load config file")?);
    let active_config = config.current();

    // Use remote implementations
    let store: Arc<dyn VectorStore> = Arc::new(RemoteVectorStore::new(host, port));

    // Use a dummy embedder since embedding happens on the remote service
    let (embedder, service_config) = init_embedder(&active_config.embedding)
        .context("failed to initialise embedding backend")?;

    let service = Arc::new(ContextService::new(
        embedder,
        Arc::clone(&store),
        service_config,
    ));

    Ok(AppHandles {
        service,
        store,
        config,
        data_dir,
    })
}

fn init_embedder(
    backend: &EmbeddingBackend,
) -> Result<(
    Arc<dyn EmbeddingEngineTrait>,
    application::services::ServiceConfig,
)> {
    let default_limit = application::services::ServiceConfig::default().default_limit;
    match backend {
        EmbeddingBackend::Simple { model, dimensions } => {
            let engine = SimpleEmbedEngine::try_new(model.clone(), *dimensions)
                .map_err(|err| anyhow!(err.to_string()))?;
            let config = application::services::ServiceConfig::new(model.clone(), default_limit);
            Ok((Arc::new(engine), config))
        }
        #[cfg(feature = "fastembed-engine")]
        EmbeddingBackend::FastEmbed { model } => {
            let engine = FastEmbedEngine::try_new(model).map_err(|err| anyhow!(err.to_string()))?;
            let config = application::services::ServiceConfig::new(model.clone(), default_limit);
            Ok((Arc::new(engine), config))
        }
    }
}

fn build_backend_response(
    active: EmbeddingBackend,
    service: Arc<ContextService>,
) -> EmbeddingBackendListResponse {
    let mut options: Vec<EmbeddingBackendOption> = available_backends()
        .into_iter()
        .map(|backend| EmbeddingBackendOption {
            id: backend.id().to_string(),
            label: backend.label().to_string(),
            description: backend.description().to_string(),
            model: backend.model_name().to_string(),
            dimensions: backend.expected_dimensions(),
            feature_gated: backend.is_feature_gated(),
        })
        .collect();

    if let Some(option) = options.iter_mut().find(|opt| opt.id == active.id()) {
        option.model = active.model_name().to_string();
        option.dimensions = service
            .embedding_dimensions()
            .or_else(|| active.expected_dimensions());
    }

    EmbeddingBackendListResponse {
        active: active.id().to_string(),
        options,
    }
}

fn apply_model_override(
    mut backend: EmbeddingBackend,
    model_override: Option<String>,
) -> EmbeddingBackend {
    if let Some(model) = model_override.and_then(|m| {
        let trimmed = m.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }) {
        match &mut backend {
            EmbeddingBackend::Simple {
                model: backend_model,
                ..
            } => *backend_model = model,
            #[cfg(feature = "fastembed-engine")]
            EmbeddingBackend::FastEmbed {
                model: backend_model,
            } => *backend_model = model,
        }
    }
    backend
}

fn resolve_data_dir() -> Result<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "ingat", "Ingat")
        .ok_or_else(|| anyhow!("unable to determine OS data dir"))?;
    let dir = dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&dir).context("failed to create data directory")?;
    Ok(dir)
}

fn map_domain_error(err: DomainError) -> String {
    err.to_string()
}
