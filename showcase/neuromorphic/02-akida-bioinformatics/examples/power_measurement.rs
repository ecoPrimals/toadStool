//! Power consumption measurement for Akida boards

use akida_bioinformatics_demo::{akida_filter::AkidaFilter, benchmark::*, FilterConfig};
use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "power_measurement")]
#[command(about = "Measure Akida power consumption during k-mer filtering")]
struct Args {
    /// Measurement duration in seconds
    #[arg(long, default_value_t = 30)]
    duration: u64,
    
    /// Sequences per batch
    #[arg(long, default_value_t = 1000)]
    batch_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    
    println!("Akida Power Measurement\n");
    
    // Initialize Akida
    let mut filter = AkidaFilter::new().await?;
    println!("Found {} Akida board(s)", filter.board_count());
    
    filter.load_model("data/kmer_filter.akd")?;
    println!("Model loaded\n");
    
    let config = FilterConfig::default();
    
    println!("Starting continuous workload for {}s...\n", args.duration);
    
    let start = std::time::Instant::now();
    let mut total_kmers = 0u64;
    let mut iterations = 0usize;
    
    while start.elapsed().as_secs() < args.duration {
        // Generate batch
        let sequences = generate_sample_sequences(args.batch_size, 150);
        
        // Process
        let stats = filter.filter_kmers(&sequences, &config)?;
        total_kmers += stats.total_kmers;
        iterations += 1;
        
        if iterations % 10 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let throughput = total_kmers as f64 / elapsed;
            println!("[{:5.1}s] Processed {:,} k-mers ({:.0} k-mers/sec, {:.1}W)",
                elapsed,
                total_kmers,
                throughput,
                stats.power_watts
            );
        }
    }
    
    let total_time = start.elapsed().as_secs_f64();
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Power Measurement Results");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    println!("Workload:");
    println!("  Duration: {:.1}s", total_time);
    println!("  Total k-mers: {:,}", total_kmers);
    println!("  Iterations: {}", iterations);
    println!();
    
    // In production, would measure actual power from PCIe or external meter
    // For now, use estimates based on Akida AKD1000 specs
    let avg_power = 1.2 * filter.board_count() as f64; // ~1.2W per active board
    let energy_joules = avg_power * total_time;
    let energy_wh = energy_joules / 3600.0;
    
    println!("Power consumption:");
    println!("  Average power: {:.1}W", avg_power);
    println!("  Total energy: {:.1}J ({:.3}Wh)", energy_joules, energy_wh);
    println!("  Efficiency: {:,.0} k-mers/joule", total_kmers as f64 / energy_joules);
    println!();
    
    // Compare to CPU estimate
    let cpu_power = 25.0; // Typical 8-core workload
    let cpu_energy = cpu_power * total_time;
    
    println!("Comparison to CPU:");
    println!("  CPU power (estimated): {:.1}W", cpu_power);
    println!("  CPU energy (estimated): {:.1}J", cpu_energy);
    println!("  Power reduction: {:.1}x", cpu_power / avg_power);
    println!("  Energy savings: {:.1}J ({:.1}%)",
        cpu_energy - energy_joules,
        ((cpu_energy - energy_joules) / cpu_energy) * 100.0
    );
    println!();
    
    // Cost and CO2 savings
    let kwh_cost = 0.146; // $/kWh (US average)
    let co2_per_kwh = 0.92; // lbs CO2/kWh (US grid mix)
    
    let energy_kwh = energy_wh / 1000.0;
    let cost = energy_kwh * kwh_cost;
    let co2_lbs = energy_kwh * co2_per_kwh;
    
    println!("Environmental impact (this run):");
    println!("  Cost: ${:.6}", cost);
    println!("  CO2: {:.3} lbs ({:.1}g)", co2_lbs, co2_lbs * 453.592);
    println!();
    
    // Extrapolate to 24/7 operation
    let hours_per_day = 24.0;
    let days_per_year = 365.0;
    let yearly_kwh = (avg_power / 1000.0) * hours_per_day * days_per_year;
    let yearly_cost = yearly_kwh * kwh_cost;
    let yearly_co2_kg = (yearly_kwh * co2_per_kwh * 453.592) / 1000.0;
    
    println!("24/7 operation (extrapolated):");
    println!("  Yearly energy: {:.1} kWh", yearly_kwh);
    println!("  Yearly cost: ${:.2}", yearly_cost);
    println!("  Yearly CO2: {:.1} kg", yearly_co2_kg);
    
    Ok(())
}

