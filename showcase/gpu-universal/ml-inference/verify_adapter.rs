// SPDX-License-Identifier: AGPL-3.0-or-later
use ml_inference_showcase::WgpuExecutor;

#[tokio::main]
async fn main() {
    println!("🔍 Detecting GPU adapter...\n");
    
    let executor = WgpuExecutor::new().await.unwrap();
    
    println!("Adapter successfully initialized!");
    println!("Running simple test to confirm GPU is working...");
    
    let input = vec![1.0, 2.0, 3.0, 4.0];
    let result = executor.execute_relu(&input).await.unwrap();
    
    println!("✅ Test passed! Result: {:?}", result);
}
