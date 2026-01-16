use ml_inference_showcase::WgpuExecutor;

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           GPU Adapter Detection Test                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Test default adapter
    println!("🔍 Testing DEFAULT adapter...");
    let default_exec = WgpuExecutor::new().await.unwrap();
    let info = default_exec.get_gpu_info();
    println!("   Name: {}", info.name);
    println!("   Vendor: 0x{:04X}", info.vendor);
    println!("   Backend: {}", info.backend);
    println!("   Type: {}\n", info.device_type);
    
    // Test AMD adapter
    println!("🔴 Testing AMD adapter...");
    match WgpuExecutor::new_amd().await {
        Ok(amd_exec) => {
            let info = amd_exec.get_gpu_info();
            println!("   ✅ AMD GPU found!");
            println!("   Name: {}", info.name);
            println!("   Vendor: 0x{:04X}", info.vendor);
            println!("   Backend: {}", info.backend);
            println!("   Type: {}\n", info.device_type);
        }
        Err(e) => println!("   ❌ No AMD GPU found: {}\n", e),
    }
    
    // Test NVIDIA adapter
    println!("🟢 Testing NVIDIA adapter...");
    match WgpuExecutor::new_nvidia().await {
        Ok(nvidia_exec) => {
            let info = nvidia_exec.get_gpu_info();
            println!("   ✅ NVIDIA GPU found!");
            println!("   Name: {}", info.name);
            println!("   Vendor: 0x{:04X}", info.vendor);
            println!("   Backend: {}", info.backend);
            println!("   Type: {}\n", info.device_type);
        }
        Err(e) => println!("   ❌ No NVIDIA GPU found: {}\n", e),
    }
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("Vendor IDs: 0x10DE = NVIDIA, 0x1002 = AMD");
    println!("═══════════════════════════════════════════════════════════════");
}
