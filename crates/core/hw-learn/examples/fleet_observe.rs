// SPDX-License-Identifier: AGPL-3.0-only
//! Fleet observation tool — discover GPUs, probe firmware, identify learning pairs.
//!
//! Run on any machine to:
//! 1. Discover all GPUs via sysmon
//! 2. Probe firmware inventory for each
//! 3. Identify teacher/student pairs via `LearningAdvisor`
//! 4. Output a fleet report as JSON
//!
//! ## eastgate usage (RTX 4070 + Titan V)
//!
//! ```bash
//! cargo run -p hw-learn --example fleet_observe
//! cargo run -p hw-learn --example fleet_observe -- --json > fleet_report.json
//! ```
//!
//! ## GSP trace capture (requires root + mmiotrace)
//!
//! ```bash
//! # Enable mmiotrace (requires root)
//! sudo sh -c 'echo mmiotrace > /sys/kernel/tracing/current_tracer'
//! # Trigger GPU load (e.g., vulkaninfo or a compute shader)
//! vulkaninfo > /dev/null 2>&1
//! # Capture trace
//! sudo cat /sys/kernel/tracing/trace > /tmp/gpu_trace.log
//! # Disable mmiotrace
//! sudo sh -c 'echo nop > /sys/kernel/tracing/current_tracer'
//! # Parse trace
//! cargo run -p hw-learn --example fleet_observe -- --trace /tmp/gpu_trace.log
//! ```

use hw_learn::brain_ext::learning_advisor::{FleetGpu, LearningAdvisor};
use hw_learn::distiller::{GpuArch, Vendor};
use hw_learn::observer::{GpuSelector, ObserveConfig, TraceMode, TraceObserver};
use toadstool_sysmon::{discover_gpus, GpuVendor};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.contains(&"--json".to_string());
    let trace_path = args
        .windows(2)
        .find(|w| w[0] == "--trace")
        .map(|w| w[1].clone());

    println!("=== hwLearn Fleet Observer ===\n");

    // Step 1: Discover GPUs
    let gpus = discover_gpus();
    if gpus.is_empty() {
        println!("No GPUs found via /sys/class/drm/");
        return;
    }

    println!("Discovered {} GPU(s):\n", gpus.len());

    let mut fleet = Vec::new();

    for gpu in &gpus {
        let fw = gpu.firmware_inventory();
        let topo = gpu.pcie_topology();

        println!(
            "  card{}: {} {:04x}",
            gpu.card_index, gpu.vendor, gpu.device_id
        );
        println!("    driver: {}", gpu.driver);
        println!("    PCI: {}", gpu.pci_slot);
        println!("    PCIe: gen{:?} x{:?}", topo.gen, topo.width);
        println!("    firmware:");
        println!("      PMU: {}", fw.pmu);
        println!("      GSP: {}", fw.gsp);
        println!("      ACR: {}", fw.acr);
        println!("      GR:  {}", fw.gr);
        println!("      SEC2: {}", fw.sec2);
        println!("      GuC: {}", fw.guc);
        println!("      HuC: {}", fw.huc);
        println!("    compute viable: {}", fw.compute_viable);
        if let Some(reason) = &fw.blocking_reason {
            println!("    blocking: {reason}");
        }
        println!();

        let vendor = match gpu.vendor {
            GpuVendor::Amd => Vendor::Amd,
            GpuVendor::Intel => Vendor::Intel,
            GpuVendor::Nvidia => Vendor::Nvidia,
            GpuVendor::Unknown => continue,
        };

        fleet.push(FleetGpu {
            id: format!("card{}", gpu.card_index),
            arch: GpuArch {
                vendor,
                generation: infer_gen(&gpu.driver, gpu.device_id),
                chip: format!("dev{:04x}", gpu.device_id),
                compute_class: infer_cc(&gpu.driver, gpu.device_id),
            },
            firmware: fw,
            compute_works: gpu.firmware_inventory().compute_viable,
            driver: gpu.driver.clone(),
        });
    }

    // Step 2: Learning analysis
    let advisor = LearningAdvisor::new(fleet);
    let summary = advisor.fleet_summary();

    println!("--- Fleet Summary ---");
    println!("  Total GPUs: {}", summary.total_gpus);
    println!("  Working:    {}", summary.working);
    println!("  Blocked:    {}", summary.blocked);
    println!();

    let opportunities = advisor.opportunities();
    if opportunities.is_empty() {
        println!("No learning opportunities found.");
        if summary.blocked == 0 {
            println!("All GPUs have working compute — this fleet is fully operational.");
        }
    } else {
        println!("--- Learning Opportunities ({}) ---\n", opportunities.len());
        for (i, opp) in opportunities.iter().enumerate() {
            println!("  {}. {} → {}", i + 1, opp.teacher, opp.student);
            println!("     confidence: {:.1}%", opp.confidence * 100.0);
            println!("     cross-vendor: {}", opp.cross_vendor);
            println!("     gap: {}", opp.gap);
            println!("     rationale: {}", opp.rationale);
            println!();
        }
    }

    // Step 3: If trace file provided, parse it
    if let Some(trace) = trace_path {
        println!("--- Parsing trace: {trace} ---\n");
        let config = ObserveConfig {
            gpu_selector: GpuSelector::Auto,
            mode: TraceMode::MmioTrace,
            trace_path: Some(trace.into()),
            trigger_compute: false,
        };
        match TraceObserver::observe(&config) {
            Ok(result) => {
                println!("  GPU: {}", result.gpu_id);
                println!("  Driver: {}", result.driver);
                println!("  Events: {}", result.events.len());
                println!("  Duration: {}us", result.duration_us);

                if json_mode {
                    if let Ok(json) = serde_json::to_string_pretty(&result) {
                        println!("\n{json}");
                    }
                }
            }
            Err(e) => {
                eprintln!("  Trace parse error: {e}");
            }
        }
    }

    // Step 4: JSON output
    if json_mode {
        println!("\n--- JSON Report ---");
        let report = serde_json::json!({
            "gpus": gpus.iter().map(|g| {
                let fw = g.firmware_inventory();
                serde_json::json!({
                    "card": g.card_index,
                    "vendor": g.vendor.to_string(),
                    "device_id": format!("0x{:04x}", g.device_id),
                    "driver": g.driver,
                    "pci_slot": g.pci_slot,
                    "firmware": {
                        "pmu": fw.pmu.to_string(),
                        "gsp": fw.gsp.to_string(),
                        "acr": fw.acr.to_string(),
                        "gr": fw.gr.to_string(),
                        "sec2": fw.sec2.to_string(),
                        "guc": fw.guc.to_string(),
                        "huc": fw.huc.to_string(),
                    },
                    "compute_viable": fw.compute_viable,
                    "blocking_reason": fw.blocking_reason,
                })
            }).collect::<Vec<_>>(),
            "summary": {
                "total": summary.total_gpus,
                "working": summary.working,
                "blocked": summary.blocked,
            },
            "opportunities": opportunities.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    }
}

fn infer_gen(driver: &str, device_id: u32) -> String {
    if driver.contains("nouveau") || driver.contains("nvidia") {
        match device_id {
            0x1D81..=0x1DFF => "Volta".into(),
            0x1E02..=0x1FFF | 0x2182..=0x21FF => "Turing".into(),
            0x2200..=0x25FF => "Ampere".into(),
            0x2600..=0x28FF => "Ada".into(),
            _ => "Unknown-NV".into(),
        }
    } else if driver.contains("amdgpu") {
        "RDNA".into()
    } else if driver.contains("i915") || driver.contains("xe") {
        "Gen12+".into()
    } else {
        "Unknown".into()
    }
}

fn infer_cc(driver: &str, device_id: u32) -> String {
    if driver.contains("nouveau") || driver.contains("nvidia") {
        match device_id {
            0x1D81..=0x1DFF => "sm70".into(),
            0x1E02..=0x1FFF => "sm75".into(),
            0x2200..=0x25FF => "sm86".into(),
            0x2600..=0x28FF => "sm89".into(),
            _ => "unknown".into(),
        }
    } else if driver.contains("amdgpu") {
        "gfx1030".into()
    } else {
        "unknown".into()
    }
}
