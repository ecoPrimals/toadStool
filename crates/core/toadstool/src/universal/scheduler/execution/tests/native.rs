// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `execute_native` paths — primal routing, local engine fallback.

use std::collections::HashMap;
use std::sync::Arc;

use crate::execution::{ExecutionStatus, RuntimeType};
use crate::universal::UniversalScheduler;
use crate::universal::primal_provider_dispatch::UniversalPrimalProviderDispatch;
use crate::universal::registry::UniversalPrimalRegistry;
use crate::universal::requests::ResponseStatus;

use super::{
    sample_context, FailingNativePrimal, MockRuntimeEngine, NativePrimalTemplate, OnlyWasmPrimal,
};

#[tokio::test]
async fn execute_native_via_primal_success_with_stdout_stderr_exit() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-1".to_string(),
        context: ctx,
        status: ResponseStatus::Success,
        payload: serde_json::json!({
            "stdout": "hello world",
            "stderr": "debug info",
            "exit_code": 0,
        }),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/echo", &["hello".to_string()], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.output.stdout.as_deref(), Some("hello world"));
    assert_eq!(out.output.stderr.as_deref(), Some("debug info"));
    assert_eq!(out.output.exit_code, Some(0));
}

#[tokio::test]
async fn execute_native_via_primal_error_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-err".to_string(),
        context: ctx,
        status: ResponseStatus::Error {
            code: "E001".to_string(),
            message: "runtime error".to_string(),
        },
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/false", &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
}

#[tokio::test]
async fn execute_native_via_primal_timeout_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-to".to_string(),
        context: ctx,
        status: ResponseStatus::Timeout,
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/sleep", &["100".to_string()], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::TimedOut);
}

#[tokio::test]
async fn execute_native_via_primal_service_unavailable_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-su".to_string(),
        context: ctx,
        status: ResponseStatus::ServiceUnavailable,
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/true", &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(
        out.status,
        ExecutionStatus::Failed { ref error } if error == "Service unavailable"
    ));
}

#[tokio::test]
async fn execute_native_primal_handler_error_propagates() {
    let registry = Arc::new(UniversalPrimalRegistry::<FailingNativePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(FailingNativePrimal {
        instance_id: "fail-native".to_string(),
        context: ctx,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let err = scheduler
        .execute_native("echo", &[], &HashMap::new())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("mock native primal failure"),
        "unexpected: {err}"
    );
}

#[tokio::test]
async fn execute_native_uses_local_engine_when_no_native_primal() {
    let registry = Arc::new(UniversalPrimalRegistry::<OnlyWasmPrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(OnlyWasmPrimal {
        instance_id: "wasm-only".to_string(),
        context: ctx,
    });
    registry.register_primal(provider).await.unwrap();

    let mut engines = HashMap::new();
    engines.insert(RuntimeType::Native, Arc::new(MockRuntimeEngine));
    let scheduler =
        UniversalScheduler::<OnlyWasmPrimal, MockRuntimeEngine>::create_with_runtime_engines(
            registry, engines,
        )
        .await
        .unwrap();

    let out = scheduler
        .execute_native("ignored", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.runtime_used, RuntimeType::Native);
}

#[tokio::test]
async fn execute_native_direct_process_stderr_none_when_empty() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("true", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output.stderr.is_none(),
        "empty stderr should be None: {:?}",
        out.output.stderr
    );
}
