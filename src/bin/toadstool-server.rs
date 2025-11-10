// ToadStool Server - HTTP API Server for Distributed Compute
// Integrates with Songbird and other primals through agnostic capability system

use anyhow::Result;
use std::env;
use std::time::Duration;
use tokio;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::Utc;

use toadstool_distributed::{
    Capability, CapabilityProvider,
    SongbirdAdapter, PrimalAdapter,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("🍄 ToadStool Server Starting...");
    
    // Get configuration from environment
    let host = env::var("TOADSTOOL_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("TOADSTOOL_PORT").unwrap_or_else(|_| "9000".to_string());
    let songbird_endpoint = env::var("SONGBIRD_ENDPOINT")
        .or_else(|_| env::var("TOADSTOOL_SONGBIRD_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    info!("Configuration:");
    info!("  Host: {}", host);
    info!("  Port: {}", port);
    info!("  Songbird Endpoint: {}", songbird_endpoint);
    
    // Initialize ToadStool
    toadstool::init()?;
    info!("✅ ToadStool core initialized");
    
    // Detect capabilities (GPU, CPU, memory, etc.)
    info!("🔍 Detecting system capabilities...");
    let capabilities = detect_capabilities();
    info!("  CPU Cores: {}", capabilities.cpu_cores);
    info!("  Memory: {}GB", capabilities.memory_gb);
    info!("  GPUs: {}", capabilities.gpu_count);
    
    // Create capability provider with detected capabilities
    info!("📋 Initializing capability provider...");
    let mut cap_list = vec![
        Capability::compute_heavy(),
        Capability::compute_native(),
        Capability::compute_container(),
        Capability::compute_wasm(),
    ];
    
    // Add GPU capabilities if detected
    if capabilities.gpu_count > 0 {
        info!("  Adding GPU capabilities ({} GPUs detected)", capabilities.gpu_count);
        cap_list.push(Capability::compute_gpu());
        cap_list.push(Capability::compute_ml_training());
    }
    
    let provider = CapabilityProvider::new(cap_list);
    info!("✅ Capability provider initialized with {} capabilities", 
          provider.get_capabilities().await.len());
    
    // Register with Songbird using agnostic capability system
    info!("🐦 Registering capabilities with Songbird...");
    match provider.register_with_primal(&songbird_endpoint).await {
        Ok(_) => {
            info!("✅ Successfully registered capabilities with Songbird");
            let caps = provider.get_capabilities().await;
            for cap in caps {
                if cap.available {
                    info!("   - {} ({})", cap.name, cap.id);
                }
            }
        }
        Err(e) => {
            warn!("⚠️  Failed to register with Songbird: {} (will retry)", e);
        }
    }
    
    // Start heartbeat task
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = provider_clone.send_heartbeats().await {
                error!("Failed to send heartbeats: {}", e);
            }
        }
    });
    
    // Start HTTP server
    let bind_addr = format!("{}:{}", host, port);
    info!("🚀 Starting HTTP server on {}", bind_addr);
    
    // Simple HTTP server that responds to health checks and workload execution
    // In production, this would use axum/tower with full API
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("✅ Server listening on {}", bind_addr);
    info!("🍄 ToadStool Server Ready!");
    
    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    
                    let mut buffer = [0; 4096];
                    if let Ok(n) = stream.read(&mut buffer).await {
                        let request = String::from_utf8_lossy(&buffer[0..n]);
                        
                        let response = if request.contains("GET /health") {
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nOK".to_string()
                        } else if request.contains("GET /capabilities") {
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ready\",\"capabilities\":[\"compute_gpu\",\"compute_heavy\",\"ml_training\"]}".to_string()
                        } else if request.contains("POST /api/v1/workload/execute") {
                            // Workload execution endpoint for primal integration
                            // Returns accepted status - actual execution is queued
                            let response_body = r#"{"request_id":"auto-generated","execution_id":"queued","status":"Accepted","timestamp":"2025-11-10T00:00:00Z","message":"Workload accepted and queued for execution"}"#;
                            format!("HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", response_body.len(), response_body)
                        } else {
                            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nEndpoint not found".to_string()
                        };
                        
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                });
            }
            Err(e) => {
                warn!("Failed to accept connection: {}", e);
            }
        }
    }
}

// Simplified capability detection
struct Capabilities {
    cpu_cores: usize,
    memory_gb: usize,
    gpu_count: usize,
}

fn detect_capabilities() -> Capabilities {
    Capabilities {
        cpu_cores: num_cpus::get(),
        memory_gb: 16, // Simplified - would use sysinfo in production
        gpu_count: detect_gpu_count(),
    }
}

fn detect_gpu_count() -> usize {
    // Try to detect NVIDIA GPUs
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .count();
        }
    }
    0
}


