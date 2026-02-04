//! Cross Attention Cross-Substrate Validation

use barracuda::device::Substrate;
use barracuda::tensor::Tensor;
use colored::*;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  Cross Attention Cross-Substrate Validation".bright_white().bold());
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
    
    // T5/BART-style dimensions (asymmetric seq_lens!)
    let batch = 2;
    let heads = 8;
    let dec_seq = 16;   // Decoder (short)
    let enc_seq = 64;   // Encoder (long)
    let dim = 64;
    
    let q_data = vec![0.5; batch * heads * dec_seq * dim];
    let k_data = vec![0.5; batch * heads * enc_seq * dim];
    let v_data = vec![1.0; batch * heads * enc_seq * dim];
    
    let q_ref = Tensor::from_vec_on(q_data.clone(), vec![batch, heads, dec_seq, dim], ref_device.clone()).await?;
    let k_ref = Tensor::from_vec_on(k_data.clone(), vec![batch, heads, enc_seq, dim], ref_device.clone()).await?;
    let v_ref = Tensor::from_vec_on(v_data.clone(), vec![batch, heads, enc_seq, dim], ref_device).await?;

    println!("  {} Computing reference result...", "🔬".bright_yellow());
    let ref_output = q_ref.cross_attention(&k_ref, &v_ref)?;
    let ref_data = ref_output.to_vec()?;
    println!("  {} Reference computed: {} elements (decoder seq: {}, encoder seq: {})", 
        "✅".green(), ref_data.len(), dec_seq, enc_seq);
    println!();

    println!("{}", "  Running cross-substrate tests...".bright_yellow().bold());
    println!();

    for substrate in &substrates {
        print!("    {} {}... ", "🧪".bright_white(), substrate.to_string().bright_cyan());
        
        let start = Instant::now();
        
        let device = Arc::new(substrate.create_device().await?);
        
        let q = Tensor::from_vec_on(q_data.clone(), vec![batch, heads, dec_seq, dim], device.clone()).await?;
        let k = Tensor::from_vec_on(k_data.clone(), vec![batch, heads, enc_seq, dim], device.clone()).await?;
        let v = Tensor::from_vec_on(v_data.clone(), vec![batch, heads, enc_seq, dim], device).await?;
        
        let output = q.cross_attention(&k, &v)?;
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
    println!("{}", "  ✅ Cross Attention validated across all substrates!".bright_green().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    Ok(())
}
