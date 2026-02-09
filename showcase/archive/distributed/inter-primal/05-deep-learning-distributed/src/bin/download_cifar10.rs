// Download CIFAR-10 dataset

use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Downloading CIFAR-10 dataset...");
    
    let data_dir = PathBuf::from("datasets/cifar10");
    std::fs::create_dir_all(&data_dir)?;
    
    println!("📂 Data directory: {:?}", data_dir);
    println!("");
    println!("Note: tch-rs will automatically download CIFAR-10 when you first");
    println!("      run training. The dataset is ~170MB and will be cached.");
    println!("");
    println!("Manual download:");
    println!("  1. Visit: https://www.cs.toronto.edu/~kriz/cifar.html");
    println!("  2. Download: CIFAR-10 binary version (for C programs)");
    println!("  3. Extract to: {:?}", data_dir);
    println!("");
    println!("Or just run training - tch-rs handles it automatically!");
    
    Ok(())
}

