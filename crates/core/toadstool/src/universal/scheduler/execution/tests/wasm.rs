// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for `execute_wasm` paths.

use std::collections::HashMap;
use std::sync::Arc;

use crate::execution::{ExecutionStatus, RuntimeType};
use crate::universal::UniversalScheduler;
use crate::universal::primal_provider_dispatch::UniversalPrimalProviderDispatch;
use crate::universal::registry::UniversalPrimalRegistry;

use super::MockRuntimeEngine;

#[tokio::test]
async fn execute_wasm_no_engine_failed_response_and_warning() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_wasm(&[0, 97, 115, 109], &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(
        out.status,
        ExecutionStatus::Failed { ref error } if error.contains("WASM")
    ));
    assert!(out.warnings.iter().any(|w| w.contains("WASM")));
}

#[tokio::test]
async fn execute_wasm_with_registered_engine() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let mut engines = HashMap::new();
    engines.insert(RuntimeType::Wasm, Arc::new(MockRuntimeEngine));
    let scheduler =
        UniversalScheduler::<UniversalPrimalProviderDispatch, MockRuntimeEngine>::create_with_runtime_engines(
            registry, engines,
        )
        .await
        .unwrap();

    let out = scheduler
        .execute_wasm(&[0, 97, 115, 109], &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.runtime_used, RuntimeType::Wasm);
}
