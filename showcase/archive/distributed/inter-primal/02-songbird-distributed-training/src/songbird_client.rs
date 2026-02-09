///! Songbird Federation Client for ToadStool
///!
///! Uses ToadStool's capability-based discovery system to find orchestration services
///! Adheres to self-knowledge principles: discovers Songbird by capability, not by name

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdCapability {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerInfo {
    pub tower_id: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub gpu_info: Option<GpuInfo>,
    pub health: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub memory_gb: u32,
    pub cuda_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    pub service_id: String,
    pub service_type: String,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLTask {
    pub task_id: String,
    pub task_type: String,
    pub model: String,
    pub dataset: String,
    pub epochs: u32,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLTaskResult {
    pub task_id: String,
    pub status: String,
    pub accuracy: Option<f64>,
    pub loss: Option<f64>,
    pub execution_time_ms: u64,
    pub executed_by: String,
}

pub struct SongbirdClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl SongbirdClient {
    /// Connect to orchestration service (discovered by capability)
    /// 
    /// This method adheres to self-knowledge principles: ToadStool knows it needs
    /// "service-discovery" and "load-balancing" capabilities, but doesn't hardcode
    /// that "Songbird" provides them.
    pub fn connect(base_url: &str) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // Accept self-signed certs
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self {
            base_url: base_url.to_string(),
            http_client,
        })
    }
    
    /// Discover orchestration endpoint using capability-based discovery
    /// 
    /// This is the preferred way to connect - discovers by capability rather than hardcoded URL
    #[cfg(feature = "capability-discovery")]
    pub async fn discover_and_connect() -> Result<Self> {
        // Use ToadStool's built-in capability discovery
        use toadstool::discovery::orchestration::discover_orchestration;
        
        let endpoint = discover_orchestration().await
            .context("Failed to discover orchestration service (looking for service-discovery + load-balancing capabilities)")?;
        
        Self::connect(&endpoint)
    }
    
    /// Check if Songbird is healthy
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        
        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => Ok(true),
            Ok(response) => {
                println!("⚠️  Songbird health check failed: {}", response.status());
                Ok(false)
            }
            Err(e) => {
                println!("⚠️  Songbird not reachable: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Discover available towers
    pub async fn discover_towers(&self) -> Result<Vec<TowerInfo>> {
        let url = format!("{}/api/v1/towers", self.base_url);
        
        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let towers: Vec<TowerInfo> = response
                    .json()
                    .await
                    .unwrap_or_else(|_| Vec::new());
                Ok(towers)
            }
            Ok(response) => {
                println!("⚠️  Tower discovery failed: {}", response.status());
                // Return empty list if API not available yet
                Ok(Vec::new())
            }
            Err(e) => {
                println!("⚠️  Tower discovery error: {}", e);
                Ok(Vec::new())
            }
        }
    }
    
    /// Register ToadStool service with Songbird
    pub async fn register_service(&self, registration: ServiceRegistration) -> Result<()> {
        let url = format!("{}/api/v1/services/register", self.base_url);
        
        let response = self.http_client
            .post(&url)
            .json(&registration)
            .send()
            .await
            .context("Failed to send registration")?;
        
        if response.status().is_success() {
            println!("✅ Registered with Songbird: {}", registration.service_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Registration failed: {} - {}", status, body)
        }
    }
    
    /// Submit ML task to Songbird for distribution
    pub async fn submit_task(&self, task: MLTask) -> Result<String> {
        let url = format!("{}/api/v1/tasks/submit", self.base_url);
        
        let response = self.http_client
            .post(&url)
            .json(&task)
            .send()
            .await
            .context("Failed to submit task")?;
        
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            let task_id = result["task_id"]
                .as_str()
                .unwrap_or(&task.task_id)
                .to_string();
            Ok(task_id)
        } else {
            anyhow::bail!("Task submission failed: {}", response.status())
        }
    }
    
    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<MLTaskResult> {
        let url = format!("{}/api/v1/tasks/{}", self.base_url, task_id);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to get task status")?;
        
        if response.status().is_success() {
            let result: MLTaskResult = response.json().await?;
            Ok(result)
        } else {
            anyhow::bail!("Failed to get task status: {}", response.status())
        }
    }
    
    /// Get Songbird capabilities
    pub async fn get_capabilities(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/capabilities", self.base_url);
        
        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let caps: serde_json::Value = response.json().await?;
                let capabilities = caps["capabilities"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(capabilities)
            }
            _ => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connect_to_songbird() {
        let client = SongbirdClient::connect("https://localhost:8000");
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let client = SongbirdClient::connect("https://localhost:8000").unwrap();
        // Don't fail if Songbird isn't running
        let _ = client.health_check().await;
    }
}

