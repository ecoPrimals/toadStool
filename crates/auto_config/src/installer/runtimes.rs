// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime setup: container (Docker) and GPU support

use std::path::Path;

use tokio::fs;
use tokio::process::Command as AsyncCommand;
use tracing::info;

use crate::ToadStoolError;

/// Setup container runtime support (Docker)
pub async fn setup_container_runtime(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🐳 Setting up container runtime support...");

    // Verify Docker is working
    if let Ok(output) = AsyncCommand::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .await
        && output.status.success()
    {
        let version = String::from_utf8_lossy(&output.stdout);
        info!("🐳 Docker version: {}", version.trim());
    }

    // Create Docker configuration if needed
    let docker_config_dir = installation_path.join("config").join("docker");
    if !docker_config_dir.exists() {
        fs::create_dir_all(&docker_config_dir).await?;

        let docker_config = serde_json::json!({
            "default_runtime": "runc",
            "runtimes": {
                "runc": {
                    "path": "runc"
                }
            },
            "storage_driver": "overlay2"
        });

        fs::write(
            docker_config_dir.join("daemon.json"),
            serde_json::to_string_pretty(&docker_config)?,
        )
        .await?;
    }

    Ok(())
}

/// Setup GPU runtime support (NVIDIA)
pub async fn setup_gpu_runtime(installation_path: &Path) -> Result<(), ToadStoolError> {
    info!("🎮 Setting up GPU runtime support...");

    if let Ok(output) = AsyncCommand::new("nvidia-smi")
        .arg("--version")
        .output()
        .await
        && output.status.success()
    {
        info!("🎮 NVIDIA GPU runtime detected");

        let gpu_config_dir = installation_path.join("config").join("gpu");
        if !gpu_config_dir.exists() {
            fs::create_dir_all(&gpu_config_dir).await?;

            let nvidia_config = serde_json::json!({
                "runtime": "nvidia",
                "memory_fraction": 0.8,
                "compute_mode": "default"
            });

            fs::write(
                gpu_config_dir.join("nvidia.json"),
                serde_json::to_string_pretty(&nvidia_config)?,
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_setup_container_runtime() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let result = setup_container_runtime(path).await;
        assert!(result.is_ok());

        let docker_config_dir = path.join("config").join("docker");
        if docker_config_dir.exists() {
            let daemon_json = docker_config_dir.join("daemon.json");
            assert!(daemon_json.exists());
            let content = tokio::fs::read_to_string(&daemon_json).await.unwrap();
            assert!(content.contains("runc"));
            assert!(content.contains("overlay2"));
        }
    }

    #[tokio::test]
    async fn test_setup_container_runtime_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let r1 = setup_container_runtime(path).await;
        let r2 = setup_container_runtime(path).await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn test_setup_gpu_runtime() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let result = setup_gpu_runtime(path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_setup_gpu_runtime_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let r1 = setup_gpu_runtime(path).await;
        let r2 = setup_gpu_runtime(path).await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }
}
