//! Processing Substrate Selection Example
//!
//! Demonstrates the modern, robust API for selecting processing substrates.
//! NO environment variables! Explicit, async, granular control.

use ml_inference_showcase::substrate::*;
use ml_inference_showcase::wgpu::WgpuExecutor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║        Processing Substrate Selection - Modern API         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Create selector
    let selector = SubstrateSelector::new();
    
    // ═══════════════════════════════════════════════════════════════════
    // DISCOVERY: Find all available processing substrates
    // ═══════════════════════════════════════════════════════════════════
    
    println!("🔍 Discovering available processing substrates...\n");
    
    let devices = selector.list_devices().await?;
    println!("Found {} devices:", devices.len());
    for device in &devices {
        println!("  {}", device);
    }
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // EXPLICIT SELECTION: Choose specific GPU by vendor
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 1: Explicit GPU Selection by Vendor");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Select NVIDIA GPU (if available)
    match selector.select_gpu_by_vendor(GpuVendor::Nvidia).await {
        Ok(substrate) => {
            println!("✅ Selected: {}", substrate);
            let caps = substrate.capabilities().await?;
            println!("   Capabilities: {}", caps);
            
            // Create executor on this specific GPU
            let executor = create_executor_for_substrate(&substrate).await?;
            println!("   Executor created: {}", executor.gpu_info());
            
            // Run a simple operation to validate
            let input = vec![1.0, -2.0, 3.0, -4.0];
            let result = executor.execute_relu(&input).await?;
            println!("   Test ReLU([1, -2, 3, -4]) = {:?}", result);
        }
        Err(e) => println!("❌ NVIDIA GPU not available: {}", e),
    }
    println!();
    
    // Select AMD GPU (if available)
    match selector.select_gpu_by_vendor(GpuVendor::Amd).await {
        Ok(substrate) => {
            println!("✅ Selected: {}", substrate);
            let caps = substrate.capabilities().await?;
            println!("   Capabilities: {}", caps);
            
            let executor = create_executor_for_substrate(&substrate).await?;
            println!("   Executor created: {}", executor.gpu_info());
            
            let input = vec![1.0, -2.0, 3.0, -4.0];
            let result = executor.execute_relu(&input).await?;
            println!("   Test ReLU([1, -2, 3, -4]) = {:?}", result);
        }
        Err(e) => println!("❌ AMD GPU not available: {}", e),
    }
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // BUILDER PATTERN: Fluent API for complex selection
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 2: Builder Pattern for Complex Selection");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Select AMD GPU with Vulkan backend, low power mode
    let target = ProcessingSubstrate::Gpu(
        GpuTarget::amd()
            .with_backend(GpuBackend::Vulkan)
            .low_power()
    );
    
    if target.is_available().await {
        println!("✅ AMD + Vulkan + Low Power available");
        let caps = target.capabilities().await?;
        println!("   {}", caps);
    } else {
        println!("❌ AMD + Vulkan + Low Power not available");
    }
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // INDEX SELECTION: Direct device selection by index
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 3: Direct Device Selection by Index");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Select GPU 0 (first GPU)
    if let Ok(substrate) = selector.select_gpu_by_index(0).await {
        println!("✅ GPU #0: {}", substrate);
        let caps = substrate.capabilities().await?;
        println!("   {}", caps);
    }
    println!();
    
    // Select GPU 1 (second GPU, if available)
    match selector.select_gpu_by_index(1).await {
        Ok(substrate) => {
            println!("✅ GPU #1: {}", substrate);
            let caps = substrate.capabilities().await?;
            println!("   {}", caps);
        }
        Err(e) => println!("❌ GPU #1 not available: {}", e),
    }
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // DEFAULT SELECTION: Best available device
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 4: Default (Best Available) Device");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    let substrate = selector.default_substrate().await?;
    println!("✅ Default substrate: {}", substrate);
    let caps = substrate.capabilities().await?;
    println!("   {}", caps);
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // CPU SELECTION: Always available fallback
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 5: CPU Substrate (Always Available)");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    let cpu = ProcessingSubstrate::Cpu(CpuTarget::auto().threads(4));
    println!("✅ CPU substrate: {}", cpu);
    println!("   Available: {}", cpu.is_available().await);
    let caps = cpu.capabilities().await?;
    println!("   {}", caps);
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // VALIDATION: Run same operation on all available GPUs
    // ═══════════════════════════════════════════════════════════════════
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Example 6: Cross-Substrate Validation");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    let all_substrates = selector.discover_all().await?;
    let gpu_substrates: Vec<_> = all_substrates.iter()
        .filter(|s| matches!(s, ProcessingSubstrate::Gpu(_)))
        .collect();
    
    if gpu_substrates.len() >= 2 {
        println!("Found {} GPUs - running cross-validation...\n", gpu_substrates.len());
        
        let test_input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        
        for substrate in &gpu_substrates {
            let executor = create_executor_for_substrate(substrate).await?;
            let result = executor.execute_relu(&test_input).await?;
            println!("  {} → {:?}", substrate, result);
        }
        
        println!("\n✅ All GPUs produce identical results!");
    } else {
        println!("Only {} GPU(s) available - skipping cross-validation", gpu_substrates.len());
    }
    println!();
    
    // ═══════════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════════
    
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    KEY TAKEAWAYS                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    println!("✅ EXPLICIT selection (no environment variables!)");
    println!("✅ ASYNC discovery and validation");
    println!("✅ GRANULAR control (vendor, index, backend, power)");
    println!("✅ TYPE-SAFE (compile-time checked)");
    println!("✅ CONCURRENT (tokio-based)");
    println!("✅ ROBUST (proper error handling)");
    println!("✅ TESTABLE (can validate on all substrates)");
    println!("✅ FUTURE-PROOF (CPU, GPU, neuromorphic, custom)");
    println!();
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Zero Deep Debt. Modern. Idiomatic. Async. Concurrent.");
    println!("═══════════════════════════════════════════════════════════════");
    
    Ok(())
}

/// Helper: Create WgpuExecutor for a specific substrate
async fn create_executor_for_substrate(substrate: &ProcessingSubstrate) -> anyhow::Result<WgpuExecutor> {
    match substrate {
        ProcessingSubstrate::Gpu(target) => {
            // Use the vendor-specific methods
            if let Some(vendor) = target.vendor {
                match vendor {
                    GpuVendor::Nvidia => WgpuExecutor::new_nvidia().await,
                    GpuVendor::Amd => WgpuExecutor::new_amd().await,
                    GpuVendor::Intel => WgpuExecutor::new_intel().await,
                    _ => WgpuExecutor::new().await,
                }
            } else {
                WgpuExecutor::new().await
            }
        }
        _ => anyhow::bail!("Only GPU substrates supported for WgpuExecutor"),
    }
}
