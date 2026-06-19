// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU mode switching for single-GPU systems.
//!
//! Switches between gaming mode (nvidia/nouveau for display) and science
//! mode (vfio-pci for sovereign compute dispatch).

use std::path::PathBuf;

use nvpmu::vfio_bind::BindingState;
use toadstool_common::pci_discovery::{PciFilter, discover_pci_devices};

use crate::Result;

use super::definitions::ModeCommand;

/// Path for persisting original driver when switching to science mode.
/// Used when switching back to gaming mode so we know which driver to restore.
fn gpu_mode_state_path(bdf: &str) -> PathBuf {
    let sanitized = bdf.replace(':', "-");
    std::env::temp_dir().join(format!("toadstool-gpu-mode-{sanitized}"))
}

/// Auto-detect the first NVIDIA GPU if no BDF specified.
fn resolve_bdf(bdf: Option<String>) -> Result<String> {
    if let Some(bdf) = bdf {
        return Ok(bdf);
    }
    let filter =
        PciFilter::vendor(toadstool_common::pci_discovery::vendors::NVIDIA).with_class(|c| {
            let masked = c & 0x00FF_FF00;
            masked == 0x0003_0000 || masked == 0x0003_0200
        });
    let devices = discover_pci_devices(&filter);
    let gpu = devices.first().ok_or_else(|| {
        crate::CliError::Other("No NVIDIA GPU found. Specify --bdf manually.".into())
    })?;
    Ok(gpu.bdf.clone())
}

fn binding_state_driver_name(state: &BindingState) -> &str {
    match state {
        BindingState::VfioPci => "vfio-pci",
        BindingState::KernelDriver(name) => name.as_str(),
        BindingState::Unbound => "none",
    }
}

/// Execute mode switching command.
pub async fn execute_mode_command(_ctx: &crate::CliContext, cmd: ModeCommand) -> Result<()> {
    match cmd {
        ModeCommand::Science { bdf } => {
            let bdf = resolve_bdf(bdf)?;
            println!("Switching {bdf} to science mode (vfio-pci)...");

            let current = nvpmu::vfio_bind::current_binding(&bdf)
                .map_err(|e| crate::CliError::Other(format!("Failed to query binding: {e}")))?;
            println!("  Current driver: {}", binding_state_driver_name(&current));

            if matches!(current, BindingState::VfioPci) {
                println!("  Already in science mode.");
                return Ok(());
            }

            let result = nvpmu::vfio_bind::bind_vfio(&bdf)
                .map_err(|e| crate::CliError::Other(format!("Failed to bind vfio: {e}")))?;
            println!(
                "  Bound to vfio-pci (was: {})",
                binding_state_driver_name(&result.previous)
            );
            println!("  Science mode active. GPU ready for sovereign dispatch.");

            if let BindingState::KernelDriver(ref driver) = result.previous {
                let path = gpu_mode_state_path(&bdf);
                if let Err(e) = tokio::fs::write(&path, driver.as_bytes()).await {
                    tracing::warn!(?path, %e, "Could not persist original driver for gaming restore");
                }
            }
        }
        ModeCommand::Gaming { bdf } => {
            let bdf = resolve_bdf(bdf)?;
            println!("Switching {bdf} to gaming mode...");

            let current = nvpmu::vfio_bind::current_binding(&bdf)
                .map_err(|e| crate::CliError::Other(format!("Failed to query binding: {e}")))?;
            if !matches!(current, BindingState::VfioPci) {
                println!(
                    "  Not in science mode (driver: {}). Nothing to do.",
                    binding_state_driver_name(&current)
                );
                return Ok(());
            }

            let state_path = gpu_mode_state_path(&bdf);
            let original_driver = tokio::fs::read_to_string(&state_path)
                .await
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let original_driver = original_driver.unwrap_or_else(|| {
                if std::path::Path::new(&toadstool_cylinder::linux_paths::sysfs_join(&[
                    "bus", "pci", "drivers", "nvidia",
                ]))
                .exists()
                {
                    "nvidia".to_string()
                } else {
                    "nouveau".to_string()
                }
            });

            nvpmu::vfio_bind::unbind_vfio(&bdf, &original_driver)
                .map_err(|e| crate::CliError::Other(format!("Failed to unbind vfio: {e}")))?;
            println!("  Unbound from vfio-pci. Display driver will rebind.");
            println!("  Gaming mode active.");

            let path = gpu_mode_state_path(&bdf);
            let _ = tokio::fs::remove_file(&path).await;
        }
        ModeCommand::Status { bdf } => {
            let bdf = resolve_bdf(bdf)?;
            let current = nvpmu::vfio_bind::current_binding(&bdf)
                .map_err(|e| crate::CliError::Other(format!("Failed to query binding: {e}")))?;
            let mode = match &current {
                BindingState::VfioPci => "science",
                BindingState::KernelDriver(name) => {
                    if name == "nvidia" || name == "nouveau" || name == "amdgpu" {
                        "gaming"
                    } else {
                        name.as_str()
                    }
                }
                BindingState::Unbound => "unbound",
            };

            let power = nvpmu::GpuPowerController::new(&bdf);
            let power_state = power
                .power_state()
                .map(|s| format!("{s}"))
                .unwrap_or_else(|_| "unknown".to_string());

            println!("GPU {bdf}:");
            println!("  Mode:        {mode}");
            println!("  Driver:      {}", binding_state_driver_name(&current));
            println!("  Power state: {power_state}");
            println!("  Supports reset: {}", power.supports_reset());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_mode_state_path_sanitizes_colons() {
        let path = gpu_mode_state_path("0000:01:00.0");
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("0000-01-00.0")
        );
        assert!(path.to_string_lossy().contains("toadstool-gpu-mode"));
    }

    #[test]
    fn binding_state_driver_name_maps_all_variants() {
        assert_eq!(
            binding_state_driver_name(&BindingState::VfioPci),
            "vfio-pci"
        );
        assert_eq!(
            binding_state_driver_name(&BindingState::KernelDriver("nvidia".into())),
            "nvidia"
        );
        assert_eq!(binding_state_driver_name(&BindingState::Unbound), "none");
    }

    #[test]
    fn resolve_bdf_returns_explicit_value() {
        let bdf = resolve_bdf(Some("0000:42:00.0".into())).expect("explicit bdf");
        assert_eq!(bdf, "0000:42:00.0");
    }

    #[tokio::test]
    async fn mode_status_with_explicit_bdf_succeeds_or_reports_binding_error() {
        let ctx = crate::CliContext {
            config_path: None,
            working_dir: std::env::current_dir().expect("cwd"),
            verbose: false,
        };
        let result = execute_mode_command(
            &ctx,
            ModeCommand::Status {
                bdf: Some("ffff:ff:00.0".into()),
            },
        )
        .await;
        match result {
            Ok(()) => {}
            Err(crate::CliError::Other(msg)) => {
                assert!(
                    msg.contains("binding") || msg.contains("query"),
                    "unexpected error: {msg}"
                );
            }
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }
}
