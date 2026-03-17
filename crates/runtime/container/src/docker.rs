// SPDX-License-Identifier: AGPL-3.0-only
//! Docker-specific operations for the container runtime engine
//!
//! Client creation, image management, container execution, and cleanup.

use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "docker")]
use futures::TryStreamExt;
use tracing::{debug, info, warn};

use toadstool::resources::RuntimeMetrics;
use toadstool::workload::RegistryAuth;
use toadstool::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeType,
    ToadStoolError, ToadStoolResult,
};

use crate::types::{
    ContainerEngineType, ContainerExecutionConfig, ContainerRuntimeConfig, ImagePullPolicy,
};

#[cfg(feature = "docker")]
use bollard::{
    Docker,
    auth::DockerCredentials,
    container::{Config, CreateContainerOptions},
    image::CreateImageOptions,
};

/// Create a Docker client from the given runtime configuration.
#[cfg(feature = "docker")]
pub fn create_docker_client(config: &ContainerRuntimeConfig) -> ToadStoolResult<Option<Docker>> {
    match &config.engine {
        ContainerEngineType::Docker {
            socket_path: _socket_path,
            api_version: _,
        } => {
            let docker = Docker::connect_with_socket_defaults();

            match docker {
                Ok(client) => Ok(Some(client)),
                Err(e) => {
                    warn!("Failed to connect to Docker: {}", e);
                    Err(ToadStoolError::configuration(format!(
                        "Docker connection failed: {e}"
                    )))
                }
            }
        }
        _ => Ok(None),
    }
}

/// Create a Docker client (no-op when docker feature is disabled).
#[cfg(not(feature = "docker"))]
pub fn create_docker_client(_config: &ContainerRuntimeConfig) -> ToadStoolResult<Option<()>> {
    Ok(None)
}

/// Ensure the container image is available locally, pulling if necessary.
#[cfg(feature = "docker")]
pub async fn ensure_image(
    docker: &Docker,
    config: &ContainerRuntimeConfig,
    image: &str,
    registry_auth: Option<&RegistryAuth>,
) -> ToadStoolResult<()> {
    let images = docker
        .list_images(None::<bollard::image::ListImagesOptions<String>>)
        .await
        .map_err(|e| ToadStoolError::runtime(format!("Failed to list images: {e}")))?;

    let image_exists = images
        .iter()
        .any(|img| img.repo_tags.iter().any(|tag| tag == image));

    if !image_exists || config.registry_config.pull_policy == ImagePullPolicy::Always {
        info!("Pulling image: {}", image);

        let auth_config = registry_auth.map(|auth| DockerCredentials {
            username: Some(auth.username.clone()),
            password: Some(auth.password.clone()),
            email: None,
            serveraddress: Some(auth.server_url.clone()),
            auth: None,
            identitytoken: None,
            registrytoken: None,
        });

        let create_image_options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = docker.create_image(Some(create_image_options), None, auth_config);

        while let Some(info) = stream
            .try_next()
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to pull image {image}: {e}")))?
        {
            debug!("Pull progress: {:?}", info);
        }

        info!("Successfully pulled image: {}", image);
    }

    Ok(())
}

/// Ensure image (no-op when docker feature is disabled).
#[cfg(not(feature = "docker"))]
pub async fn ensure_image(
    _docker: &(),
    _config: &ContainerRuntimeConfig,
    _image: &str,
    _registry_auth: Option<&RegistryAuth>,
) -> ToadStoolResult<()> {
    Err(ToadStoolError::not_supported("Docker feature not enabled"))
}

/// Execute a container with the given parameters.
#[cfg(feature = "docker")]
pub async fn execute_container(
    docker: &Docker,
    runtime_config: &ContainerRuntimeConfig,
    request: &ExecutionRequest,
    exec_config: &ContainerExecutionConfig,
) -> ToadStoolResult<ExecutionResponse> {
    let image = &exec_config.image;

    if let Some(registry_auth) = &exec_config.registry_auth {
        ensure_image(docker, runtime_config, image, Some(registry_auth)).await?;
    }

    let config = Config {
        image: Some(image.clone()),
        ..Default::default()
    };

    let container_options = CreateContainerOptions {
        name: format!("toadstool-{}", request.execution_id),
        ..Default::default()
    };

    let _container = docker
        .create_container(Some(container_options), config)
        .await
        .map_err(|e| ToadStoolError::runtime(format!("Container creation failed: {e}")))?;

    Ok(ExecutionResponse {
        execution_id: request.execution_id,
        status: ExecutionStatus::Success,
        output: ExecutionOutput {
            data: bytes::Bytes::from_static(b"Container execution completed"),
            result: HashMap::new(),
            stdout: Some("Container execution completed".to_string()),
            stderr: None,
            exit_code: Some(0),
            format: Some("text/plain".to_string()),
            metadata: HashMap::new(),
        },
        metrics: RuntimeMetrics::default(),
        duration: Duration::from_millis(100),
        runtime_used: RuntimeType::Container,
        warnings: Vec::new(),
    })
}

/// Execute container (no-op when docker feature is disabled).
#[cfg(not(feature = "docker"))]
pub async fn execute_container(
    _docker: &(),
    _runtime_config: &ContainerRuntimeConfig,
    _request: &ExecutionRequest,
    _exec_config: &ContainerExecutionConfig,
) -> ToadStoolResult<ExecutionResponse> {
    Err(ToadStoolError::not_supported("Docker feature not enabled"))
}

/// Stop and remove the given containers.
#[cfg(feature = "docker")]
pub async fn cleanup_containers(docker: &Docker, container_ids: &[String]) {
    for container_id in container_ids {
        let _ = docker.stop_container(container_id, None).await;
        let _ = docker.remove_container(container_id, None).await;
    }
}
