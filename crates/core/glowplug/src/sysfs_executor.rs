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

impl SysfsSwapExecutor {
    fn bdf_from_id(device: &DeviceId) -> Result<&str, SysfsSwapError> {
        match device {
            DeviceId::PciBdf(bdf) => Ok(bdf.as_str()),
            other => Err(SysfsSwapError::NotPciBdf(other.short_label())),
        }
    }

    fn read_current_driver(bdf: &str) -> Option<String> {
        let link = format!("/sys/bus/pci/devices/{bdf}/driver");
        std::fs::read_link(&link)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    fn sysfs_write(path: &str, value: &str) -> Result<(), SysfsSwapError> {
        std::fs::write(path, value).map_err(|e| SysfsSwapError::SysfsWrite {
            path: path.into(),
            reason: e.to_string(),
        })
    }

    /// Walk the sysfs device path upward, pinning `power/control=on` and
    /// `d3cold_allowed=0` on every ancestor PCI bridge. This prevents PLX
    /// (and similar PCIe switch) bridges from entering D3cold when the
    /// downstream endpoint is unbound — critical for the Tesla K80 whose
    /// PLX PEX 8747 fabric goes dark instantly on unbind.
    fn pin_bridge_hierarchy(bdf: &str) {
        let device_link = format!("/sys/bus/pci/devices/{bdf}");
        let Ok(canonical) = std::fs::canonicalize(&device_link) else {
            return;
        };

        let mut current = canonical.as_path().parent();
        while let Some(parent) = current {
            let Some(name) = parent.file_name().and_then(|n| n.to_str()) else {
                break;
            };

            if !name.contains(':') {
                break;
            }

            let control = parent.join("power/control");
            let d3cold = parent.join("d3cold_allowed");
            if control.exists() {
                let _ = std::fs::write(&control, "on");
                let _ = std::fs::write(&d3cold, "0");
                tracing::debug!(bridge = name, "pinned bridge power (d3cold_allowed=0)");
            }

            let pm_state_path = parent.join("power_state");
            if let Ok(state) = std::fs::read_to_string(&pm_state_path) {
                tracing::debug!(bridge = name, state = state.trim(), "bridge power state");
            }

            current = parent.parent();
        }

        let control = format!("/sys/bus/pci/devices/{bdf}/power/control");
        let d3cold = format!("/sys/bus/pci/devices/{bdf}/d3cold_allowed");
        let _ = std::fs::write(&control, "on");
        let _ = std::fs::write(&d3cold, "0");
        tracing::debug!(bdf, "pinned device power pre-swap");
    }

    /// Disable FLR (Function Level Reset) for a PCI device by clearing its
    /// `reset_method` sysfs attribute. When swapping from an initializing
    /// driver (nouveau/nvidia/amdgpu) to vfio-pci, FLR destroys the warm
    /// state that the driver set up (PRI Ring, clock trees, memory training).
    /// Clearing `reset_method` before the swap prevents this.
    ///
    /// Validated on Titan V (Exp 194): 27/27 registers preserved through
    /// nouveau→vfio-pci swap with FLR disabled.
    fn disable_flr(bdf: &str) {
        let path = format!("/sys/bus/pci/devices/{bdf}/reset_method");
        if std::path::Path::new(&path).exists() {
            match std::fs::write(&path, "") {
                Ok(()) => tracing::info!(bdf, "FLR disabled (reset_method cleared)"),
                Err(e) => tracing::warn!(bdf, error = %e, "failed to clear reset_method"),
            }
        }
    }

    /// Re-enable default reset methods for a PCI device after a warm swap
    /// is complete and the device is stable.
    #[expect(dead_code, reason = "called by future stable-state restore path")]
    fn restore_flr(bdf: &str) {
        let path = format!("/sys/bus/pci/devices/{bdf}/reset_method");
        if std::path::Path::new(&path).exists() {
            match std::fs::write(&path, "flr,bus") {
                Ok(()) => tracing::debug!(bdf, "reset_method restored to flr,bus"),
                Err(e) => tracing::debug!(bdf, error = %e, "could not restore reset_method"),
            }
        }
    }

    /// Returns `true` if this swap should preserve warm state (no-FLR).
    /// A warm-preserving swap goes FROM an initializing driver (nouveau,
    /// nvidia, amdgpu) TO vfio-pci, keeping the GPU's register state intact.
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

        // Step 1: Unbind current driver (if any)
        let t = Instant::now();
        let prev_driver = Self::read_current_driver(bdf);
        if let Some(ref current) = prev_driver {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            if let Err(e) = Self::sysfs_write(&unbind_path, bdf) {
                tracing::warn!(bdf, driver = current.as_str(), error = %e, "unbind failed (continuing)");
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
        let override_path = format!("/sys/bus/pci/devices/{bdf}/driver_override");
        if let Err(e) = Self::sysfs_write(&override_path, seeder_module) {
            steps.push(WarmInitStep {
                name: "seeder_bind".into(),
                ok: false,
                detail: Some(format!("driver_override failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "seeder_bind", steps, overall);
        }
        if let Err(e) = Self::sysfs_write("/sys/bus/pci/drivers_probe", bdf) {
            steps.push(WarmInitStep {
                name: "seeder_bind".into(),
                ok: false,
                detail: Some(format!("drivers_probe failed: {e}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return halt_result(bdf, &plan.seeder.name, "seeder_bind", steps, overall);
        }

        let bound = Self::read_current_driver(bdf);
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
        Self::pin_bridge_hierarchy(bdf);
        Self::disable_flr(bdf);
        steps.push(WarmInitStep {
            name: "prepare_warm_swap".into(),
            ok: true,
            detail: Some("bridge pinned, FLR disabled".into()),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // Step 5: Warm swap to final target (vfio-pci)
        let t = Instant::now();
        let final_driver = Self::driver_name_for_target(&plan.final_target);

        if let Some(ref current) = Self::read_current_driver(bdf) {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            if let Err(e) = Self::sysfs_write(&unbind_path, bdf) {
                steps.push(WarmInitStep {
                    name: "warm_swap".into(),
                    ok: false,
                    detail: Some(format!("unbind {current} failed: {e}")),
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
        let _ = Self::sysfs_write("/sys/bus/pci/drivers_probe", bdf);

        let final_bound = Self::read_current_driver(bdf);
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
        let from = Self::read_current_driver(bdf)
            .unwrap_or_else(|| "unbound".to_string());

        let target_driver = Self::driver_name_for_target(target_personality);
        let warm_swap = Self::is_warm_preserving_swap(&from, target_driver);

        Self::pin_bridge_hierarchy(bdf);

        if warm_swap {
            Self::disable_flr(bdf);
            tracing::info!(bdf, from = from.as_str(), to = target_driver, "warm-preserving swap (FLR disabled)");
        }

        if let Some(ref current) = Self::read_current_driver(bdf) {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            tracing::info!(bdf, driver = current.as_str(), "unbinding current driver");
            Self::sysfs_write(&unbind_path, bdf)?;
        }

        if target_personality != "unbound" {
            let override_path = format!("/sys/bus/pci/devices/{bdf}/driver_override");
            Self::sysfs_write(&override_path, target_driver)?;

            let probe_path = "/sys/bus/pci/drivers_probe";
            Self::sysfs_write(probe_path, bdf)?;

            let bound = Self::read_current_driver(bdf);
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
        if let Some(ref current) = Self::read_current_driver(bdf) {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            Self::sysfs_write(&unbind_path, bdf)?;
            tracing::info!(bdf, driver = current.as_str(), "device released (unbound)");
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
}
