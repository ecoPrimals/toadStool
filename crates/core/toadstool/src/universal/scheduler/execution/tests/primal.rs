// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `execute_primal` routing — typed provider dispatch.

use std::sync::Arc;

use crate::execution::ExecutionStatus;
use crate::universal::UniversalScheduler;
use crate::universal::registry::UniversalPrimalRegistry;
use crate::universal::requests::ResponseStatus;
use crate::universal::types::PrimalType;

use super::{TypedRoutePrimal, sample_context};

#[tokio::test]
async fn execute_primal_success() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "worker-1".to_string(),
        context: ctx,
        primal_type: PrimalType::Custom("worker".to_string()),
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("worker", "run", &serde_json::json!({"x": 1}))
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output
            .stdout
            .as_ref()
            .is_some_and(|s| s.contains("worker")),
        "stdout: {:?}",
        out.output.stdout
    );
}

#[tokio::test]
async fn execute_primal_response_error_timeout_unavailable() {
    for (status, needle) in [
        (
            ResponseStatus::Error {
                code: "E".to_string(),
                message: "e-msg".to_string(),
            },
            "e-msg",
        ),
        (ResponseStatus::Timeout, "timed out"),
        (ResponseStatus::ServiceUnavailable, "unavailable"),
    ] {
        let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
        let ctx = sample_context();
        let provider = Arc::new(TypedRoutePrimal {
            instance_id: format!("inst-{needle}"),
            context: ctx,
            primal_type: PrimalType::Custom("alpha".to_string()),
            status,
            fail_route: false,
        });
        registry.register_primal(provider).unwrap();

        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let out = scheduler
            .execute_primal("alpha", "go", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(
            matches!(out.status, ExecutionStatus::Failed { .. }),
            "expected failed for {needle}: {:?}",
            out.status
        );
        let ExecutionStatus::Failed { error } = out.status else {
            unreachable!();
        };
        assert!(
            error.contains(needle),
            "error={error} expected needle={needle}"
        );
    }
}

#[tokio::test]
async fn execute_primal_route_handler_error_returns_failed_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "bad-route".to_string(),
        context: ctx,
        primal_type: PrimalType::Custom("beta".to_string()),
        status: ResponseStatus::Success,
        fail_route: true,
    });
    registry.register_primal(provider).unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("beta", "x", &serde_json::json!({}))
        .await
        .unwrap();
    assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    assert!(
        out.output
            .stderr
            .as_ref()
            .is_some_and(|s| s.contains("route handler failure")),
        "stderr: {:?}",
        out.output.stderr
    );
}

#[tokio::test]
async fn execute_primal_no_provider_lists_available_when_present() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "only-compute".to_string(),
        context: ctx,
        primal_type: PrimalType::Compute,
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("storage", "x", &serde_json::json!({}))
        .await
        .unwrap();
    let ExecutionStatus::Failed { error } = out.status else {
        panic!("expected failed");
    };
    assert!(
        error.contains("compute"),
        "error should list providers: {error}"
    );
}
