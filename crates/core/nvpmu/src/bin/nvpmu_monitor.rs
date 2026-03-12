// SPDX-License-Identifier: AGPL-3.0-only
//! CLI tool: monitor GPU sensors (sovereign nvidia-smi replacement).
//!
//! Supports multiple sensor backends:
//! - nvidia-smi for proprietary NVIDIA drivers
//! - hwmon/sysfs for nouveau and AMD (amdgpu)
//!
//! Usage:
//!   nvpmu-monitor [--interval <ms>] [--json] [--once]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json");
    let once = args.iter().any(|a| a == "--once");
    let interval_ms: u64 = args
        .windows(2)
        .find(|w| w[0] == "--interval")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(1000);

    // Discover NVIDIA GPUs via PCI sysfs
    let pci_gpus = nvpmu::pci::discover_gpus().unwrap_or_default();

    // Try nvidia-smi for proprietary driver sensors
    let smi_available = nvpmu::nvidia_smi::discover_via_smi().is_ok();

    if !json_mode {
        eprintln!("nvPmu Monitor — Sovereign GPU Telemetry");
        eprintln!("========================================");
        eprintln!("PCI NVIDIA GPUs: {}", pci_gpus.len());
        eprintln!(
            "nvidia-smi: {}",
            if smi_available {
                "available"
            } else {
                "not available"
            }
        );
        for gpu in &pci_gpus {
            eprintln!(
                "  {} — {:04x}:{:04x} driver={} chip={}",
                gpu.bdf,
                gpu.vendor_id,
                gpu.device_id,
                gpu.driver.as_deref().unwrap_or("none"),
                gpu.chip.as_deref().unwrap_or("unknown"),
            );

            // Firmware inventory
            if let Ok(fw) = gpu.firmware() {
                eprintln!(
                    "    firmware: PMU={:?} GSP={:?} GR={:?} compute_viable={}",
                    fw.pmu, fw.gsp, fw.gr, fw.compute_viable()
                );
            }
        }

        // Also check for AMD GPUs via hwmon
        check_hwmon_devices(json_mode);

        if !once {
            eprintln!("\nPolling every {interval_ms}ms. Ctrl+C to stop.\n");
        }
    }

    loop {
        // NVIDIA via nvidia-smi (proprietary driver)
        if let Ok(smi_gpus) = nvpmu::nvidia_smi::discover_via_smi() {
            for gpu in &smi_gpus {
                if json_mode {
                    let report = serde_json::json!({
                        "source": "nvidia-smi",
                        "name": gpu.name,
                        "bdf": gpu.bdf,
                        "driver": gpu.driver_version,
                        "temp_c": gpu.temp_c,
                        "power_w": gpu.power_w,
                        "power_limit_w": gpu.power_limit_w,
                        "clock_mhz": gpu.clock_mhz,
                        "mem_clock_mhz": gpu.mem_clock_mhz,
                        "fan_pct": gpu.fan_pct,
                        "mem_used_mib": gpu.mem_used_mib,
                        "mem_total_mib": gpu.mem_total_mib,
                    });
                    println!("{report}");
                } else {
                    print!("[{}] {} ", gpu.bdf, gpu.name);
                    if let Some(t) = gpu.temp_c {
                        print!("{t:.0}°C ");
                    }
                    if let (Some(p), Some(l)) = (gpu.power_w, gpu.power_limit_w) {
                        print!("{p:.0}W/{l:.0}W ");
                    }
                    if let Some(c) = gpu.clock_mhz {
                        print!("{c}MHz ");
                    }
                    if let Some(f) = gpu.fan_pct {
                        print!("fan:{f}% ");
                    }
                    if let (Some(u), Some(t)) = (gpu.mem_used_mib, gpu.mem_total_mib) {
                        print!("{u}/{t}MiB ");
                    }
                    println!("(nvidia-smi)");
                }
            }
        }

        // NVIDIA GPUs via hwmon (nouveau driver)
        for gpu in &pci_gpus {
            if gpu.driver.as_deref() == Some("nouveau") {
                match gpu.sensors() {
                    Ok(sensors) => print_hwmon_sensors(
                        &gpu.bdf,
                        gpu.chip.as_deref().unwrap_or("nvidia"),
                        &sensors,
                        json_mode,
                    ),
                    Err(e) if !json_mode => eprintln!("[{}] hwmon error: {e}", gpu.bdf),
                    _ => {}
                }
            }
        }

        // AMD GPUs via hwmon
        read_amd_hwmon(json_mode);

        if once {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
    }
}

fn print_hwmon_sensors(bdf: &str, name: &str, sensors: &nvpmu::HwmonSensors, json_mode: bool) {
    if json_mode {
        let report = serde_json::json!({
            "source": "hwmon",
            "bdf": bdf,
            "name": name,
            "temp_c": sensors.temp_c(),
            "power_w": sensors.power_w(),
            "clock_mhz": sensors.clock_mhz,
            "mem_clock_mhz": sensors.mem_clock_mhz,
            "fan_rpm": sensors.fan_rpm,
        });
        println!("{report}");
    } else {
        print!("[{bdf}] {name} ");
        if let Some(t) = sensors.temp_c() {
            print!("{t:.0}°C ");
        }
        if let Some(p) = sensors.power_w() {
            print!("{p:.1}W ");
        }
        if let Some(c) = sensors.clock_mhz {
            print!("{c}MHz ");
        }
        if let Some(f) = sensors.fan_rpm {
            print!("{f}RPM ");
        }
        println!("(hwmon)");
    }
}

fn check_hwmon_devices(json_mode: bool) {
    if json_mode {
        return;
    }
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let name_path = entry.path().join("name");
            if let Ok(name) = std::fs::read_to_string(&name_path) {
                let name = name.trim();
                if name == "amdgpu" {
                    eprintln!("  AMD GPU at {} (amdgpu hwmon)", entry.path().display());
                }
            }
        }
    }
}

fn read_amd_hwmon(json_mode: bool) {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return;
    };
    for entry in entries.flatten() {
        let name_path = entry.path().join("name");
        let Ok(name) = std::fs::read_to_string(&name_path) else {
            continue;
        };
        if name.trim() != "amdgpu" {
            continue;
        }
        let hwmon_path = entry.path();
        match nvpmu::HwmonSensors::from_hwmon_path(&hwmon_path) {
            Ok(sensors) => {
                // Read AMD-specific multi-sensor data
                let junction = read_sensor_opt(&hwmon_path, "temp2_input");
                let mem_temp = read_sensor_opt(&hwmon_path, "temp3_input");
                let vddgfx = read_sensor_opt(&hwmon_path, "in0_input");

                if json_mode {
                    let report = serde_json::json!({
                        "source": "hwmon",
                        "name": "amdgpu",
                        "hwmon": hwmon_path.display().to_string(),
                        "temp_edge_c": sensors.temp_c(),
                        "temp_junction_c": junction.map(|v| v as f64 / 1000.0),
                        "temp_mem_c": mem_temp.map(|v| v as f64 / 1000.0),
                        "power_w": sensors.power_w(),
                        "clock_mhz": sensors.clock_mhz,
                        "mem_clock_mhz": sensors.mem_clock_mhz,
                        "fan_rpm": sensors.fan_rpm,
                        "vddgfx_mv": vddgfx,
                    });
                    println!("{report}");
                } else {
                    print!("[amdgpu] ");
                    if let Some(t) = sensors.temp_c() {
                        print!("edge:{t:.0}°C ");
                    }
                    if let Some(j) = junction {
                        print!("junc:{:.0}°C ", j as f64 / 1000.0);
                    }
                    if let Some(m) = mem_temp {
                        print!("mem:{:.0}°C ", m as f64 / 1000.0);
                    }
                    if let Some(p) = sensors.power_w() {
                        print!("{p:.1}W ");
                    }
                    if let Some(c) = sensors.clock_mhz {
                        print!("{c}MHz ");
                    }
                    if let Some(f) = sensors.fan_rpm {
                        print!("{f}RPM ");
                    }
                    if let Some(v) = vddgfx {
                        print!("{v}mV ");
                    }
                    println!("(hwmon)");
                }
            }
            Err(_) => {
                if !json_mode {
                    eprintln!("[amdgpu] sensors unavailable (suspended?)");
                }
            }
        }
    }
}

fn read_sensor_opt(hwmon: &std::path::Path, name: &str) -> Option<i64> {
    std::fs::read_to_string(hwmon.join(name))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}
