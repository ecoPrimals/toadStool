// SPDX-License-Identifier: AGPL-3.0-or-later
//! Device lifecycle CLI — coralctl-equivalent commands.
//!
//! Provides `toadstool device swap|list|status|warm` subcommands that
//! invoke the same sysfs / glowplug operations as the JSON-RPC
//! `device.*` handlers.

use toadstool_common::pci_discovery::{PciFilter, discover_pci_devices};

use crate::Result;

use super::definitions::DeviceCommand;

fn is_gpu_or_npu_class(class: u32) -> bool {
    let masked = class & 0x00FF_FF00;
    masked == 0x0003_0000 // VGA
        || masked == 0x0003_0200 // 3D controller
        || masked == 0x0012_0000 // Processing accelerator (NPU)
}

fn read_current_driver(bdf: &str) -> String {
    let link = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unbound".into())
}

fn read_power_state(bdf: &str) -> String {
    let path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "power_state");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

pub async fn execute_device_command(cmd: DeviceCommand) -> Result<()> {
    match cmd {
        DeviceCommand::Swap {
            bdf,
            target,
            format,
        } => {
            println!("Swapping {bdf} → {target}...");

            let executor = toadstool_glowplug::SysfsSwapExecutor;
            let orchestrator = toadstool_glowplug::SwapOrchestrator::new(executor);
            let device = toadstool_glowplug::DeviceId::PciBdf(bdf.clone());
            let current = read_current_driver(&bdf);

            let result = orchestrator
                .execute_boot(&device, Some(&current), &target)
                .await;

            if format == "json" {
                let json = serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{json}");
            } else {
                println!("  Result:  {}", if result.success { "OK" } else { "FAILED" });
                println!("  Summary: {}", result.summary);
                for step in &result.steps {
                    let icon = match step.status {
                        toadstool_glowplug::StepStatus::Ok => "ok",
                        toadstool_glowplug::StepStatus::Failed => "FAIL",
                        toadstool_glowplug::StepStatus::Skipped => "skip",
                    };
                    println!(
                        "    [{icon}] {} ({}ms){}",
                        step.name,
                        step.duration_ms,
                        step.detail
                            .as_ref()
                            .map(|d| format!(" — {d}"))
                            .unwrap_or_default()
                    );
                }
            }
        }

        DeviceCommand::List { format } => {
            let filter = PciFilter::default().with_class(is_gpu_or_npu_class);
            let devices = discover_pci_devices(&filter);

            if format == "json" {
                let entries: Vec<serde_json::Value> = devices
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "bdf": d.bdf,
                            "vendor": format!("0x{:04x}", d.vendor_id),
                            "device": format!("0x{:04x}", d.device_id),
                            "class": format!("0x{:06x}", d.class_code),
                            "driver": read_current_driver(&d.bdf),
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
            } else if devices.is_empty() {
                println!("No GPU/NPU devices found.");
            } else {
                println!("{:<16} {:>6} {:>6} {:>10} CLASS", "BDF", "VEN", "DEV", "DRIVER");
                for d in &devices {
                    println!(
                        "{:<16} 0x{:04x} 0x{:04x} {:>10} 0x{:06x}",
                        d.bdf,
                        d.vendor_id,
                        d.device_id,
                        read_current_driver(&d.bdf),
                        d.class_code,
                    );
                }
            }
        }

        DeviceCommand::Status { bdf, format } => {
            let bdf = if let Some(bdf) = bdf {
                bdf
            } else {
                let filter = PciFilter::vendor(
                    toadstool_common::pci_discovery::vendors::NVIDIA,
                )
                .with_class(|c| (c & 0x00FF_FF00) == 0x0003_0000 || (c & 0x00FF_FF00) == 0x0003_0200);
                let devices = discover_pci_devices(&filter);
                devices
                    .first()
                    .map(|d| d.bdf.clone())
                    .ok_or_else(|| crate::CliError::Other("No GPU found. Specify --bdf.".into()))?
            };

            let driver = read_current_driver(&bdf);
            let power = read_power_state(&bdf);
            let vendor_id = toadstool_ember::sysfs::read_pci_id(&bdf, "vendor");
            let device_id = toadstool_ember::sysfs::read_pci_id(&bdf, "device");

            if format == "json" {
                let status = serde_json::json!({
                    "bdf": bdf,
                    "driver": driver,
                    "power_state": power,
                    "vendor_id": format!("0x{vendor_id:04x}"),
                    "device_id": format!("0x{device_id:04x}"),
                });
                println!("{}", serde_json::to_string_pretty(&status).unwrap_or_default());
            } else {
                println!("Device {bdf}:");
                println!("  Vendor:      0x{vendor_id:04x}");
                println!("  Device:      0x{device_id:04x}");
                println!("  Driver:      {driver}");
                println!("  Power state: {power}");
            }
        }

        DeviceCommand::Warm { bdf } => {
            let config_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(&bdf, "config");
            let pmc_enable = std::fs::File::open(&config_path)
                .and_then(|f| {
                    use std::io::{Read, Seek, SeekFrom};
                    let mut f = f;
                    f.seek(SeekFrom::Start(0x200))?;
                    let mut buf = [0u8; 4];
                    f.read_exact(&mut buf)?;
                    Ok(u32::from_le_bytes(buf))
                })
                .unwrap_or(0);

            let popcount = pmc_enable.count_ones();
            let warm = popcount > 4;
            let resource0_exists = std::path::Path::new(
                &toadstool_cylinder::linux_paths::sysfs_pci_device_file(&bdf, "resource0"),
            )
            .exists();

            println!("Warm detection for {bdf}:");
            println!("  PMC_ENABLE:     0x{pmc_enable:08x}");
            println!("  Popcount:       {popcount}");
            println!("  Warm detected:  {warm}");
            println!("  BAR0 resource:  {resource0_exists}");
        }
    }
    Ok(())
}
