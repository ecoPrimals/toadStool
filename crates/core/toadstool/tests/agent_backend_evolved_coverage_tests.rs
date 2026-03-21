// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::pedantic)]
//! Coverage tests for `biomeos_integration::agent_backend_evolved`
//! Target: agent_backend_evolved.rs — capability-based agent deployment backend

use toadstool::biomeos_integration::agent_backend_evolved::{
    AgentBackend, AgentBackendError, AgentInfo, AgentStatus, DeployAgentRequest, LoadModelRequest,
    ModelInfo, ModelStatus,
};

// ─── AgentBackend construction ───────────────────────────────────────────────

#[test]
fn test_agent_backend_new_and_default() {
    let _backend = AgentBackend::new();
    let _backend = AgentBackend::default();
}

#[tokio::test]
async fn test_agent_backend_is_available_without_provider() {
    let backend = AgentBackend::new();
    assert!(!backend.is_available().await);
}

#[tokio::test]
async fn test_agent_backend_provider_info_none_without_provider() {
    let backend = AgentBackend::new();
    assert!(backend.provider_info().await.is_none());
}

// ─── Error paths (no provider) ────────────────────────────────────────────────

#[tokio::test]
async fn test_deploy_agent_fails_without_provider() {
    let backend = AgentBackend::new();
    let req = DeployAgentRequest {
        name: "test".to_string(),
        model: "gpt-4".to_string(),
        replicas: 1,
        capabilities: vec![],
    };
    let result = backend.deploy_agent(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_model_fails_without_provider() {
    let backend = AgentBackend::new();
    let req = LoadModelRequest {
        name: "m".to_string(),
        model_type: "transformer".to_string(),
        source: "s3://x".to_string(),
    };
    let result = backend.load_model(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_scale_agent_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.scale_agent("agent-1", 2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_agent_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.stop_agent("agent-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_remove_agent_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.remove_agent("agent-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_agent_status_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.get_agent_status("agent-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_agents_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.list_agents().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_models_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.list_models().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unload_model_fails_without_provider() {
    let backend = AgentBackend::new();
    let result = backend.unload_model("model-1").await;
    assert!(result.is_err());
}

// ─── AgentBackendError variants ───────────────────────────────────────────────

#[test]
fn test_agent_backend_error_no_agent_provider() {
    let err = AgentBackendError::NoAgentProvider;
    assert!(err.to_string().contains("provider"));
}

#[test]
fn test_agent_backend_error_deployment_failed() {
    let err = AgentBackendError::DeploymentFailed("failed".to_string());
    assert!(err.to_string().contains("deployment"));
}

#[test]
fn test_agent_backend_error_model_load_failed() {
    let err = AgentBackendError::ModelLoadFailed("load err".to_string());
    assert!(err.to_string().contains("Model loading"));
}

#[test]
fn test_agent_backend_error_scaling_failed() {
    let err = AgentBackendError::ScalingFailed("scale err".to_string());
    assert!(err.to_string().contains("scaling"));
}

#[test]
fn test_agent_backend_error_agent_not_found() {
    let err = AgentBackendError::AgentNotFound("agent-x".to_string());
    assert!(err.to_string().contains("agent-x"));
}

#[test]
fn test_agent_backend_error_model_not_found() {
    let err = AgentBackendError::ModelNotFound("m1".to_string());
    assert!(err.to_string().contains("Model not found"));
}

#[test]
fn test_agent_backend_error_termination_failed() {
    let err = AgentBackendError::TerminationFailed("term err".to_string());
    assert!(err.to_string().contains("termination"));
}

#[test]
fn test_agent_backend_error_capability_from_capability_error() {
    use toadstool_common::capability_provider::CapabilityError;
    use toadstool_common::primal_identity::{Capability, ComputeCapability};
    let cap_err =
        CapabilityError::NoProviderFound(Capability::Compute(ComputeCapability::NativeExecution));
    let agent_err: AgentBackendError = cap_err.into();
    assert!(!agent_err.to_string().is_empty());
}

#[test]
fn test_agent_backend_error_json_from_serde_error() {
    let json_err = serde_json::from_str::<AgentInfo>("invalid").unwrap_err();
    let agent_err: AgentBackendError = json_err.into();
    assert!(!agent_err.to_string().is_empty());
}

// ─── AgentStatus and ModelStatus ──────────────────────────────────────────────

#[test]
fn test_agent_status_all_variants() {
    let _ = AgentStatus::Deploying;
    let _ = AgentStatus::Running;
    let _ = AgentStatus::Scaling;
    let _ = AgentStatus::Stopped;
    let _ = AgentStatus::Failed;
}

#[test]
fn test_agent_status_serde() {
    let status = AgentStatus::Running;
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json, "running");
    let parsed: AgentStatus = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, AgentStatus::Running);
}

#[test]
fn test_model_status_all_variants() {
    let _ = ModelStatus::Loading;
    let _ = ModelStatus::Ready;
    let _ = ModelStatus::Unloading;
    let _ = ModelStatus::Error;
}

#[test]
fn test_model_status_serde() {
    let status = ModelStatus::Ready;
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json, "ready");
}

// ─── AgentInfo and ModelInfo ──────────────────────────────────────────────────

#[test]
fn test_agent_info_serialization() {
    let info = AgentInfo {
        id: "a1".to_string(),
        name: "test".to_string(),
        model: "gpt-4".to_string(),
        status: AgentStatus::Running,
        replicas: 2,
        capabilities: vec!["inference".to_string()],
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["id"], "a1");
    assert_eq!(json["replicas"], 2);
}

#[test]
fn test_model_info_serialization() {
    let info = ModelInfo {
        id: "m1".to_string(),
        name: "gpt-4".to_string(),
        model_type: "transformer".to_string(),
        size_bytes: 1_000_000_000,
        status: ModelStatus::Ready,
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["id"], "m1");
}

// ─── Request types ─────────────────────────────────────────────────────────────

#[test]
fn test_deploy_agent_request_serialization() {
    let req = DeployAgentRequest {
        name: "deploy".to_string(),
        model: "gpt-4".to_string(),
        replicas: 3,
        capabilities: vec!["inference".to_string()],
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "deploy");
}

#[test]
fn test_load_model_request_serialization() {
    let req = LoadModelRequest {
        name: "m".to_string(),
        model_type: "transformer".to_string(),
        source: "s3://bucket/model".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["source"], "s3://bucket/model");
}
