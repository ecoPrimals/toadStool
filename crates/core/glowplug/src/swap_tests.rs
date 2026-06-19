// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
struct MockExecutor {
    should_fail: AtomicBool,
}

impl MockExecutor {
    fn new() -> Self {
        Self {
            should_fail: AtomicBool::new(false),
        }
    }

    fn failing() -> Self {
        Self {
            should_fail: AtomicBool::new(true),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("mock swap error")]
struct MockSwapErr;

impl SwapExecutor for MockExecutor {
    type Error = MockSwapErr;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        if self.should_fail.load(Ordering::Relaxed) {
            return Err(MockSwapErr);
        }
        Ok(SwapObservation {
            device_id: device.short_label(),
            from: "unbound".into(),
            to: target_personality.into(),
            success: true,
            duration: Duration::from_millis(50),
            error: None,
            detail: None,
        })
    }

    async fn release(&self, _device: &DeviceId) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn orchestrator_default_timeout() {
    let orch = SwapOrchestrator::new(MockExecutor::new());
    assert_eq!(orch.quiescence_timeout(), Duration::from_secs(5));
}

#[test]
fn orchestrator_custom_timeout() {
    let orch =
        SwapOrchestrator::new(MockExecutor::new()).with_quiescence_timeout(Duration::from_secs(10));
    assert_eq!(orch.quiescence_timeout(), Duration::from_secs(10));
}

#[test]
fn orchestrator_executor_accessible() {
    let orch = SwapOrchestrator::new(MockExecutor::new());
    assert!(format!("{:?}", orch.executor()).contains("MockExecutor"));
}

#[tokio::test]
async fn mock_executor_swap_succeeds() {
    let exec = MockExecutor::new();
    let device = DeviceId::PciBdf("0000:01:00.0".into());
    let obs = exec.execute_swap(&device, "vfio").await.expect("swap");
    assert!(obs.success);
    assert_eq!(obs.to, "vfio");
}

#[tokio::test]
async fn mock_executor_release_succeeds() {
    let exec = MockExecutor::new();
    let device = DeviceId::UsbPath("1-2".into());
    exec.release(&device).await.expect("release");
}

#[test]
fn swap_observation_serde_roundtrip() {
    let obs = SwapObservation {
        device_id: "pci:0000:01:00.0".into(),
        from: "nouveau".into(),
        to: "vfio".into(),
        success: true,
        duration: Duration::from_millis(123),
        error: None,
        detail: Some(serde_json::json!({"bar0_ok": true})),
    };
    let json = serde_json::to_string(&obs).expect("serialize");
    let back: SwapObservation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.device_id, "pci:0000:01:00.0");
    assert!(back.success);
    assert!(back.error.is_none());
    assert!(back.detail.is_some());
}

#[test]
fn swap_observation_failure() {
    let obs = SwapObservation {
        device_id: "usb:1-2".into(),
        from: "host".into(),
        to: "gadget".into(),
        success: false,
        duration: Duration::from_secs(1),
        error: Some("device busy".into()),
        detail: None,
    };
    assert!(!obs.success);
    assert_eq!(obs.error.as_deref(), Some("device busy"));
}

#[tokio::test]
async fn orchestrate_swap_7_steps_success() {
    let orch = SwapOrchestrator::new(MockExecutor::new());
    let device = DeviceId::PciBdf("0000:01:00.0".into());
    let result = orch.orchestrate_swap(&device, "nouveau", "vfio").await;

    assert!(result.success);
    assert_eq!(result.steps.len(), 7);
    assert_eq!(result.steps[0].name, "quiesce");
    assert_eq!(result.steps[1].name, "persist");
    assert_eq!(result.steps[2].name, "drop_handle");
    assert_eq!(result.steps[3].name, "delegate_swap");
    assert_eq!(result.steps[4].name, "reacquire");
    assert_eq!(result.steps[5].name, "restore");
    assert_eq!(result.steps[6].name, "health_check");
    assert!(result.summary.contains("succeeded"));
    assert_eq!(result.initial_personality.as_deref(), Some("nouveau"));
    assert_eq!(result.final_personality.as_deref(), Some("vfio"));
}

#[tokio::test]
async fn orchestrate_swap_failure_stops_at_delegate() {
    let orch = SwapOrchestrator::new(MockExecutor::failing());
    let device = DeviceId::PciBdf("0000:01:00.0".into());
    let result = orch.orchestrate_swap(&device, "nouveau", "vfio").await;

    assert!(!result.success);
    assert_eq!(result.steps.len(), 4);
    assert_eq!(result.steps[3].name, "delegate_swap");
    assert_eq!(result.steps[3].status, StepStatus::Failed);
    assert!(result.summary.contains("failed"));
}

#[tokio::test]
async fn execute_boot_delegates_to_orchestrate() {
    let orch = SwapOrchestrator::new(MockExecutor::new());
    let device = DeviceId::PciBdf("0000:01:00.0".into());
    let result = orch.execute_boot(&device, Some("nvidia"), "vfio").await;

    assert!(result.success);
    assert_eq!(result.initial_personality.as_deref(), Some("nvidia"));
    assert_eq!(result.final_personality.as_deref(), Some("vfio"));
    assert_eq!(result.steps.len(), 7);
}

#[tokio::test]
async fn execute_boot_unknown_personality() {
    let orch = SwapOrchestrator::new(MockExecutor::new());
    let device = DeviceId::PciBdf("0000:01:00.0".into());
    let result = orch.execute_boot(&device, None, "vfio").await;

    assert!(result.success);
    assert_eq!(result.initial_personality.as_deref(), Some("unknown"));
}

/// Executor where `release` always fails but `execute_swap` succeeds.
#[derive(Debug)]
struct FailingReleaseMockExecutor;

impl SwapExecutor for FailingReleaseMockExecutor {
    type Error = MockSwapErr;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        Ok(SwapObservation {
            device_id: device.short_label(),
            from: "old".into(),
            to: target_personality.into(),
            success: true,
            duration: Duration::from_millis(10),
            error: None,
            detail: None,
        })
    }

    async fn release(&self, _device: &DeviceId) -> Result<(), Self::Error> {
        Err(MockSwapErr)
    }
}

#[tokio::test]
async fn orchestrate_swap_release_failure_is_non_fatal() {
    let orch = SwapOrchestrator::new(FailingReleaseMockExecutor);
    let device = DeviceId::PciBdf("0000:02:00.0".into());
    let result = orch.orchestrate_swap(&device, "nvidia", "vfio").await;

    assert!(result.success, "swap should succeed even if release fails");
    assert_eq!(result.steps.len(), 7);
    assert_eq!(result.steps[2].name, "drop_handle");
    assert_eq!(result.steps[2].status, StepStatus::Skipped);
    assert!(
        result.steps[2]
            .detail
            .as_ref()
            .is_some_and(|d| d.contains("release failed (non-fatal)"))
    );
}

/// Executor where swap succeeds but returns `success: false` in observation.
#[derive(Debug)]
struct UnhealthySwapMockExecutor;

impl SwapExecutor for UnhealthySwapMockExecutor {
    type Error = MockSwapErr;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        Ok(SwapObservation {
            device_id: device.short_label(),
            from: "old".into(),
            to: target_personality.into(),
            success: false,
            duration: Duration::from_millis(30),
            error: Some("device not responding after swap".into()),
            detail: None,
        })
    }

    async fn release(&self, _device: &DeviceId) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[tokio::test]
async fn orchestrate_swap_unhealthy_device_fails_at_health_step() {
    let orch = SwapOrchestrator::new(UnhealthySwapMockExecutor);
    let device = DeviceId::PciBdf("0000:03:00.0".into());
    let result = orch.orchestrate_swap(&device, "nouveau", "vfio").await;

    assert!(!result.success, "swap should fail when device is unhealthy");
    assert_eq!(result.steps.len(), 7);
    assert_eq!(result.steps[6].name, "health_check");
    assert_eq!(result.steps[6].status, StepStatus::Failed);
    assert!(
        result.steps[6]
            .detail
            .as_ref()
            .is_some_and(|d| d.contains("not healthy"))
    );
    assert!(result.summary.contains("failed"));
}

#[tokio::test]
async fn execute_boot_with_unhealthy_swap_reports_failure() {
    let orch = SwapOrchestrator::new(UnhealthySwapMockExecutor);
    let device = DeviceId::PciBdf("0000:04:00.0".into());
    let result = orch.execute_boot(&device, Some("nvidia"), "vfio").await;

    assert!(!result.success);
    assert_eq!(result.final_personality.as_deref(), Some("vfio"));
    assert!(result.summary.contains("failed"));
}
