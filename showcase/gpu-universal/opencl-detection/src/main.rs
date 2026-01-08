use anyhow::Result;
use ocl::{Platform, Device, DeviceType};

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  OpenCL Vendor-Agnostic Detection Test (Rust)               ║");
    println!("║  Bypassing Python - Direct API Access                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Enumerate all OpenCL platforms
    let platforms = Platform::list();
    
    println!("🔍 Discovered {} OpenCL platform(s):", platforms.len());
    println!();

    let mut total_devices = 0;
    let mut nvidia_found = false;
    let mut amd_found = false;

    for (platform_idx, platform) in platforms.iter().enumerate() {
        let platform_name = platform.name().unwrap_or_else(|_| "Unknown".to_string());
        let platform_vendor = platform.vendor().unwrap_or_else(|_| "Unknown".to_string());
        let platform_version = platform.version().unwrap_or_else(|_| "Unknown".to_string());

        println!("──────────────────────────────────────────────────────────────");
        println!("Platform {}: {}", platform_idx, platform_name);
        println!("──────────────────────────────────────────────────────────────");
        println!("  Vendor:  {}", platform_vendor);
        println!("  Version: {}", platform_version);
        println!();

        // Enumerate devices for this platform
        match Device::list_all(*platform) {
            Ok(devices) if !devices.is_empty() => {
                println!("  📱 Devices ({}):", devices.len());
                println!();
                
                for (device_idx, device) in devices.iter().enumerate() {
                    let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                    let device_vendor = device.vendor().unwrap_or_else(|_| "Unknown".to_string());
                    let device_type = device.info(ocl::enums::DeviceInfo::Type)
                        .unwrap_or_else(|_| ocl::enums::DeviceInfoResult::Type(DeviceType::empty()));
                    
                    let global_mem_size = device.info(ocl::enums::DeviceInfo::GlobalMemSize)
                        .and_then(|info| {
                            if let ocl::enums::DeviceInfoResult::GlobalMemSize(size) = info {
                                Ok(size)
                            } else {
                                Err(ocl::Error::from("Unexpected info type"))
                            }
                        })
                        .unwrap_or(0);
                    
                    let max_compute_units = device.info(ocl::enums::DeviceInfo::MaxComputeUnits)
                        .and_then(|info| {
                            if let ocl::enums::DeviceInfoResult::MaxComputeUnits(units) = info {
                                Ok(units)
                            } else {
                                Err(ocl::Error::from("Unexpected info type"))
                            }
                        })
                        .unwrap_or(0);
                    
                    let max_clock_freq = device.info(ocl::enums::DeviceInfo::MaxClockFrequency)
                        .and_then(|info| {
                            if let ocl::enums::DeviceInfoResult::MaxClockFrequency(freq) = info {
                                Ok(freq)
                            } else {
                                Err(ocl::Error::from("Unexpected info type"))
                            }
                        })
                        .unwrap_or(0);

                    println!("    [{}] {}", device_idx, device_name);
                    println!("        Type:          {:?}", device_type);
                    println!("        Vendor:        {}", device_vendor);
                    println!("        Memory:        {:.1} GB", global_mem_size as f64 / 1e9);
                    println!("        Compute Units: {}", max_compute_units);
                    println!("        Clock:         {} MHz", max_clock_freq);
                    println!();

                    // Track vendor detection
                    if device_name.to_lowercase().contains("nvidia") || 
                       device_vendor.to_lowercase().contains("nvidia") {
                        nvidia_found = true;
                    }
                    if device_name.to_lowercase().contains("amd") || 
                       device_vendor.to_lowercase().contains("amd") ||
                       device_name.contains("gfx") {
                        amd_found = true;
                    }

                    total_devices += 1;
                }
            },
            Ok(_) => {
                println!("  ⚠️  No devices found on this platform");
                println!();
            },
            Err(e) => {
                println!("  ❌ Error enumerating devices: {}", e);
                println!();
            }
        }
    }

    println!("══════════════════════════════════════════════════════════════");
    println!("📊 SUMMARY");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("  Total Platforms: {}", platforms.len());
    println!("  Total Devices:   {}", total_devices);
    println!();
    println!("  NVIDIA GPU:      {}", if nvidia_found { "✅ DETECTED" } else { "❌ NOT FOUND" });
    println!("  AMD GPU:         {}", if amd_found { "✅ DETECTED" } else { "❌ NOT FOUND" });
    println!();

    if nvidia_found && amd_found {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  🎉 SUCCESS: BOTH GPUS DETECTED VIA OPENCL                  ║");
        println!("║  Vendor-Agnostic Compute: READY ✅                          ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else if nvidia_found || amd_found {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ⚠️  PARTIAL SUCCESS: One GPU detected                      ║");
        println!("║  Check hardware and drivers                                 ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ❌ NO GPUS DETECTED                                        ║");
        println!("║  Check OpenCL installation and drivers                      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    println!();
    println!("💡 Key Insight:");
    println!("   This Rust code bypasses Python binding issues and directly");
    println!("   accesses OpenCL APIs, proving vendor-agnostic GPU detection.");
    println!();

    Ok(())
}
