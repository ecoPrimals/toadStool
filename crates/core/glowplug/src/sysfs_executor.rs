// SPDX-License-Identifier: AGPL-3.0-or-later

//! Production [`SwapExecutor`] using sysfs driver bind/unbind.
//!
//! Replaces `coral-glowplug`'s `EmberClient` cross-process IPC pattern:
//! since ember is now toadStool-internal (S237), personality swaps are
//! performed directly via `/sys/bus/pci/drivers/*/unbind` and `/bind`.
//!
//! The executor reads the current driver, unbinds it, writes
//! `driver_override` to the target, and triggers the new driver to bind.
//! This is the same operation that `coral-ember` performed over Unix
//! sockets — now inlined.

use std::time::Instant;

use crate::device_id::DeviceId;
use crate::swap::{SwapExecutor, SwapObservation};

/// Sysfs-based swap executor for PCI devices.
///
/// Performs driver unbind/rebind via sysfs writes. This is the first
/// production `SwapExecutor` — it replaces `EmberClient::swap_device`.
#[derive(Debug)]
pub struct SysfsSwapExecutor;

/// Errors from sysfs swap operations.
#[derive(Debug, thiserror::Error)]
pub enum SysfsSwapError {
    /// The device identifier is not a PCI BDF.
    #[error("sysfs swap requires PCI BDF, got: {0}")]
    NotPciBdf(String),
    /// sysfs write failed.
    #[error("sysfs write to {path}: {reason}")]
    SysfsWrite {
        /// sysfs path that failed.
        path: String,
        /// Reason for failure.
        reason: String,
    },
    /// The target driver did not bind after the swap.
    #[error("driver {driver} did not bind to {bdf} after swap")]
    BindFailed {
        /// PCI BDF.
        bdf: String,
        /// Expected driver.
        driver: String,
    },
}

use toadstool_cylinder::vfio::guarded_sysfs;

impl SysfsSwapExecutor {
    fn bdf_from_id(device: &DeviceId) -> Result<&str, SysfsSwapError> {
        match device {
            DeviceId::PciBdf(bdf) => Ok(bdf.as_str()),
            other => Err(SysfsSwapError::NotPciBdf(other.short_label())),
        }
    }

    fn sysfs_write(path: &str, value: &str) -> Result<(), SysfsSwapError> {
        guarded_sysfs::sysfs_write(path, value).map_err(|e| SysfsSwapError::SysfsWrite {
            path: path.into(),
            reason: e.to_string(),
        })
    }

    fn sysfs_write_guarded(path: &str, value: &str, timeout: std::time::Duration) -> Result<(), SysfsSwapError> {
        guarded_sysfs::sysfs_write_guarded(path, value, timeout).map_err(|e| SysfsSwapError::SysfsWrite {
            path: path.into(),
            reason: e.to_string(),
        })
    }

    fn is_warm_preserving_swap(from: &str, to: &str) -> bool {
        let is_init_driver = matches!(from, "nouveau" | "nvidia" | "amdgpu" | "xe" | "i915");
        let is_vfio = to == "vfio-pci";
        is_init_driver && is_vfio
    }

    fn driver_name_for_target(target: &str) -> &str {
        match target {
            "vfio" | "vfio-pci" => "vfio-pci",
            "nouveau" => "nouveau",
            "nvidia" => "nvidia",
            "nvidia-open" | "nvidia_open" => "nvidia",
            "amdgpu" => "amdgpu",
            "xe" => "xe",
            "i915" => "i915",
            "akida" | "akida-pcie" => "akida-pcie",
            other => other,
        }
    }
}

impl SysfsSwapExecutor {
    /// Execute a bare-metal [`WarmInitPlan`] — the diesel engine's core
    /// injection path for host-safe seeder drivers.
    ///
    /// Steps:
    /// 1. Unbind current driver (if any)
    /// 2. Bind seeder driver to target GPU
    /// 3. Wait for seeder to complete hardware init
    /// 4. Disable FLR to preserve warm state
    /// 5. Warm swap seeder → vfio-pci
    ///
    /// After this completes, cylinder can access the GPU via BAR0/BAR1/BAR3
    /// with all hardware state preserved from the seeder's initialization.
    ///
    /// # Panics
    ///
    /// Panics if the plan requires VM containment. Contained plans must be
    /// dispatched through agentReagents, not through sysfs bare-metal swaps.
    /// Use [`WarmInitPlan::is_bare_metal`] to check before calling.
    pub async fn execute_warm_init(
        &self,
        plan: &crate::warm_init::WarmInitPlan,
    ) -> crate::warm_init::WarmInitResult {
        use crate::warm_init::{WarmInitResult, WarmInitStep};
        use std::time::Instant;

        assert!(
            plan.is_bare_metal(),
            "contained plans must go through agentReagents, not bare-metal sysfs: {}",
            plan.seeder.name
        );

        let overall = Instant::now();
        let mut steps = Vec::new();
        let bdf = plan.bdf.as_str();

        // Step 1: Unbind current driver (if any) — guarded to prevent D-state hang
        let t = Instant::now();
        let prev_driver = guarded_sysfs::read_current_driver(bdf);
        if let Some(ref current) = prev_driver {
            let unbind_path = toadstool_cylinder::linux_paths::sysfs_pci_driver_unbind(current);
            if let Err(e) = Self::sysfs_write_guarded(&unbind_path, bdf, guarded_sysfs::UNBIND_TIMEOUT) {
                tracing::warn!(bdf, driver = current.as_str(), error = %e, "guarded unbind failed (continuing)");
            }
        }
        steps.push(WarmInitStep {
            name: "unbind_current".into(),
            ok: true,
            detail: prev_driver.map(|d| format!("was: {d}")),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // Step 2: Bind seeder driver
        let t = Instant::now();
        let seeder_module = Self::driver_name_for_target(&plan.seeder.name);
        let override_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "driver_override");
        if let Err(e) = Self::sysfs_write(&override_path, seeder_module) {
            steps.push(WarmInitStep {
                name: "seeder_bind".into(),
                ok: false,
                detail: Some(format!("driver_override failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "seeder_bind", steps, overall);
        }
        let drivers_probe_path = toadstool_cylinder::linux_paths::sysfs_pci_drivers_probe();
        if let Err(e) = Self::sysfs_write_guarded(&drivers_probe_path, bdf, guarded_sysfs::PROBE_TIMEOUT) {
            steps.push(WarmInitStep {
                name: "seeder_bind".into(),
                ok: false,
                detail: Some(format!("guarded drivers_probe failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "seeder_bind", steps, overall);
        }

        let bound = guarded_sysfs::read_current_driver(bdf);
        let bind_ok = bound.as_deref() == Some(seeder_module);
        steps.push(WarmInitStep {
            name: "seeder_bind".into(),
            ok: bind_ok,
            detail: Some(format!(
                "driver={} expected={}",
                bound.as_deref().unwrap_or("none"),
                seeder_module
            )),
            duration_ms: t.elapsed().as_millis() as u64,
        });
        if !bind_ok {
            return halt_result(bdf, &plan.seeder.name, "seeder_bind", steps, overall);
        }

        // Step 3: Wait for seeder to complete hardware init
        let t = Instant::now();
        tracing::info!(
            bdf,
            seeder = plan.seeder.name.as_str(),
            settle_ms = plan.seeder_settle.as_millis() as u64,
            "waiting for seeder hardware initialization"
        );
        tokio::time::sleep(plan.seeder_settle).await;
        steps.push(WarmInitStep {
            name: "seeder_settle".into(),
            ok: true,
            detail: Some(format!("{}ms settle", plan.seeder_settle.as_millis())),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // Step 4: Pin bridges + disable FLR for warm swap
        let t = Instant::now();
        guarded_sysfs::pin_bridge_hierarchy(bdf);
        guarded_sysfs::disable_flr(bdf);
        steps.push(WarmInitStep {
            name: "prepare_warm_swap".into(),
            ok: true,
            detail: Some("bridge pinned, FLR disabled".into()),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // Step 5: Warm swap to final target (vfio-pci)
        let t = Instant::now();
        let final_driver = Self::driver_name_for_target(&plan.final_target);

        if let Some(ref current) = guarded_sysfs::read_current_driver(bdf) {
            let unbind_path = toadstool_cylinder::linux_paths::sysfs_pci_driver_unbind(current);
            if let Err(e) = Self::sysfs_write_guarded(&unbind_path, bdf, guarded_sysfs::UNBIND_TIMEOUT) {
                steps.push(WarmInitStep {
                    name: "warm_swap".into(),
                    ok: false,
                    detail: Some(format!("guarded unbind {current} failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                return halt_result(bdf, &plan.seeder.name, "warm_swap", steps, overall);
            }
        }

        if let Err(e) = Self::sysfs_write(&override_path, final_driver) {
            steps.push(WarmInitStep {
                name: "warm_swap".into(),
                ok: false,
                detail: Some(format!("override to {final_driver} failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "warm_swap", steps, overall);
        }
        if let Err(e) = Self::sysfs_write_guarded(&drivers_probe_path, bdf, guarded_sysfs::PROBE_TIMEOUT) {
            steps.push(WarmInitStep {
                name: "warm_swap".into(),
                ok: false,
                detail: Some(format!("guarded drivers_probe for {final_driver} failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "warm_swap", steps, overall);
        }

        let final_bound = guarded_sysfs::read_current_driver(bdf);
        let swap_ok = final_bound.as_deref() == Some(final_driver);
        steps.push(WarmInitStep {
            name: "warm_swap".into(),
            ok: swap_ok,
            detail: Some(format!(
                "{} → {} (warm_preserved={})",
                plan.seeder.name,
                final_bound.as_deref().unwrap_or("none"),
                swap_ok
            )),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        WarmInitResult {
            bdf: bdf.to_string(),
            success: swap_ok,
            halted_at: None,
            seeder_used: plan.seeder.name.clone(),
            warm_preserved: swap_ok,
            steps,
            total_ms: overall.elapsed().as_millis() as u64,
        }
    }
}

fn halt_result(
    bdf: &str,
    seeder: &str,
    halted_at: &str,
    steps: Vec<crate::warm_init::WarmInitStep>,
    start: Instant,
) -> crate::warm_init::WarmInitResult {
    crate::warm_init::WarmInitResult {
        bdf: bdf.to_string(),
        success: false,
        halted_at: Some(halted_at.to_string()),
        seeder_used: seeder.to_string(),
        warm_preserved: false,
        steps,
        total_ms: start.elapsed().as_millis() as u64,
    }
}

impl SwapExecutor for SysfsSwapExecutor {
    type Error = SysfsSwapError;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        let bdf = Self::bdf_from_id(device)?;
        let start = Instant::now();
        let from = guarded_sysfs::read_current_driver(bdf)
            .unwrap_or_else(|| "unbound".to_string());

        let target_driver = Self::driver_name_for_target(target_personality);
        let warm_swap = Self::is_warm_preserving_swap(&from, target_driver);

        guarded_sysfs::pin_bridge_hierarchy(bdf);

        if warm_swap {
            guarded_sysfs::disable_flr(bdf);
            tracing::info!(bdf, from = from.as_str(), to = target_driver, "warm-preserving swap (FLR disabled)");
        }

        if let Some(ref current) = guarded_sysfs::read_current_driver(bdf) {
            let unbind_path = toadstool_cylinder::linux_paths::sysfs_pci_driver_unbind(current);
            tracing::info!(bdf, driver = current.as_str(), "unbinding current driver (guarded)");
            Self::sysfs_write_guarded(&unbind_path, bdf, guarded_sysfs::UNBIND_TIMEOUT)?;
        }

        if target_personality != "unbound" {
            let override_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "driver_override");
            Self::sysfs_write(&override_path, target_driver)?;

            let probe_path = toadstool_cylinder::linux_paths::sysfs_pci_drivers_probe();
            Self::sysfs_write_guarded(&probe_path, bdf, guarded_sysfs::PROBE_TIMEOUT)?;

            let bound = guarded_sysfs::read_current_driver(bdf);
            if bound.as_deref() != Some(target_driver) {
                return Err(SysfsSwapError::BindFailed {
                    bdf: bdf.into(),
                    driver: target_driver.into(),
                });
            }

            tracing::info!(bdf, driver = target_driver, "driver bound successfully");
        }

        let duration = start.elapsed();
        Ok(SwapObservation {
            device_id: device.short_label(),
            from,
            to: target_personality.into(),
            success: true,
            duration,
            error: None,
            detail: Some(serde_json::json!({
                "method": "sysfs",
                "bind_ms": duration.as_millis() as u64,
            })),
        })
    }

    async fn release(&self, device: &DeviceId) -> Result<(), Self::Error> {
        let bdf = Self::bdf_from_id(device)?;
        if let Some(ref current) = guarded_sysfs::read_current_driver(bdf) {
            let unbind_path = toadstool_cylinder::linux_paths::sysfs_pci_driver_unbind(current);
            Self::sysfs_write_guarded(&unbind_path, bdf, guarded_sysfs::UNBIND_TIMEOUT)?;
            tracing::info!(bdf, driver = current.as_str(), "device released (unbound, guarded)");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdf_from_pci_device_id() {
        let id = DeviceId::PciBdf("0000:01:00.0".into());
        assert_eq!(SysfsSwapExecutor::bdf_from_id(&id).unwrap(), "0000:01:00.0");
    }

    #[test]
    fn bdf_from_non_pci_errors() {
        let id = DeviceId::UsbPath("1-2".into());
        assert!(SysfsSwapExecutor::bdf_from_id(&id).is_err());
    }

    #[test]
    fn driver_name_mapping() {
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio-pci"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nouveau"), "nouveau");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nvidia-open"), "nvidia");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("amdgpu"), "amdgpu");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("xe"), "xe");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("akida"), "akida-pcie");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("custom"), "custom");
    }

    #[test]
    fn sysfs_swap_error_display() {
        let err = SysfsSwapError::NotPciBdf("usb:1-2".into());
        assert!(err.to_string().contains("PCI BDF"));

        let err = SysfsSwapError::SysfsWrite {
            path: "/sys/foo".into(),
            reason: "permission denied".into(),
        };
        assert!(err.to_string().contains("/sys/foo"));

        let err = SysfsSwapError::BindFailed {
            bdf: "0000:01:00.0".into(),
            driver: "vfio-pci".into(),
        };
        assert!(err.to_string().contains("vfio-pci"));
    }

    #[test]
    fn warm_preserving_swap_detection() {
        assert!(SysfsSwapExecutor::is_warm_preserving_swap("nouveau", "vfio-pci"));
        assert!(SysfsSwapExecutor::is_warm_preserving_swap("nvidia", "vfio-pci"));
        assert!(SysfsSwapExecutor::is_warm_preserving_swap("amdgpu", "vfio-pci"));
        assert!(SysfsSwapExecutor::is_warm_preserving_swap("xe", "vfio-pci"));
        assert!(!SysfsSwapExecutor::is_warm_preserving_swap("vfio-pci", "nouveau"));
        assert!(!SysfsSwapExecutor::is_warm_preserving_swap("vfio-pci", "vfio-pci"));
        assert!(!SysfsSwapExecutor::is_warm_preserving_swap("nouveau", "nvidia"));
        assert!(!SysfsSwapExecutor::is_warm_preserving_swap("unbound", "vfio-pci"));
    }

    #[tokio::test]
    async fn release_nonexistent_device_is_noop() {
        let exec = SysfsSwapExecutor;
        let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
        let result = exec.release(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_swap_nonexistent_device_errors() {
        let exec = SysfsSwapExecutor;
        let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
        let result = exec.execute_swap(&id, "vfio-pci").await;
        // No driver bound on nonexistent device, so it tries to write
        // driver_override and drivers_probe — which should fail (not hang)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_swap_unbound_target_succeeds_on_nonexistent() {
        let exec = SysfsSwapExecutor;
        let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
        // "unbound" target skips bind, so the swap is a no-op
        let result = exec.execute_swap(&id, "unbound").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to, "unbound");
    }

    #[test]
    fn halt_result_records_correct_step() {
        let steps = vec![
            crate::warm_init::WarmInitStep {
                name: "seeder_bind".into(),
                ok: false,
                detail: Some("guarded drivers_probe failed: timeout".into()),
                duration_ms: 30000,
            },
        ];
        let result = halt_result(
            "0000:02:00.0",
            "nouveau",
            "seeder_bind",
            steps,
            std::time::Instant::now(),
        );
        assert!(!result.success);
        assert_eq!(result.halted_at.as_deref(), Some("seeder_bind"));
        assert_eq!(result.seeder_used, "nouveau");
        assert!(!result.warm_preserved);
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].name, "seeder_bind");
        assert!(!result.steps[0].ok);
    }

    #[test]
    fn driver_name_mapping_exhaustive() {
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio-pci"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nouveau"), "nouveau");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nvidia"), "nvidia");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nvidia-open"), "nvidia");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nvidia_open"), "nvidia");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("amdgpu"), "amdgpu");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("xe"), "xe");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("i915"), "i915");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("akida"), "akida-pcie");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("akida-pcie"), "akida-pcie");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("unknown_driver"), "unknown_driver");
    }
}
