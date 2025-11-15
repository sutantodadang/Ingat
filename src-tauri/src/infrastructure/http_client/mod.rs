//! HTTP client for communicating with the mcp-service backend.
//!
//! This module provides a client implementation that proxies storage and service
//! operations to a running mcp-service instance via HTTP, eliminating the need
//! for direct database access and avoiding lock conflicts.

mod remote_store;

pub use remote_store::RemoteVectorStore;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Default service host
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Default service port
pub const DEFAULT_PORT: u16 = 3200;

/// Health check response from the service
#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
}

/// Check if the mcp-service is running and accessible
pub fn check_service_availability(host: &str, port: u16) -> bool {
    let url = format!("http://{}:{}/health", host, port);

    eprintln!("[ingat::http_client] Checking service at: {}", url);

    match ureq::get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .call()
    {
        Ok(response) => {
            eprintln!(
                "[ingat::http_client] Response status: {}",
                response.status()
            );
            if response.status() == 200 {
                if let Ok(health) = response.into_json::<HealthResponse>() {
                    let is_healthy = health.status == "healthy";
                    eprintln!(
                        "[ingat::http_client] Service health: {} (healthy={})",
                        health.status, is_healthy
                    );
                    return is_healthy;
                } else {
                    eprintln!("[ingat::http_client] Failed to parse health response");
                }
            }
            false
        }
        Err(e) => {
            eprintln!("[ingat::http_client] Connection failed: {}", e);
            false
        }
    }
}

/// Get the service base URL
pub fn get_service_url(host: &str, port: u16) -> String {
    format!("http://{}:{}", host, port)
}

/// Error response from the service
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Convert HTTP errors to anyhow errors
pub fn handle_http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => {
            if let Ok(err_response) = response.into_json::<ErrorResponse>() {
                anyhow::anyhow!(
                    "HTTP {} - {}: {}",
                    code,
                    err_response.code,
                    err_response.error
                )
            } else {
                anyhow::anyhow!("HTTP error: {}", code)
            }
        }
        ureq::Error::Transport(transport) => {
            anyhow::anyhow!("Transport error: {}", transport)
        }
    }
}
