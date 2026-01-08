use anyhow::Result;
use ocl::{Platform, Device, Context, Queue};
use ocl_core::{self as core, ContextProperties};

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  AMD OpenCL Debug Tool - Investigating Context Creation     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let platforms = Platform::list();
    
    for (idx, platform) in platforms.iter().enumerate() {
        let platform_name = platform.name().unwrap_or_else(|_| "Unknown".to_string());
        
        // Focus on AMD platform
        if !platform_name.contains("AMD") {
            continue;
        }

        println!("══════════════════════════════════════════════════════════════");
        println!("Platform {}: {}", idx, platform_name);
        println!("══════════════════════════════════════════════════════════════");
        println!();

        let platform_vendor = platform.vendor().unwrap_or_else(|_| "Unknown".to_string());
        let platform_version = platform.version().unwrap_or_else(|_| "Unknown".to_string());
        
        println!("Platform Details:");
        println!("  Vendor:  {}", platform_vendor);
        println!("  Version: {}", platform_version);
        println!();

        // Get devices
        let devices = match Device::list_all(*platform) {
            Ok(devs) => devs,
            Err(e) => {
                println!("❌ Failed to list devices: {}", e);
                continue;
            }
        };

        if devices.is_empty() {
            println!("⚠️  No devices found on this platform");
            continue;
        }

        for (device_idx, device) in devices.iter().enumerate() {
            let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            
            println!("──────────────────────────────────────────────────────────────");
            println!("Device {}: {}", device_idx, device_name);
            println!("──────────────────────────────────────────────────────────────");
            println!();

            // Print detailed device info
            print_device_info(device);
            println!();

            // Test 1: High-level Context creation (ocl crate)
            println!("🔧 Test 1: High-level Context (ocl::Context::builder())");
            match Context::builder()
                .devices(*device)
                .build() 
            {
                Ok(context) => {
                    println!("  ✅ SUCCESS: Context created");
                    
                    // Try to create a queue
                    println!();
                    println!("🔧 Test 2: Queue creation");
                    match Queue::new(&context, *device, None) {
                        Ok(_queue) => {
                            println!("  ✅ SUCCESS: Queue created");
                            println!();
                            println!("╔══════════════════════════════════════════════════════════════╗");
                            println!("║  🎉 AMD GPU OPENCL WORKING!                                  ║");
                            println!("╚══════════════════════════════════════════════════════════════╝");
                        },
                        Err(e) => {
                            println!("  ❌ FAILED: Queue creation");
                            println!("  Error: {}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("  ❌ FAILED: Context creation");
                    println!("  Error: {}", e);
                    println!();

                    // Test 2: Low-level context creation with explicit properties
                    println!("🔧 Test 3: Low-level Context (with platform property)");
                    match test_low_level_context(*platform, device) {
                        Ok(()) => {
                            println!("  ✅ SUCCESS: Low-level context created");
                        },
                        Err(e) => {
                            println!("  ❌ FAILED: Low-level context");
                            println!("  Error: {}", e);
                        }
                    }
                    println!();

                    // Test 3: Context without properties
                    println!("🔧 Test 4: Context with minimal setup");
                    match test_minimal_context(device) {
                        Ok(()) => {
                            println!("  ✅ SUCCESS: Minimal context created");
                        },
                        Err(e) => {
                            println!("  ❌ FAILED: Minimal context");
                            println!("  Error: {}", e);
                        }
                    }
                }
            }
            println!();
        }
    }

    Ok(())
}

fn print_device_info(device: &Device) {
    println!("Device Information:");
    
    if let Ok(device_type) = device.info(ocl::enums::DeviceInfo::Type) {
        println!("  Type: {:?}", device_type);
    }
    
    if let Ok(vendor) = device.vendor() {
        println!("  Vendor: {}", vendor);
    }
    
    if let Ok(version) = device.version() {
        println!("  Version: {}", version);
    }
    
    if let Ok(driver_version) = device.info(ocl::enums::DeviceInfo::DriverVersion) {
        if let ocl::enums::DeviceInfoResult::DriverVersion(v) = driver_version {
            println!("  Driver: {}", v);
        }
    }
    
    // Check if device is available
    if let Ok(available) = device.info(ocl::enums::DeviceInfo::Available) {
        if let ocl::enums::DeviceInfoResult::Available(is_avail) = available {
            println!("  Available: {}", is_avail);
            if !is_avail {
                println!("  ⚠️  WARNING: Device reports as NOT AVAILABLE!");
            }
        }
    }
    
    // Check compiler availability
    if let Ok(compiler_available) = device.info(ocl::enums::DeviceInfo::CompilerAvailable) {
        if let ocl::enums::DeviceInfoResult::CompilerAvailable(is_avail) = compiler_available {
            println!("  Compiler Available: {}", is_avail);
            if !is_avail {
                println!("  ⚠️  WARNING: OpenCL compiler NOT available!");
            }
        }
    }
}

fn test_low_level_context(platform: Platform, device: &Device) -> Result<()> {
    // Create context properties with explicit platform
    let properties = ContextProperties::new().platform(platform);
    
    // Try to create context with explicit properties
    let context = core::create_context(
        Some(&properties),
        &[device],
        None,
        None,
    )?;
    
    println!("  Context created: {:?}", context);
    Ok(())
}

fn test_minimal_context(device: &Device) -> Result<()> {
    // Try with no properties at all
    let context = core::create_context(
        None,  // No properties
        &[device],
        None,
        None,
    )?;
    
    println!("  Context created: {:?}", context);
    Ok(())
}
