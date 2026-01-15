//! List all available GPUs for research
//!
//! Utility to enumerate all GPUs in the system for multi-GPU experiments

use ml_inference_showcase::wgpu::WgpuExecutor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Enumerating Available GPUs");
    println!("==============================\n");
    
    let gpus = WgpuExecutor::list_gpus().await;
    
    println!("Found {} GPU(s):\n", gpus.len());
    
    for (i, gpu) in gpus.iter().enumerate() {
        println!("  [{}] {}", i, gpu);
    }
    
    println!("\n✅ GPU enumeration complete!");
    
    Ok(())
}
