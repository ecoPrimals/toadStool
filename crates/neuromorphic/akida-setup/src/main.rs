//! Akida NPU Setup - Rust Binary (No Scripts!)
//!
//! Replaces bash scripts with compiled Rust code.
//! "Jelly string to constrained DNA" - from flexible scripts to portable binary.
//!
//! This binary can be distributed to any Linux system and will:
//! 1. Load kernel module
//! 2. Enable `PCIe` devices
//! 3. Set up permissions
//! 4. Verify operation

use anyhow::{bail, Result};

mod pcie;
mod permissions;
mod verification;

use pcie::{discover_akida_devices, enable_pcie_device, is_module_loaded, load_kernel_module};
use permissions::{
    list_device_nodes, setup_device_permissions, setup_pcie_permissions, setup_udev_rules,
};
use verification::verify_setup;

#[derive(Debug)]
pub struct SetupConfig {
    /// Path to kernel module
    pub module_path: Option<String>,

    /// Whether to set up persistent udev rules
    pub persistent_permissions: bool,

    /// Skip verification steps
    pub skip_verification: bool,
}

impl Default for SetupConfig {
    fn default() -> Self {
        Self {
            module_path: Some(format!(
                "{}/Development/ecoPrimals/akida_dw_edma/akida-pcie.ko",
                std::env::var("HOME").unwrap_or_default()
            )),
            persistent_permissions: true,
            skip_verification: false,
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("akida_setup=info")
        .init();

    tracing::info!("🧠 Akida NPU Setup - Rust Binary");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check if running as root
    if !is_root() {
        bail!(
            "This binary must be run as root. Try: sudo {}",
            std::env::current_exe()?.display()
        );
    }

    let config = SetupConfig::default();

    // Step 1: Discover Akida devices
    tracing::info!("\n📡 Step 1: Discovering Akida devices...");
    let devices = discover_akida_devices()?;

    if devices.is_empty() {
        bail!("No Akida devices found. Check lspci output.");
    }

    tracing::info!("✅ Found {} Akida device(s):", devices.len());
    for device in &devices {
        tracing::info!("   - {}", device.pcie_address);
    }

    // Step 2: Enable PCIe devices
    tracing::info!("\n🔌 Step 2: Enabling PCIe devices...");
    for device in &devices {
        enable_pcie_device(&device.pcie_address)?;
        tracing::info!("✅ Enabled {}", device.pcie_address);
    }

    // Step 3: Load kernel module
    tracing::info!("\n🔧 Step 3: Loading kernel module...");
    if let Some(module_path) = &config.module_path {
        load_kernel_module(module_path)?;
        tracing::info!("✅ Kernel module loaded");
    } else {
        tracing::warn!("⚠️  No module path specified, skipping kernel module load");
    }

    // Step 4: Set up permissions
    tracing::info!("\n📝 Step 4: Setting up permissions...");

    if config.persistent_permissions {
        setup_udev_rules()?;
        tracing::info!("✅ Udev rules installed");
    }

    // Set permissions on device nodes
    if let Err(e) = setup_device_permissions() {
        tracing::warn!("⚠️  Could not set device permissions: {}", e);
        tracing::warn!("   Device nodes may not be created yet");
    }

    // Set permissions on PCIe resources
    for device in &devices {
        setup_pcie_permissions(&device.pcie_address)?;
        tracing::info!("✅ Set permissions for {}", device.pcie_address);
    }

    // Step 5: Verification
    if !config.skip_verification {
        tracing::info!("\n🔍 Step 5: Verifying setup...");
        verify_setup(&devices)?;
        tracing::info!("✅ Verification complete");
    }

    // Summary
    tracing::info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("✅ Akida NPU Setup Complete!");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("\n📊 Status:");
    tracing::info!("   Devices enabled: {}", devices.len());
    tracing::info!(
        "   Kernel module: {}",
        if is_module_loaded()? {
            "loaded"
        } else {
            "not loaded"
        }
    );

    if let Ok(device_nodes) = list_device_nodes() {
        if !device_nodes.is_empty() {
            tracing::info!("   Device nodes:");
            for node in device_nodes {
                tracing::info!("      - {}", node);
            }
        }
    }

    tracing::info!("\n🎯 Next Steps:");
    tracing::info!("   1. Test detection: cargo run --example detect_akida_real");
    tracing::info!("   2. Run validation: cargo run --bin cross_platform_homomorphic");

    Ok(())
}

fn is_root() -> bool {
    // Pure Rust root detection — no unsafe, no libc dependency for this.
    // Parse /proc/self/status to find the Uid line, which contains:
    //   Uid: <real> <effective> <saved> <filesystem>
    // We check the effective UID (index 1).
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(2) // effective UID (0=field name, 1=real, 2=effective)
                        .and_then(|uid| uid.parse::<u32>().ok())
                })
        })
        == Some(0)
}
