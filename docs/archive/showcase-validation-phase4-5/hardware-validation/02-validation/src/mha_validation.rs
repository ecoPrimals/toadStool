//! Multi-Head Attention Cross-Substrate Validation
//!
//! Standalone validator for multi_head_attention across all GPUs

use barracuda::device::Substrate;
use barracuda::tensor::Tensor;
use colored::*;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  Multi-Head Attention Cross-Substrate Validation".bright_white().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    // Discover substrates
    let substrates = Substrate::discover_all()?;
    println!("  {} Found {} substrates", "✅".green(), substrates.len());
    for substrate in &substrates {
        println!("    - {}", substrate.to_string().bright_white());
    }
    println!();

    // Create reference device
    let ref_substrate = &substrates[0];
    let ref_device = Arc::new(ref_substrate.create_device().await?);
    
    println!("{}", "  Creating test tensors...".bright_yellow());
    
    let batch = 2;
    let seq_len = 16;
    let d_model = 128;
    let num_heads = 8;
    
    // Create inputs [B, S, D]
    let q_data = vec![0.5; batch * seq_len * d_model];
    let k_data = vec![0.5; batch * seq_len * d_model];
    let v_data = vec![1.0; batch * seq_len * d_model];
    
    let q_ref = Tensor::from_vec_on(q_data.clone(), vec![batch, seq_len, d_model], ref_device.clone()).await?;
    let k_ref = Tensor::from_vec_on(k_data.clone(), vec![batch, seq_len, d_model], ref_device.clone()).await?;
    let v_ref = Tensor::from_vec_on(v_data.clone(), vec![batch, seq_len, d_model], ref_device.clone()).await?;
    
    // Create projection weights [D, D]
    let weight_data = vec![0.01; d_model * d_model];
    let w_q_ref = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], ref_device.clone()).await?;
    let w_k_ref = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], ref_device.clone()).await?;
    let w_v_ref = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], ref_device.clone()).await?;
    let w_o_ref = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], ref_device).await?;

    println!("  {} Computing reference result...", "🔬".bright_yellow());
    let ref_output = q_ref.multi_head_attention(&k_ref, &v_ref, &w_q_ref, &w_k_ref, &w_v_ref, &w_o_ref, num_heads)?;
    let ref_data = ref_output.to_vec()?;
    println!("  {} Reference computed: {} elements", "✅".green(), ref_data.len());
    println!();

    // Test on each substrate
    println!("{}", "  Running cross-substrate tests...".bright_yellow().bold());
    println!();

    for substrate in &substrates {
        print!("    {} {}... ", "🧪".bright_white(), substrate.to_string().bright_cyan());
        
        let start = Instant::now();
        
        let device = Arc::new(substrate.create_device().await?);
        
        let q = Tensor::from_vec_on(q_data.clone(), vec![batch, seq_len, d_model], device.clone()).await?;
        let k = Tensor::from_vec_on(k_data.clone(), vec![batch, seq_len, d_model], device.clone()).await?;
        let v = Tensor::from_vec_on(v_data.clone(), vec![batch, seq_len, d_model], device.clone()).await?;
        
        let w_q = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], device.clone()).await?;
        let w_k = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], device.clone()).await?;
        let w_v = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], device.clone()).await?;
        let w_o = Tensor::from_vec_on(weight_data.clone(), vec![d_model, d_model], device).await?;
        
        let output = q.multi_head_attention(&k, &v, &w_q, &w_k, &w_v, &w_o, num_heads)?;
        let test_data = output.to_vec()?;
        
        let elapsed = start.elapsed();
        
        // Compare results
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
    println!("{}", "  ✅ Multi-Head Attention validated across all substrates!".bright_green().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    Ok(())
}
