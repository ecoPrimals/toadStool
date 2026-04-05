// SPDX-License-Identifier: AGPL-3.0-or-later
//! WASM execution path when no engine is available.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use toadstool::execution::{ExecutionStatus, RuntimeType};
use toadstool::resources::ResourceRequirements;
use toadstool::universal::{
    JobPriority, UniversalJob, UniversalJobType, UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::super::helpers::create_test_context;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_wasm_no_engine_returns_failed() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No WASM execution capability")
    ));
    assert_eq!(response.runtime_used, RuntimeType::Wasm);
    assert!(!response.warnings.is_empty());
}
