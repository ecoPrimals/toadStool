// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compare CPU vs Akida k-mer filtering performance

use akida_bioinformatics_demo::{
    akida_filter::AkidaFilter, benchmark::*, cpu_filter::filter_kmers_cpu, FilterConfig,
};
use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "compare_cpu_akida")]
#[command(about = "Compare CPU vs Akida k-mer filtering performance")]
struct Args {
    /// Number of sequences to process
    #[arg(long, default_value_t = 100_000)]
    sequences: usize,
    
    /// Length of each sequence
    #[arg(long, default_value_t = 150)]
    length: usize,
    
    /// K-mer size
    #[arg(long, default_value_t = 31)]
    kmer_size: usize,
    
    /// Number of iterations for averaging
    #[arg(long, default_value_t = 3)]
    iterations: usize,
    
    /// Output file for results
    #[arg(long, default_value = "results/comparison.json")]
    output: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     CPU vs Akida K-mer Filtering Benchmark                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    // Generate sample sequences
    println!("Generating {} sequences of length {}...", args.sequences, args.length);
    let sequences = generate_sample_sequences(args.sequences, args.length);
    println!("✓ Generated {:,} total bases\n", args.sequences * args.length);
    
    let config = FilterConfig {
        kmer_size: args.kmer_size,
        ..Default::default()
    };
    
    println!("Configuration:");
    println!("  K-mer size: {}", config.kmer_size);
    println!("  GC content: {:.0}% - {:.0}%",
        config.min_gc_content * 100.0,
        config.max_gc_content * 100.0
    );
    println!("  Filter low complexity: {}", config.filter_low_complexity);
    println!("  Filter adapters: {}\n", config.filter_adapters);
    
    // Benchmark CPU
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" CPU Baseline");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    println!("Running CPU filtering ({} iterations)...", args.iterations);
    let mut cpu_stats = vec![];
    
    for i in 0..args.iterations {
        print!("  Iteration {}/{}... ", i + 1, args.iterations);
        let stats = filter_kmers_cpu(&sequences, &config)?;
        println!("✓ {:.2}s", stats.processing_time_secs);
        cpu_stats.push(stats);
    }
    
    let cpu_avg = average_stats(&cpu_stats);
    println!("\nCPU Average:");
    println!("  Processing time: {:.2}s", cpu_avg.processing_time_secs);
    println!("  Throughput: {:,.0} k-mers/sec", cpu_avg.throughput);
    println!("  Power: {:.1}W", cpu_avg.power_watts);
    println!("  Efficiency: {:,.0} k-mers/joule", cpu_avg.efficiency);
    
    // Benchmark Akida
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Akida Accelerated");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    match AkidaFilter::new().await {
        Ok(mut filter) => {
            println!("Detected {} Akida board(s)", filter.board_count());
            
            println!("Loading k-mer filter model...");
            filter.load_model("data/kmer_filter.akd")?;
            println!("✓ Model loaded\n");
            
            println!("Running Akida filtering ({} iterations)...", args.iterations);
            let mut akida_stats = vec![];
            
            for i in 0..args.iterations {
                print!("  Iteration {}/{}... ", i + 1, args.iterations);
                let stats = filter.filter_kmers(&sequences, &config)?;
                println!("✓ {:.2}s", stats.processing_time_secs);
                akida_stats.push(stats);
            }
            
            let akida_avg = average_stats(&akida_stats);
            println!("\nAkida Average:");
            println!("  Processing time: {:.2}s", akida_avg.processing_time_secs);
            println!("  Throughput: {:,.0} k-mers/sec", akida_avg.throughput);
            println!("  Power: {:.1}W", akida_avg.power_watts);
            println!("  Efficiency: {:,.0} k-mers/joule", akida_avg.efficiency);
            
            // Generate comparison
            let comparison = ComparisonResults::from_stats(cpu_avg, akida_avg);
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!(" Comparison");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            
            println!("Speedup: {:.1}x faster", comparison.speedup);
            println!("Power reduction: {:.1}x less power", comparison.power_reduction);
            println!("Efficiency gain: {:.0}x more efficient", comparison.efficiency_gain);
            
            // Save results
            std::fs::create_dir_all("results")?;
            comparison.save_json(&args.output)?;
            println!("\n✓ Results saved to {}", args.output);
        }
        Err(e) => {
            println!("⚠ No Akida boards detected: {}", e);
            println!("\nNote: This benchmark requires Akida PCIe boards.");
            println!("Expected deployment: 2x on Strandgate, 1x on Southgate");
        }
    }
    
    Ok(())
}

/// Average multiple filter stats
fn average_stats(stats: &[akida_bioinformatics_demo::FilterStats]) -> akida_bioinformatics_demo::FilterStats {
    let count = stats.len() as f64;
    
    let total_kmers = stats[0].total_kmers;
    let kept_kmers = stats[0].kept_kmers;
    
    let avg_time = stats.iter().map(|s| s.processing_time_secs).sum::<f64>() / count;
    let avg_power = stats.iter().map(|s| s.power_watts).sum::<f64>() / count;
    
    akida_bioinformatics_demo::FilterStats::new(total_kmers, kept_kmers, avg_time, avg_power)
}

