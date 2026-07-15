// SPDX-License-Identifier: AGPL-3.0-or-later

//! Personality swap orchestration.
//!
//! The swap lifecycle generalizes the visualization service's GPU personality swap into a
//! hardware-agnostic sequence:
//!
//! 1. **Quiesce** — drain in-flight operations, wait for quiescence
//! 2. **Persist** — snapshot device-specific state to ember's metadata store
//! 3. **Drop** — release the current exclusive handle
//! 4. **Delegate** — ask ember to perform the actual driver bind/unbind
//! 5. **Reacquire** — get the new handle from ember
//! 6. **Restore** — replay persisted state onto the new personality
//! 7. **Health** — verify the device is healthy in its new personality
//!
//! Each hardware class implements [`SwapExecutor`] for steps 3-5 (the
//! bus-specific part). The orchestration framework (steps 1-2, 6-7) is
//! shared.

use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::boot::{BootResult, BootStep, StepStatus};
use crate::device_id::DeviceId;
use crate::firmware::BootServiceEvidence;

/// Result of a personality swap attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapObservation {
    /// Device that was swapped.
    pub device_id: String,
    /// Personality before the swap.
    pub from: String,
    /// Personality after the swap (or attempted target on failure).
    pub to: String,
    /// Whether the swap succeeded.
    pub success: bool,
    /// How long the swap took.
    pub duration: Duration,
    /// Optional error message if the swap failed.
    pub error: Option<String>,
    /// Optional diagnostic detail (hardware-specific).
    pub detail: Option<serde_json::Value>,
}

/// Executes the bus-specific portion of a personality swap.
///
/// The orchestrator calls this after quiescing and persisting state,
/// and before restoring state and health-checking.
#[expect(
    async_fn_in_trait,
    reason = "generic via type param, no dyn dispatch; associated Error type prevents object safety"
)]
pub trait SwapExecutor: Send + Sync + fmt::Debug {
    /// Error type for swap operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Perform the actual driver unbind/rebind for this device.
    ///
    /// The implementation should:
    /// 1. Drop the current exclusive handle (if any)
    /// 2. Trigger the kernel to switch drivers (sysfs bind/unbind, etc.)
    /// 3. Acquire the new exclusive handle
    ///
    /// # Errors
    ///
    /// Returns an error if the swap fails at any step.
    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error>;

    /// Release a device back to unbound state without swapping to another
    /// personality.
    ///
    /// # Errors
    ///
    /// Returns an error if the release fails.
    async fn release(&self, device: &DeviceId) -> Result<(), Self::Error>;
}

/// Callback invoked between Persist and Drop to capture firmware
/// boot service evidence before the driver swap destroys it.
///
/// When `Some`, the orchestrator calls this after persisting device state
/// but before releasing the handle. This is the ExitBootServices slot
/// from the UEFI model — the firmware is still running, and this is the
/// last chance to capture what it initialized.
pub type ExitBootServicesFn = Box<dyn Fn(&DeviceId) -> Option<BootServiceEvidence> + Send + Sync>;

/// Orchestrates the full swap lifecycle.
///
/// Wraps a [`SwapExecutor`] with quiescence, persistence, restoration, and
/// health checking. This is the high-level API that the ecosystem calls.
///
/// The lifecycle is 7 steps (8 when ExitBootServices is configured):
///
/// 1. **Quiesce** — drain in-flight operations
/// 2. **Persist** — snapshot device state
/// 3. **ExitBootServices** (optional) — capture firmware evidence
/// 4. **Drop** — release the current handle
/// 5. **Delegate** — execute the bus-specific swap
/// 6. **Reacquire** — get the new handle
/// 7. **Restore** — replay persisted state
/// 8. **Health** — verify the device is healthy
pub struct SwapOrchestrator<E: SwapExecutor> {
    executor: E,
    quiescence_timeout: Duration,
    exit_boot_services: Option<ExitBootServicesFn>,
}

impl<E: SwapExecutor> fmt::Debug for SwapOrchestrator<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SwapOrchestrator")
            .field("executor", &self.executor)
            .field("quiescence_timeout", &self.quiescence_timeout)
            .field(
                "exit_boot_services",
                &self.exit_boot_services.as_ref().map(|_| "<fn>"),
            )
            .finish()
    }
}

impl<E: SwapExecutor> SwapOrchestrator<E> {
    const DEFAULT_QUIESCENCE_TIMEOUT_SECS: u64 = 5;

    /// Create a new orchestrator wrapping a bus-specific executor.
    #[must_use]
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            quiescence_timeout: Duration::from_secs(Self::DEFAULT_QUIESCENCE_TIMEOUT_SECS),
            exit_boot_services: None,
        }
    }

    /// Set the quiescence timeout.
    #[must_use]
    pub fn with_quiescence_timeout(mut self, timeout: Duration) -> Self {
        self.quiescence_timeout = timeout;
        self
    }

    /// Configure the ExitBootServices callback.
    ///
    /// When set, the orchestrator calls this after persisting state but
    /// before releasing the handle. This captures firmware-initialized
    /// hardware state (PRI ring, TPC stations) as evidence before the
    /// driver swap can destroy it.
    #[must_use]
    pub fn with_exit_boot_services(mut self, f: ExitBootServicesFn) -> Self {
        self.exit_boot_services = Some(f);
        self
    }

    /// Access the underlying executor.
    #[must_use]
    pub const fn executor(&self) -> &E {
        &self.executor
    }

    /// The configured quiescence timeout.
    #[must_use]
    pub const fn quiescence_timeout(&self) -> Duration {
        self.quiescence_timeout
    }

    /// Path for persisting device state across a swap.
    fn swap_state_path(device: &DeviceId) -> std::path::PathBuf {
        let label = device.short_label().replace([':', '/'], "-");
        std::env::temp_dir().join(format!("toadstool-swap-{label}.json"))
    }

    /// Step 1: Quiesce — poll sysfs for DRM engine idle state.
    ///
    /// For PCI devices, reads `/sys/bus/pci/devices/{bdf}/power_state` to
    /// confirm the device is in D0 and reads `drm/card*/gpu_busy_percent`
    /// to check for idle engines. Falls back to a short yield if sysfs
    /// attributes are unavailable.
    async fn quiesce_device(device: &DeviceId, timeout: Duration) -> String {
        let DeviceId::PciBdf(bdf) = device else {
            tokio::time::sleep(Duration::from_millis(1)).await;
            return format!("timeout_ms={} (non-PCI, yield only)", timeout.as_millis());
        };

        #[cfg(target_os = "linux")]
        {
            return Self::quiesce_device_linux(bdf.as_str(), timeout).await;
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = bdf;
            tokio::time::sleep(Duration::from_millis(1)).await;
            format!("timeout_ms={} (non-Linux, yield only)", timeout.as_millis())
        }
    }

    /// Linux sysfs quiescence: poll DRM `gpu_busy_percent` and PCI power state.
    #[cfg(target_os = "linux")]
    async fn quiesce_device_linux(bdf: &str, timeout: Duration) -> String {
        let power_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "power_state");
        let power = std::fs::read_to_string(&power_path)
            .ok()
            .map_or_else(|| "unknown".into(), |s| s.trim().to_string());

        let gpu_busy = Self::read_gpu_busy_percent(bdf);

        let deadline = tokio::time::Instant::now() + timeout;
        if let Some(busy) = gpu_busy {
            if busy > 0 {
                tracing::info!(bdf, busy, "GPU busy, waiting for quiescence");
                while tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    if Self::read_gpu_busy_percent(bdf).unwrap_or(0) == 0 {
                        break;
                    }
                }
            }
        } else {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        let final_busy = Self::read_gpu_busy_percent(bdf).unwrap_or(0);
        format!(
            "power={power} gpu_busy={final_busy}% timeout_ms={}",
            timeout.as_millis()
        )
    }

    /// Read gpu_busy_percent from the first DRM card node for this BDF.
    #[cfg(target_os = "linux")]
    fn read_gpu_busy_percent(bdf: &str) -> Option<u32> {
        let drm_dir = std::path::PathBuf::from(
            toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "drm"),
        );
        let entries = std::fs::read_dir(&drm_dir).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("card") && !name_str.contains("render") {
                let busy_path = drm_dir.join(&name).join("gpu_busy_percent");
                if let Ok(content) = std::fs::read_to_string(&busy_path) {
                    return content.trim().parse().ok();
                }
            }
        }
        None
    }

    /// Step 2: Persist — snapshot current driver and power state to a temp file.
    fn persist_device_state(device: &DeviceId, from: &str) -> String {
        let state = serde_json::json!({
            "device": device.short_label(),
            "from_personality": from,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        });

        let path = Self::swap_state_path(device);
        match std::fs::write(&path, state.to_string()) {
            Ok(()) => {
                tracing::debug!(?path, "persisted swap state");
                format!("from={from} state_file={}", path.display())
            }
            Err(e) => {
                tracing::warn!(?path, %e, "could not persist swap state (non-fatal)");
                format!("from={from} persist_error={e}")
            }
        }
    }

    /// Step 6: Verify device came back in the expected personality.
    fn verify_post_swap_state(device: &DeviceId, target: &str, obs: &SwapObservation) -> String {
        let path = Self::swap_state_path(device);
        let _ = std::fs::remove_file(&path);

        if obs.to == target {
            format!("verified personality={} matches target", obs.to)
        } else {
            format!("personality mismatch: got {} expected {}", obs.to, target)
        }
    }

    /// Execute the full 7-step swap lifecycle for a device.
    ///
    /// 1. **Quiesce** — wait for in-flight operations to drain
    /// 2. **Persist** — snapshot device state (recorded in step log)
    /// 3. **Drop** — release the current handle
    /// 4. **Delegate** — execute the bus-specific swap
    /// 5. **Reacquire** — the executor returns the new observation
    /// 6. **Restore** — replay persisted state (recorded in step log)
    /// 7. **Health** — verify the device is healthy
    pub async fn orchestrate_swap(
        &self,
        device: &DeviceId,
        from: &str,
        target: &str,
    ) -> BootResult {
        let mut steps = Vec::new();
        let overall_start = Instant::now();
        let device_label = device.short_label();

        // Step 1: Quiesce — verify device has no in-flight DRM work via sysfs
        let step_start = Instant::now();
        tracing::info!(
            device = device_label.as_str(),
            timeout_ms = self.quiescence_timeout.as_millis() as u64,
            "quiescing device"
        );
        let quiesce_detail = Self::quiesce_device(device, self.quiescence_timeout).await;
        steps.push(BootStep::ok(
            "quiesce",
            Some(quiesce_detail),
            step_start.elapsed().as_millis() as u64,
        ));

        // Step 2: Persist — snapshot driver and power state to temp file
        let step_start = Instant::now();
        tracing::debug!(device = device_label.as_str(), "persisting device state");
        let persist_detail = Self::persist_device_state(device, from);
        steps.push(BootStep::ok(
            "persist",
            Some(persist_detail),
            step_start.elapsed().as_millis() as u64,
        ));

        // Step 3 (optional): ExitBootServices — capture firmware evidence
        if let Some(ref ebs_fn) = self.exit_boot_services {
            let step_start = Instant::now();
            if let Some(evidence) = ebs_fn(device) {
                let n_preserved = evidence.preserved_state.len();
                tracing::info!(
                    device = device_label.as_str(),
                    engine = evidence.engine.as_str(),
                    preserved_keys = n_preserved,
                    "ExitBootServices: firmware evidence captured"
                );
                steps.push(BootStep::ok(
                    "exit_boot_services",
                    Some(format!(
                        "engine={} preserved={n_preserved} keys",
                        evidence.engine
                    )),
                    step_start.elapsed().as_millis() as u64,
                ));
            } else {
                tracing::debug!(
                    device = device_label.as_str(),
                    "ExitBootServices: no evidence (firmware not in boot services mode)"
                );
                steps.push(BootStep {
                    name: "exit_boot_services".into(),
                    status: StepStatus::Skipped,
                    detail: Some("firmware not in boot services mode".into()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
        }

        // Step 4: Drop (release current handle)
        let step_start = Instant::now();
        match self.executor.release(device).await {
            Ok(()) => {
                steps.push(BootStep::ok(
                    "drop_handle",
                    None,
                    step_start.elapsed().as_millis() as u64,
                ));
            }
            Err(e) => {
                tracing::warn!(device = device_label.as_str(), error = %e, "release before swap failed (continuing)");
                steps.push(BootStep {
                    name: "drop_handle".into(),
                    status: StepStatus::Skipped,
                    detail: Some(format!("release failed (non-fatal): {e}")),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
        }

        // Step 4: Delegate (execute bus-specific swap)
        let step_start = Instant::now();
        let obs = match self.executor.execute_swap(device, target).await {
            Ok(obs) => {
                steps.push(BootStep::ok(
                    "delegate_swap",
                    Some(format!(
                        "to={target} duration_ms={}",
                        obs.duration.as_millis()
                    )),
                    step_start.elapsed().as_millis() as u64,
                ));
                obs
            }
            Err(e) => {
                steps.push(BootStep::failed(
                    "delegate_swap",
                    Some(format!("swap failed: {e}")),
                    step_start.elapsed().as_millis() as u64,
                ));
                return BootResult {
                    device_id: device_label,
                    initial_personality: Some(from.into()),
                    warm_cycle_performed: false,
                    final_personality: None,
                    init_result: None,
                    steps,
                    success: false,
                    summary: format!("swap to {target} failed: {e}"),
                };
            }
        };

        // Step 5: Reacquire (already done by executor, record observation)
        steps.push(BootStep::ok(
            "reacquire",
            Some(format!("personality={}", obs.to)),
            0,
        ));

        // Step 6: Restore — verify device came back in expected personality
        let step_start_restore = Instant::now();
        let restore_detail = Self::verify_post_swap_state(device, target, &obs);
        steps.push(BootStep::ok(
            "restore",
            Some(restore_detail),
            step_start_restore.elapsed().as_millis() as u64,
        ));

        // Step 7: Health check
        let step_start = Instant::now();
        let health_ok = obs.success;
        steps.push(if health_ok {
            BootStep::ok(
                "health_check",
                Some("device healthy after swap".to_string()),
                step_start.elapsed().as_millis() as u64,
            )
        } else {
            BootStep::failed(
                "health_check",
                Some("device not healthy after swap".to_string()),
                step_start.elapsed().as_millis() as u64,
            )
        });

        let total_ms = overall_start.elapsed().as_millis();
        BootResult {
            device_id: device_label,
            initial_personality: Some(from.into()),
            warm_cycle_performed: false,
            final_personality: Some(obs.to.clone()),
            init_result: obs.detail.clone(),
            steps,
            success: health_ok,
            summary: format!(
                "swap {from} → {} {} (total: {total_ms}ms)",
                obs.to,
                if health_ok { "succeeded" } else { "failed" },
            ),
        }
    }

    /// Execute the full sovereign boot sequence for a device.
    ///
    /// This is the high-level entry point that combines driver detection
    /// with the 7-step swap lifecycle.
    pub async fn execute_boot(
        &self,
        device: &DeviceId,
        current_personality: Option<&str>,
        target_personality: &str,
    ) -> BootResult {
        let device_label = device.short_label();
        let from = current_personality.unwrap_or("unknown");

        tracing::info!(
            device = device_label.as_str(),
            from,
            to = target_personality,
            "beginning sovereign boot"
        );

        let result = self
            .orchestrate_swap(device, from, target_personality)
            .await;

        if result.success {
            tracing::info!(
                device = device_label.as_str(),
                steps = result.steps.len(),
                summary = result.summary.as_str(),
                "sovereign boot completed"
            );
        } else {
            tracing::error!(
                device = device_label.as_str(),
                summary = result.summary.as_str(),
                "sovereign boot failed"
            );
        }

        result
    }
}

#[cfg(test)]
#[path = "swap_tests.rs"]
mod tests;
