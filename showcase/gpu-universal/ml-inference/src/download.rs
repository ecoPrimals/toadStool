//! Download MNIST dataset

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

fn download_file(url: &str, path: &Path) -> Result<()> {
    if path.exists() {
        println!("✓ {} already exists", path.display());
        return Ok(());
    }

    println!("Downloading {}...", url);
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;

    let mut file = fs::File::create(path)?;
    file.write_all(&bytes)?;

    println!("✓ Saved to {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let data_dir = Path::new("data/mnist");
    fs::create_dir_all(data_dir)?;

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Downloading MNIST Dataset                               ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Use GitHub mirror (Yann LeCun's site is unreliable)
    let base_url = "https://github.com/cvdfoundation/mnist/raw/main";
    let files = [
        (
            "train-images-idx3-ubyte.gz",
            format!("{}/train-images-idx3-ubyte.gz", base_url),
        ),
        (
            "train-labels-idx1-ubyte.gz",
            format!("{}/train-labels-idx1-ubyte.gz", base_url),
        ),
        (
            "t10k-images-idx3-ubyte.gz",
            format!("{}/t10k-images-idx3-ubyte.gz", base_url),
        ),
        (
            "t10k-labels-idx1-ubyte.gz",
            format!("{}/t10k-labels-idx1-ubyte.gz", base_url),
        ),
    ];

    for (filename, url) in files {
        let path = data_dir.join(&filename);
        download_file(&url, &path)?;
    }

    println!();
    println!("✅ MNIST dataset downloaded successfully!");
    println!();
    println!("Next steps:");
    println!("  1. cargo run --release --bin mnist-cpu-baseline");
    println!("  2. cargo run --release --bin mnist-gpu-cuda --features cuda");
    println!("  3. cargo run --release --bin validate-correctness");

    Ok(())
}
