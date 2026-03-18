// SPDX-License-Identifier: AGPL-3.0-or-later
//! Basic I/O test with Akida device
//!
//! Demonstrates opening a device and performing simple read/write operations.

use akida_driver::{DeviceManager, Result};

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("akida_driver=info")
        .init();

    println!("🧠 Akida Basic I/O Test\n");

    // Discover and open first device
    let manager = DeviceManager::discover()?;
    println!("Found {} device(s)", manager.device_count());

    let mut device = manager.open_first()?;
    println!("✅ Opened: {}\n", device.path().display());

    // Test pattern write (i % 256 always in 0..256, fits u8)
    let test_data: Vec<u8> = (0..1024)
        .map(|i| u8::try_from(i % 256).expect("0..256 fits u8"))
        .collect();
    println!("📤 Writing {} bytes...", test_data.len());

    let written = device.write(&test_data)?;
    println!("✅ Wrote {written} bytes");

    // Test read
    let mut buffer = vec![0u8; 1024];
    println!("\n📥 Reading {} bytes...", buffer.len());

    let read_bytes = device.read(&mut buffer)?;
    println!("✅ Read {read_bytes} bytes");

    // Verify data (if device echoes back)
    if buffer[..read_bytes] == test_data[..read_bytes] {
        println!("\n🎉 Data verification: PASSED");
    } else {
        println!("\nℹ️  Data differs (expected for non-echo device)");
    }

    println!("\n✅ I/O test complete");

    Ok(())
}
