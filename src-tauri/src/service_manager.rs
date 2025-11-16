//! Service manager for auto-starting and managing the mcp-service process.
//!
//! This module handles starting the mcp-service as a child process when the
//! Tauri UI launches, and ensures it's properly shut down when the UI closes.

use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use tracing::{debug, error, info};

/// Manages the lifecycle of the mcp-service child process.
pub struct ServiceManager {
    child: Arc<Mutex<Option<Child>>>,
    port: u16,
    host: String,
}

impl ServiceManager {
    /// Create a new service manager with default settings.
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            port: Self::resolve_port(),
            host: Self::resolve_host(),
        }
    }

    /// Create a service manager with custom port and host.
    pub fn with_config(port: u16, host: String) -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            port,
            host,
        }
    }

    /// Start the mcp-service as a child process.
    ///
    /// This method will:
    /// 1. Check if the service is already running on the configured port
    /// 2. If not, spawn the mcp-service binary
    /// 3. Wait briefly to ensure it starts successfully
    pub fn start(&self) -> Result<()> {
        // Check if service is already running
        if self.is_running() {
            info!("mcp-service is already running on port {}", self.port);
            return Ok(());
        }

        // Find the mcp-service binary
        let binary_path = self.find_binary()?;

        info!("Starting mcp-service at {}", binary_path.display());
        debug!("Configuration: {}:{}", self.host, self.port);

        // Spawn the service process as a detached background process
        #[cfg(windows)]
        let child = {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            const DETACHED_PROCESS: u32 = 0x00000008;

            Command::new(&binary_path)
                .env("INGAT_SERVICE_HOST", &self.host)
                .env("INGAT_SERVICE_PORT", self.port.to_string())
                .env("INGAT_LOG", Self::resolve_log_level())
                .env("RUST_LOG_STYLE", "never")
                .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
                .spawn()
                .context("Failed to spawn mcp-service process")?
        };

        #[cfg(unix)]
        let child = {
            Command::new(&binary_path)
                .env("INGAT_SERVICE_HOST", &self.host)
                .env("INGAT_SERVICE_PORT", self.port.to_string())
                .env("INGAT_LOG", Self::resolve_log_level())
                .env("RUST_LOG_STYLE", "never")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("Failed to spawn mcp-service process")?
        };

        let pid = child.id();
        info!("mcp-service started with PID: {} (detached)", pid);

        // Don't store the child process - it's detached and will persist independently
        // This allows the UI to close without stopping the service
        drop(child);

        // Wait a moment for the service to start
        std::thread::sleep(Duration::from_millis(1500));

        // Verify it started successfully
        if !self.is_running() {
            error!(
                "mcp-service failed to start - not listening on port {}",
                self.port
            );
            return Err(anyhow::anyhow!("Service failed to start"));
        }

        info!("mcp-service is now running and accepting connections");
        Ok(())
    }

    /// Stop the mcp-service process (if it was started by this manager).
    /// Note: Since service runs detached, this is a no-op by default.
    /// The service will continue running after the UI closes.
    pub fn stop(&self) {
        // Service runs detached and persists independently
        // To stop it, user should manually kill the process or it will be cleaned up by OS
        debug!("Service runs in detached mode - will persist after UI closes");
    }

    /// Check if the service is running by attempting to connect to the health endpoint.
    pub fn is_running(&self) -> bool {
        let url = format!("http://{}:{}/health", self.host, self.port);

        // Try to connect to the health endpoint
        match ureq::get(&url).timeout(Duration::from_secs(2)).call() {
            Ok(response) => {
                debug!("Health check succeeded: {}", response.status());
                response.status() == 200
            }
            Err(e) => {
                debug!("Health check failed: {}", e);
                false
            }
        }
    }

    /// Get the service URL.
    pub fn service_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Find the mcp-service binary.
    ///
    /// Search order:
    /// 1. Next to the current executable (for bundled apps)
    /// 2. In target/release (for development)
    /// 3. In target/debug (for development)
    fn find_binary(&self) -> Result<std::path::PathBuf> {
        let binary_name = if cfg!(windows) {
            "mcp_service.exe"
        } else {
            "mcp_service"
        };

        // 1. Check next to current executable (bundled)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bundled_path = exe_dir.join(binary_name);
                if bundled_path.exists() {
                    debug!("Found mcp-service at: {}", bundled_path.display());
                    return Ok(bundled_path);
                }
            }
        }

        // 2. Check target/release (development)
        let release_path = std::path::PathBuf::from("target/release").join(binary_name);
        if release_path.exists() {
            debug!("Found mcp-service at: {}", release_path.display());
            return Ok(release_path);
        }

        // 3. Check target/debug (development)
        let debug_path = std::path::PathBuf::from("target/debug").join(binary_name);
        if debug_path.exists() {
            debug!("Found mcp-service at: {}", debug_path.display());
            return Ok(debug_path);
        }

        // 4. Check in PATH
        if let Ok(path) = which::which(binary_name) {
            debug!("Found mcp-service in PATH: {}", path.display());
            return Ok(path);
        }

        Err(anyhow::anyhow!(
            "Could not find mcp_service binary. Please ensure it's built and accessible."
        ))
    }

    /// Resolve port from environment or use default.
    fn resolve_port() -> u16 {
        std::env::var("INGAT_SERVICE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3200)
    }

    /// Resolve host from environment or use default.
    fn resolve_host() -> String {
        std::env::var("INGAT_SERVICE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
    }

    /// Resolve log level from environment or use default.
    fn resolve_log_level() -> String {
        std::env::var("INGAT_LOG").unwrap_or_else(|_| "error".to_string())
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ServiceManager {
    fn drop(&mut self) {
        // Service runs detached, so we don't stop it when UI closes
        // This allows the service to persist and serve multiple clients
        debug!("ServiceManager dropped - detached service will continue running");
    }
}

/// Check if a port is available (not in use).
#[cfg(windows)]
pub fn is_port_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Check if a port is available (not in use).
#[cfg(unix)]
pub fn is_port_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_manager_creation() {
        let manager = ServiceManager::new();
        assert_eq!(manager.port, 3200);
        assert_eq!(manager.host, "127.0.0.1");
    }

    #[test]
    fn test_custom_config() {
        let manager = ServiceManager::with_config(3201, "localhost".to_string());
        assert_eq!(manager.port, 3201);
        assert_eq!(manager.host, "localhost");
    }

    #[test]
    fn test_service_url() {
        let manager = ServiceManager::new();
        assert_eq!(manager.service_url(), "http://127.0.0.1:3200");
    }
}
