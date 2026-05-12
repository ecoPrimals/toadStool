// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `execute_biome_os` paths.

use std::sync::Arc;

use crate::execution::ExecutionStatus;
use crate::universal::UniversalScheduler;
use crate::universal::primal_provider_dispatch::UniversalPrimalProviderDispatch;
use crate::universal::registry::UniversalPrimalRegistry;
use crate::universal::requests::ResponseStatus;
use crate::universal::types::PrimalType;

use super::{sample_context, TypedRoutePrimal};

#[tokio::test]
async fn execute_biome_os_success() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "biome-1".to_string(),
        context: ctx,
        primal_type: PrimalType::OS,
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({"v": 1}), "team-a")
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output
            .stdout
            .as_ref()
            .is_some_and(|s| s.contains("team-a")),
        "stdout: {:?}",
        out.output.stdout
    );
}

#[tokio::test]
async fn execute_biome_os_non_success_statuses() {
    for status in [
        ResponseStatus::Error {
            code: "E".to_string(),
            message: "bio-err".to_string(),
        },
        ResponseStatus::Timeout,
        ResponseStatus::ServiceUnavailable,
    ] {
        let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
        let ctx = sample_context();
        let provider = Arc::new(TypedRoutePrimal {
            instance_id: "biome-os-x".to_string(),
            context: ctx,
            primal_type: PrimalType::OS,
            status,
            fail_route: false,
        });
        registry.register_primal(provider).await.unwrap();

        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let out = scheduler
            .execute_biome_os(&serde_json::json!({}), "t1")
            .await
            .unwrap();
        assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    }
}

#[tokio::test]
async fn execute_biome_os_route_error() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "biome-bad".to_string(),
        context: ctx,
        primal_type: PrimalType::OS,
        status: ResponseStatus::Success,
        fail_route: true,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({}), "team-z")
        .await
        .unwrap();
    assert!(
        out.output
            .stderr
            .as_ref()
            .is_some_and(|s| s.contains("BiomeOS"))
    );
}

#[tokio::test]
async fn execute_biome_os_no_os_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({}), "solo")
        .await
        .unwrap();
    assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    assert!(
        out.warnings.iter().any(|w| w.contains("BiomeOS primal")),
        "warnings: {:?}",
        out.warnings
    );
}
