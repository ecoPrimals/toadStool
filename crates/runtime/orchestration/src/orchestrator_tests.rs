// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::future::Future;
use std::sync::Arc;

struct MockSubstrate {
    name: String,
    substrate_type: SubstrateType,
}

impl ComputeSubstrate for MockSubstrate {
    fn name(&self) -> &str {
        &self.name
    }

    fn substrate_type(&self) -> SubstrateType {
        self.substrate_type
    }

    fn execute_buffer_op(
        &self,
        _op: BufferOperation,
    ) -> impl Future<Output = Result<BufferOutput, toadstool_runtime_universal::SubstrateError>>
    + Send
    + '_ {
        let substrate_name = self.name.clone();
        async move {
            Ok(BufferOutput {
                data: vec![0; 100],
                metadata: BufferMetadata {
                    duration: Duration::from_millis(10),
                    substrate_name,
                    power_consumed_mw: Some(50000.0),
                },
            })
        }
    }
}

#[tokio::test]
async fn test_orchestrator_creation() {
    let orchestrator = WorkloadOrchestrator::<MockSubstrate>::discover()
        .await
        .unwrap();
    assert_eq!(orchestrator.num_substrates().unwrap(), 0);
}

#[tokio::test]
async fn test_register_substrate() {
    let orchestrator = WorkloadOrchestrator::<MockSubstrate>::discover()
        .await
        .unwrap();

    let substrate = Arc::new(MockSubstrate {
        name: "Test CPU".to_string(),
        substrate_type: SubstrateType::Cpu,
    });

    orchestrator.register_substrate(substrate).unwrap();
    assert_eq!(orchestrator.num_substrates().unwrap(), 1);
}

#[tokio::test]
async fn test_workload_execution() {
    let substrate = Arc::new(MockSubstrate {
        name: "Test GPU".to_string(),
        substrate_type: SubstrateType::Gpu,
    });

    let orchestrator = WorkloadOrchestrator::with_substrates(vec![substrate]);

    let request = WorkloadRequest::new()
        .operation_count(1000)
        .target_latency()
        .build()
        .unwrap();

    let result = orchestrator.execute(request).await.unwrap();
    assert_eq!(result.substrate_name, "Test GPU");
    assert!(result.success);
}

#[test]
fn test_workload_request_builder() {
    let request = WorkloadRequest::new()
        .operation_count(5000)
        .power_budget_watts(50.0)
        .target_energy()
        .batch_size(100)
        .build()
        .unwrap();

    assert_eq!(request.operation_count, 5000);
    assert_eq!(request.power_budget_watts, Some(50.0));
    assert_eq!(request.target, PerformanceTarget::Energy);
    assert_eq!(request.batch_size, Some(100));
}

#[tokio::test]
async fn test_execute_with_fallback_no_substrates() {
    let orchestrator = WorkloadOrchestrator::<MockSubstrate>::discover()
        .await
        .unwrap();
    let request = WorkloadRequest::default();
    let result = orchestrator.execute_with_fallback(request).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        OrchestrationError::AllSubstratesFailed
    ));
}

#[tokio::test]
async fn test_execute_no_substrates_returns_error() {
    let orchestrator = WorkloadOrchestrator::<MockSubstrate>::discover()
        .await
        .unwrap();
    let request = WorkloadRequest::default();
    let result = orchestrator.execute(request).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        OrchestrationError::NoSubstrates
    ));
}

#[tokio::test]
async fn test_orchestrator_stats() {
    let orchestrator = WorkloadOrchestrator::<MockSubstrate>::discover()
        .await
        .unwrap();
    let stats = orchestrator.stats().unwrap();
    assert_eq!(stats.substrates_available, 0);
    assert_eq!(stats.total_executions, 0);
}
