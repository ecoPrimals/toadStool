//! Simple Game Launcher - Launch games via ToadStool
//! 
//! This tool makes it easy to launch games through ToadStool's
//! native runtime. Perfect for testing with old CD games!

use std::path::PathBuf;
use std::process::Command;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "game-launcher")]
#[command(about = "Launch games via ToadStool", long_about = None)]
struct Args {
    /// Path to game executable
    #[arg(short, long)]
    game: PathBuf,
    
    /// Game arguments
    #[arg(short, long)]
    args: Vec<String>,
    
    /// CPU cores to allocate (default: 1.0)
    #[arg(short, long, default_value = "1.0")]
    cpu: f32,
    
    /// Memory in MB (default: 512)
    #[arg(short, long, default_value = "512")]
    memory: u32,
    
    /// Working directory
    #[arg(short, long)]
    workdir: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🎮 Simple Game Launcher");
    println!("======================\n");
    
    // Verify game exists
    if !args.game.exists() {
        eprintln!("❌ Error: Game not found at {:?}", args.game);
        eprintln!("\nMake sure to provide the full path to your game executable.");
        std::process::exit(1);
    }
    
    println!("📋 Configuration:");
    println!("  Game: {:?}", args.game);
    println!("  Args: {:?}", args.args);
    println!("  CPU: {} cores", args.cpu);
    println!("  Memory: {} MB", args.memory);
    if let Some(ref wd) = args.workdir {
        println!("  Working Dir: {:?}", wd);
    }
    println!();
    
    // For now, execute directly
    // TODO: Wire up ToadStool API when server is running
    println!("🚀 Launching game...\n");
    
    let mut cmd = Command::new(&args.game);
    
    if let Some(workdir) = args.workdir {
        cmd.current_dir(workdir);
    }
    
    if !args.args.is_empty() {
        cmd.args(&args.args);
    }
    
    let status = cmd.status()?;
    
    println!("\n📊 Game finished!");
    println!("  Exit code: {:?}", status.code());
    
    if status.success() {
        println!("  ✅ Success!");
    } else {
        println!("  ❌ Failed!");
    }
    
    println!("\n💡 Next steps:");
    println!("  - This currently executes directly");
    println!("  - Wire up ToadStool API for full features:");
    println!("    • Job tracking");
    println!("    • Resource monitoring");
    println!("    • Remote execution");
    println!("    • Multiplayer coordination");
    
    Ok(())
}

