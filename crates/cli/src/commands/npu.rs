// NPU management commands

use akida_driver::setup::NpuSetup;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "npu")]
#[command(about = "Manage Akida NPU hardware")]
pub enum NpuCommand {
    /// Setup NPU kernel driver and devices
    Setup(SetupCommand),

    /// Show NPU status
    Status,

    /// List available NPUs
    List,
}

#[derive(Parser)]
pub struct SetupCommand {
    /// Skip confirmation prompts
    #[arg(short, long)]
    yes: bool,
}

impl NpuCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::Setup(cmd) => cmd.run(),
            Self::Status => show_status(),
            Self::List => list_devices(),
        }
    }
}

impl SetupCommand {
    pub fn run(self) -> Result<()> {
        if !self.yes {
            println!("This will:");
            println!("  - Enable PCIe devices");
            println!("  - Load kernel module (requires pkexec)");
            println!("  - Create /dev/akida* device nodes");
            println!();
            print!("Continue? [y/N] ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }

        // Run pure Rust setup (no scripts!)
        let mut setup = NpuSetup::new();
        setup.run()?;

        println!();
        println!("✅ NPU setup complete!");
        println!();
        println!("Next steps:");
        println!("  toadstool npu list    # List available NPUs");
        println!("  toadstool npu status  # Show detailed status");

        Ok(())
    }
}

fn show_status() -> Result<()> {
    use akida_driver::DeviceManager;

    println!("🧠 Akida NPU Status\n");

    // Try to discover devices
    match DeviceManager::discover() {
        Ok(manager) => {
            println!("✅ {} device(s) operational\n", manager.device_count());

            for device in manager.devices() {
                let caps = device.capabilities();
                println!("Device {}: {}", device.index(), device.pcie_address());
                println!("  Chip: {:?}", caps.chip_version);
                println!("  NPUs: {}", caps.npu_count);
                println!("  SRAM: {} MB", caps.memory_mb);
                println!("  PCIe: Gen{} x{}", caps.pcie.generation, caps.pcie.lanes);
                println!();
            }
        }
        Err(e) => {
            println!("❌ No NPU devices accessible: {}", e);
            println!();
            println!("Try running: toadstool npu setup");
        }
    }

    Ok(())
}

fn list_devices() -> Result<()> {
    use akida_driver::DeviceManager;

    match DeviceManager::discover() {
        Ok(manager) => {
            println!("Available NPUs:");
            for device in manager.devices() {
                println!(
                    "  {} - {} @ {}",
                    device.index(),
                    device.capabilities().chip_version.to_string(),
                    device.pcie_address()
                );
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
