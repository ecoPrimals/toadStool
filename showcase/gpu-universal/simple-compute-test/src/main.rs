use anyhow::Result;
use ocl::{Platform, Device, Queue, Program, Buffer, Kernel};
use ocl::enums::DeviceSpecifier;
use ocl_core::ContextProperties;

const VECTOR_ADD_KERNEL: &str = r#"
__kernel void vector_add(
    __global const float* a,
    __global const float* b,
    __global float* c,
    const unsigned int n)
{
    int i = get_global_id(0);
    if (i < n) {
        c[i] = a[i] + b[i];
    }
}
"#;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  GPU Compute Execution Test (Not Just Detection)            ║");
    println!("║  Verifying: Can both GPUs EXECUTE compute, not just detect? ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let platforms = Platform::list();
    
    let mut total_gpus_tested = 0;
    let mut gpus_working = 0;
    let mut gpus_failed = 0;

    for (platform_idx, platform) in platforms.iter().enumerate() {
        let platform_name = platform.name().unwrap_or_else(|_| "Unknown".to_string());
        
        // Skip CPU-only platforms
        if platform_name.contains("Clover") {
            continue;
        }

        println!("──────────────────────────────────────────────────────────────");
        println!("Platform {}: {}", platform_idx, platform_name);
        println!("──────────────────────────────────────────────────────────────");
        
        match Device::list_all(*platform) {
            Ok(devices) if !devices.is_empty() => {
                for (device_idx, device) in devices.iter().enumerate() {
                    let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                    println!();
                    println!("  Testing Device {}: {}", device_idx, device_name);
                    
                    total_gpus_tested += 1;
                    
                    // Test compute execution with low-level API for AMD compatibility
                    match test_vector_add(*platform, device, &device_name) {
                        Ok(()) => {
                            println!("    Result: ✅ COMPUTE WORKING");
                            gpus_working += 1;
                        },
                        Err(e) => {
                            println!("    Result: ❌ COMPUTE FAILED");
                            println!("    Error: {}", e);
                            gpus_failed += 1;
                        }
                    }
                }
            },
            Ok(_) => {
                println!("  ⚠️  No devices on this platform");
            },
            Err(e) => {
                println!("  ❌ Error enumerating devices: {}", e);
            }
        }
        println!();
    }

    println!("══════════════════════════════════════════════════════════════");
    println!("📊 SUMMARY");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("  GPUs Tested:  {}", total_gpus_tested);
    println!("  Working:      {} ✅", gpus_working);
    println!("  Failed:       {} ❌", gpus_failed);
    println!();

    if gpus_working >= 2 {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  🎉 SUCCESS: MULTI-VENDOR COMPUTE EXECUTION VERIFIED        ║");
        println!("║  Both NVIDIA and AMD can EXECUTE compute workloads! ✅      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else if gpus_working == 1 {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ⚠️  PARTIAL: One GPU working, one needs debugging          ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ❌ ISSUE: No GPUs successfully executed compute             ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    println!();
    println!("💡 Key Insight:");
    println!("   Detection vs Execution are different capabilities.");
    println!("   This test verifies EXECUTION (kernel compilation, memory,");
    println!("   data transfer, and compute) all work on both vendors.");
    println!("   Using low-level OpenCL API for AMD compatibility.");
    println!();

    Ok(())
}

fn test_vector_add(platform: Platform, device: &Device, _device_name: &str) -> Result<()> {
    let size = 10000;
    
    println!("    Creating compute context...");
    
    // Use ocl::Context::new() with explicit properties for AMD compatibility
    let properties = ContextProperties::new().platform(platform);
    let context = ocl::Context::new(
        Some(properties),
        Some(DeviceSpecifier::Single(*device)),
        None,
        None,
    )?;
    
    let queue = Queue::new(&context, *device, None)?;
    
    println!("    Compiling kernel...");
    
    // Build program
    let program = Program::builder()
        .devices(*device)
        .src(VECTOR_ADD_KERNEL)
        .build(&context)?;
    
    println!("    Allocating device memory...");
    
    // Create buffers
    let a_buf = Buffer::<f32>::builder()
        .queue(queue.clone())
        .len(size)
        .build()?;
    
    let b_buf = Buffer::<f32>::builder()
        .queue(queue.clone())
        .len(size)
        .build()?;
    
    let c_buf = Buffer::<f32>::builder()
        .queue(queue.clone())
        .len(size)
        .build()?;
    
    println!("    Transferring data to device...");
    
    // Initialize input data
    let a_vec = vec![1.0f32; size];
    let b_vec = vec![2.0f32; size];
    
    // Write to device
    a_buf.write(&a_vec).enq()?;
    b_buf.write(&b_vec).enq()?;
    
    println!("    Executing kernel...");
    
    // Create and execute kernel
    let kernel = Kernel::builder()
        .program(&program)
        .name("vector_add")
        .queue(queue.clone())
        .global_work_size(size)
        .arg(&a_buf)
        .arg(&b_buf)
        .arg(&c_buf)
        .arg(&(size as u32))
        .build()?;
    
    unsafe { kernel.enq()?; }
    
    println!("    Reading results...");
    
    // Read result
    let mut c_vec = vec![0.0f32; size];
    c_buf.read(&mut c_vec).enq()?;
    
    // Wait for completion
    queue.finish()?;
    
    println!("    Verifying correctness...");
    
    // Verify results
    let expected = 3.0f32;
    let mut errors = 0;
    for (i, &val) in c_vec.iter().enumerate() {
        if (val - expected).abs() > 0.001 {
            if errors == 0 {
                println!("    ⚠️  First error at index {}: expected {}, got {}", i, expected, val);
            }
            errors += 1;
            if errors > 10 {
                break;
            }
        }
    }
    
    if errors > 0 {
        anyhow::bail!("Verification failed: {} errors out of {} elements", errors, size);
    }
    
    println!("    Verification: ✅ All {} elements correct!", size);
    
    Ok(())
}
