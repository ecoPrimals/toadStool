//! GPU Precision Capability Check
//!
//! Checks which precision features (f64, f16) are available on each GPU.

#[tokio::main]
async fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  GPU PRECISION CAPABILITY CHECK                               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        let features = adapter.features();
        let limits = adapter.limits();
        
        println!("\n══════════════════════════════════════════════════════════════");
        println!("  {}", info.name);
        println!("══════════════════════════════════════════════════════════════");
        println!("Backend: {:?}", info.backend);
        println!("Device type: {:?}", info.device_type);
        println!("Driver: {}", info.driver);
        
        println!("\nPrecision Features:");
        
        // F64 - Critical for scientific computing
        if features.contains(wgpu::Features::SHADER_F64) {
            println!("  ✅ SHADER_F64: Native f64 in shaders SUPPORTED!");
        } else {
            println!("  ❌ SHADER_F64: Not available (need emulation)");
        }
        
        // F16 - Useful for ML inference
        if features.contains(wgpu::Features::SHADER_F16) {
            println!("  ✅ SHADER_F16: Native f16 in shaders");
        } else {
            println!("  ❌ SHADER_F16: Not available");
        }
        
        println!("\nCompute Features:");
        
        // Timestamps
        if features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            println!("  ✅ TIMESTAMP_QUERY: GPU timing available");
        } else {
            println!("  ❌ TIMESTAMP_QUERY: Not available");
        }
        
        // Pipeline statistics
        if features.contains(wgpu::Features::PIPELINE_STATISTICS_QUERY) {
            println!("  ✅ PIPELINE_STATISTICS_QUERY: Pipeline stats available");
        } else {
            println!("  ❌ PIPELINE_STATISTICS_QUERY: Not available");
        }
        
        println!("\nLimits:");
        println!("  Max buffer size: {} MB", limits.max_buffer_size / (1024 * 1024));
        println!("  Max storage buffer binding: {} MB", limits.max_storage_buffer_binding_size / (1024 * 1024));
        println!("  Max compute workgroup size: [{}, {}, {}]", 
            limits.max_compute_workgroup_size_x,
            limits.max_compute_workgroup_size_y,
            limits.max_compute_workgroup_size_z);
        println!("  Max compute invocations per workgroup: {}", 
            limits.max_compute_invocations_per_workgroup);
        
        // Print all available features for reference
        println!("\nAll available features:");
        println!("  {:?}", features);
    }
    
    println!("\n══════════════════════════════════════════════════════════════");
    println!("  FP64 PERFORMANCE EXPECTATIONS");
    println!("══════════════════════════════════════════════════════════════");
    println!("  Consumer GPUs (RTX 3090, RX 6950 XT):");
    println!("    - fp64:fp32 ratio: 1:32 (NVIDIA) to 1:16 (AMD)");
    println!("    - Expect ~3-6% of fp32 performance");
    println!("  ");
    println!("  Workstation/HPC GPUs (Titan V, A100, MI250):");
    println!("    - fp64:fp32 ratio: 1:2");
    println!("    - Expect ~50% of fp32 performance");
}
