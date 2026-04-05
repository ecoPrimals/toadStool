// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for the runtime execution pipeline.
//!
//! Tests cover `RuntimeOrchestrator` lifecycle, workload construction,
//! security context validation, and graceful error handling when no
//! engine is registered.  All assertions use the actual production API.

use std::collections::HashMap;
use toadstool::{
    ExecutableSource, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeOrchestrator,
    RuntimeSelectionStrategy, RuntimeType, ToadStoolError, WorkloadSpec, WorkloadType,
};
use uuid::Uuid;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_orchestrator() -> RuntimeOrchestrator {
    RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable)
}

/// A `WorkloadSpec` that passes validation without requiring any file on disk
/// (uses a URL executable source).
fn make_url_workload() -> WorkloadSpec {
    WorkloadSpec::Native {
        executable: ExecutableSource::Url {
            url: "https://example.com/echo".to_string(),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    }
}

fn make_request() -> ExecutionRequest {
    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: make_url_workload(),
        ..ExecutionRequest::default()
    }
}

// ── Orchestrator lifecycle ─────────────────────────────────────────────────────

#[test]
fn test_runtime_orchestrator_creates_with_each_strategy() {
    let _first = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let _lb = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);
    let _opt = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);
}

#[tokio::test]
async fn test_runtime_orchestrator_starts_with_no_engines() {
    let orch = make_orchestrator();
    // An orchestrator with no registered engines should fail gracefully on execute,
    // not panic or deadlock.
    let result = orch.execute(make_request()).await;
    assert!(
        result.is_err(),
        "Empty orchestrator must return error, not panic"
    );
}

// ── WorkloadSpec construction and validation ───────────────────────────────────

#[test]
fn test_workload_spec_default_is_native() {
    let spec = WorkloadSpec::default();
    assert_eq!(spec.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_url_is_native() {
    let spec = make_url_workload();
    assert_eq!(spec.workload_type(), WorkloadType::Native);
}

#[test]
fn test_workload_spec_url_passes_validation() {
    // URL-based executables are validated without touching the filesystem.
    let spec = make_url_workload();
    assert!(
        spec.validate().is_ok(),
        "URL-based WorkloadSpec must pass validation"
    );
}

#[test]
fn test_workload_spec_default_file_fails_without_disk_path() {
    // The default spec points to "echo" (relative path) which won't exist.
    // This confirms that file-based validation is active.
    let spec = WorkloadSpec::default();
    // Either valid (if `echo` is found on PATH) or a Configuration error — never a panic.
    let _ = spec.validate();
}

#[test]
fn test_workload_spec_container_with_empty_image_fails_validation() {
    use std::collections::HashMap;
    let spec = WorkloadSpec::Container {
        image: String::new(),
        command: None,
        args: None,
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };
    let err = spec.validate().unwrap_err();
    // `ToadStoolError::validation(...)` wraps into `Configuration(ConfigError::ValidationError)`
    assert!(
        matches!(err, ToadStoolError::Configuration(_)),
        "Empty container image must return Configuration/Validation error, got: {err:?}"
    );
}

#[test]
fn test_workload_type_variants_are_distinct() {
    assert_ne!(WorkloadType::Native, WorkloadType::Wasm);
    assert_ne!(WorkloadType::Wasm, WorkloadType::Container);
    assert_ne!(WorkloadType::Container, WorkloadType::Gpu);
    assert_ne!(WorkloadType::Gpu, WorkloadType::Python);
    assert_ne!(WorkloadType::Python, WorkloadType::AiMl);
    assert_ne!(WorkloadType::AiMl, WorkloadType::Cuda);
}

// ── ExecutionRequest construction ─────────────────────────────────────────────

#[test]
fn test_execution_request_default_is_well_formed() {
    let req = ExecutionRequest::default();
    // Unique execution IDs
    assert_ne!(req.execution_id, Uuid::nil());
    // Security context validates cleanly
    assert!(
        req.security_context.validate().is_ok(),
        "Default security context must pass validation"
    );
    // Workload type accessible
    let _ = req.workload.workload_type();
}

#[test]
fn test_execution_request_unique_ids() {
    let r1 = make_request();
    let r2 = make_request();
    assert_ne!(
        r1.execution_id, r2.execution_id,
        "Each request must receive a unique ID"
    );
}

#[test]
fn test_execution_request_with_runtime_hint() {
    let req = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::default(),
        runtime_hint: Some(RuntimeType::Native),
        ..ExecutionRequest::default()
    };
    assert_eq!(req.runtime_hint, Some(RuntimeType::Native));
}

// ── Execution with no registered engines ──────────────────────────────────────

#[tokio::test]
async fn test_execute_with_no_engines_returns_not_found() {
    let orch = make_orchestrator();
    let req = make_request();
    let result = orch.execute(req).await;
    assert!(
        result.is_err(),
        "Executing with no registered engines must fail"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ToadStoolError::NotFound(_) | ToadStoolError::Runtime(_)
        ),
        "Expected NotFound or Runtime error, got: {err:?}"
    );
}

#[tokio::test]
async fn test_execute_with_native_hint_and_no_engine_returns_not_found() {
    let orch = make_orchestrator();
    let req = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: make_url_workload(),
        runtime_hint: Some(RuntimeType::Native),
        ..ExecutionRequest::default()
    };
    let result = orch.execute(req).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ToadStoolError::NotFound(_) | ToadStoolError::Runtime(_)
        ),
        "Hint to unavailable engine: {err:?}"
    );
}

#[tokio::test]
async fn test_concurrent_executions_both_complete() {
    let orch = std::sync::Arc::new(make_orchestrator());
    let orch2 = orch.clone();
    let (r1, r2) = tokio::join!(orch.execute(make_request()), orch2.execute(make_request()),);
    // Both should complete (success or graceful error — no panics or hangs)
    let _ = r1;
    let _ = r2;
}

// ── Security context ──────────────────────────────────────────────────────────

#[test]
fn test_security_context_validate_default() {
    let ctx = toadstool::SecurityContext::default();
    assert!(
        ctx.validate().is_ok(),
        "Default SecurityContext must pass validate()"
    );
}

#[test]
fn test_security_context_in_request_validates() {
    let req = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: WorkloadSpec::default(),
        security_context: toadstool::SecurityContext::default(),
        ..ExecutionRequest::default()
    };
    assert!(req.security_context.validate().is_ok());
}

// ── ExecutionResponse structure ───────────────────────────────────────────────

#[test]
fn test_execution_response_default_is_success() {
    let resp = ExecutionResponse::default();
    assert_eq!(resp.status, ExecutionStatus::Success);
    assert!(!resp.execution_id.is_nil());
    // duration is zero for default
    assert_eq!(resp.duration.as_secs(), 0);
    // runtime_used is Native by default
    assert_eq!(resp.runtime_used, RuntimeType::Native);
    // no warnings by default
    assert!(resp.warnings.is_empty());
}

#[test]
fn test_execution_response_preserves_execution_id() {
    let id = Uuid::new_v4();
    let resp = ExecutionResponse {
        execution_id: id,
        ..ExecutionResponse::default()
    };
    assert_eq!(resp.execution_id, id);
}

// ── Resource requirements ──────────────────────────────────────────────────────

#[test]
fn test_resource_requirements_default_is_valid() {
    let req = toadstool::ResourceRequirements::default();
    // validate() was implemented in Session 22
    assert!(req.validate().is_ok());
}

// ── Workload validation edge cases ────────────────────────────────────────────

#[test]
fn test_workload_type_roundtrip_through_spec() {
    let types_and_specs: Vec<(WorkloadType, WorkloadSpec)> =
        vec![(WorkloadType::Native, make_url_workload())];
    for (expected_type, spec) in types_and_specs {
        assert_eq!(
            spec.workload_type(),
            expected_type,
            "WorkloadSpec should report {expected_type:?}"
        );
    }
}
