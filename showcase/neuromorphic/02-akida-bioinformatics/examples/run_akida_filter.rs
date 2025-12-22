//! Run k-mer filtering on Akida boards

use akida_bioinformatics_demo::{akida_filter::AkidaFilter, benchmark::*, FilterConfig};
use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "run_akida_filter")]
#[command(about = "Run k-mer filtering on Akida boards")]
struct Args {
    /// Number of sequences
    #[arg(long, default_value_t = 10_000)]
    sequences: usize,
    
    /// Sequence length
    #[arg(long, default_value_t = 150)]
    length: usize,
    
    /// K-mer size
    #[arg(long, default_value_t = 31)]
    kmer_size: usize,
    
    /// Model path
    #[arg(long, default_value = "data/kmer_filter.akd")]
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    
    println!("Akida K-mer Filtering Demo\n");
    
    // Initialize Akida
    let mut filter = AkidaFilter::new().await?;
    println!("✓ Found {} Akida board(s)\n", filter.board_count());
    
    // Load model
    println!("Loading model from {}...", args.model);
    filter.load_model(&args.model)?;
    println!("✓ Model loaded\n");
    
    // Generate sequences
    println!("Generating {} sequences...", args.sequences);
    let sequences = generate_sample_sequences(args.sequences, args.length);
    println!("✓ Generated\n");
    
    // Configure filter
    let config = FilterConfig {
        kmer_size: args.kmer_size,
        ..Default::default()
    };
    
    // Run filtering
    println!("Filtering k-mers...");
    let stats = filter.filter_kmers(&sequences, &config)?;
    
    println!("\nResults:");
    println!("  Total k-mers: {:,}", stats.total_kmers);
    println!("  Kept: {:,} ({:.1}%)",
        stats.kept_kmers,
        (stats.kept_kmers as f64 / stats.total_kmers as f64) * 100.0
    );
    println!("  Discarded: {:,} ({:.1}%)",
        stats.discarded_kmers,
        (stats.discarded_kmers as f64 / stats.total_kmers as f64) * 100.0
    );
    println!("\nPerformance:");
    println!("  Time: {:.3}s", stats.processing_time_secs);
    println!("  Throughput: {:.0} k-mers/sec", stats.throughput);
    println!("  Power: {:.1}W", stats.power_watts);
    println!("  Efficiency: {:,.0} k-mers/joule", stats.efficiency);
    
    Ok(())
}

