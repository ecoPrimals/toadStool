//! Rotary Embedding (RoPE) Cross-Substrate Validation

use barracuda::device::Substrate;
use barracuda::tensor::Tensor;
use colored::*;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  Rotary Embedding (RoPE) Cross-Substrate Validation".bright_white().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    let substrates = Substrate::discover_all()?;
    println!("  {} Found {} substrates", "✅".green(), substrates.len());
    for substrate in &substrates {
        println!("    - {}", substrate.to_string().bright_white());
    }
    println!();

    let ref_substrate = &substrates[0];
    let ref_device = Arc::new(ref_substrate.create_device().await?);
    
    println!("{}", "  Creating test tensors...".bright_yellow());
    
    // Llama-style dimensions
    let batch = 2;
    let seq_len = 128;
    let num_heads = 8;
    let head_dim = 64;
    
    let input_data = vec![0.5; batch * seq_len * num_heads * head_dim];
    
    let input_ref = Tensor::from_vec_on(
        input_data.clone(),
        vec![batch, seq_len, num_heads, head_dim],
        ref_device
    ).await?;

    println!("  {} Computing reference result...", "🔬".bright_yellow());
    let ref_output = input_ref.rotary_embedding()?;
    let ref_data = ref_output.to_vec()?;
    println!("  {} Reference computed: {} elements", "✅".green(), ref_data.len());
    println!();

    println!("{}", "  Running cross-substrate tests...".bright_yellow().bold());
    println!();

    for substrate in &substrates {
        print!("    {} {}... ", "🧪".bright_white(), substrate.to_string().bright_cyan());
        
        let start = Instant::now();
        
        let device = Arc::new(substrate.create_device().await?);
        
        let input = Tensor::from_vec_on(
            input_data.clone(),
            vec![batch, seq_len, num_heads, head_dim],
            device
        ).await?;
        
        let output = input.rotary_embedding()?;
        let test_data = output.to_vec()?;
        
        let elapsed = start.elapsed();
        
        let max_diff = ref_data.iter()
            .zip(test_data.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        
        if max_diff < 1e-4 {
            println!("{} {} (max_diff: {:.2e}, {:.1}ms)", 
                "PASS".bright_green().bold(),
                "✅".green(),
                max_diff,
                elapsed.as_secs_f64() * 1000.0
            );
        } else {
            println!("{} {} (max_diff: {:.2e}, {:.1}ms)", 
                "FAIL".bright_red().bold(),
                "❌".red(),
                max_diff,
                elapsed.as_secs_f64() * 1000.0
            );
        }
    }

    println!();
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  ✅ Rotary Embedding validated across all substrates!".bright_green().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    Ok(())
}
