// SPDX-License-Identifier: AGPL-3.0-or-later
//! Akida-accelerated k-mer filtering

use anyhow::{Context, Result};
use akida_detection_demo::{detect_all_boards, AkidaMesh};

use crate::kmer::{extract_kmers, to_one_hot};
use crate::{FilterConfig, FilterStats};

/// Akida-based k-mer filtering
pub struct AkidaFilter {
    /// Detected Akida boards
    mesh: AkidaMesh,
    
    /// Model loaded (mock for now)
    model_loaded: bool,
}

impl AkidaFilter {
    /// Create new Akida filter
    pub async fn new() -> Result<Self> {
        let mesh = detect_all_boards().await?;
        
        if mesh.boards.is_empty() {
            anyhow::bail!("No Akida boards detected");
        }
        
        Ok(Self {
            mesh,
            model_loaded: false,
        })
    }
    
    /// Load SNN model for k-mer filtering
    pub fn load_model(&mut self, _model_path: &str) -> Result<()> {
        // In production, this would:
        // 1. Load trained SNN model from disk
        // 2. Transfer to Akida board memory
        // 3. Configure NPUs for inference
        
        tracing::info!("Loading k-mer filter model to Akida board(s)");
        
        // Mock: assume model loaded successfully
        self.model_loaded = true;
        
        Ok(())
    }
    
    /// Filter k-mers using Akida
    pub fn filter_kmers(
        &self,
        sequences: &[Vec<u8>],
        config: &FilterConfig,
    ) -> Result<FilterStats> {
        if !self.model_loaded {
            anyhow::bail!("Model not loaded. Call load_model() first.");
        }
        
        let start = std::time::Instant::now();
        
        // Extract all k-mers
        let all_kmers: Vec<Vec<u8>> = sequences
            .iter()
            .flat_map(|seq| extract_kmers(seq, config.kmer_size))
            .collect();
        
        let total_kmers = all_kmers.len() as u64;
        
        // Distribute k-mers across available boards
        let kept_kmers = self.filter_on_boards(&all_kmers)?;
        
        let elapsed = start.elapsed().as_secs_f64();
        
        // Measure actual power consumption
        let power_watts = self.measure_power();
        
        Ok(FilterStats::new(
            total_kmers,
            kept_kmers as u64,
            elapsed,
            power_watts,
        ))
    }
    
    /// Filter k-mers distributed across Akida boards
    fn filter_on_boards(&self, kmers: &[Vec<u8>]) -> Result<usize> {
        let num_boards = self.mesh.boards.len();
        
        // Split k-mers across boards
        let chunk_size = (kmers.len() + num_boards - 1) / num_boards;
        
        let mut kept_count = 0;
        
        for (board_idx, kmer_chunk) in kmers.chunks(chunk_size).enumerate() {
            if board_idx >= num_boards {
                break;
            }
            
            // Process chunk on this board
            let kept = self.process_on_board(board_idx, kmer_chunk)?;
            kept_count += kept;
        }
        
        Ok(kept_count)
    }
    
    /// Process k-mers on a specific board
    fn process_on_board(&self, board_idx: usize, kmers: &[Vec<u8>]) -> Result<usize> {
        // In production, this would:
        // 1. Convert k-mers to spike trains
        // 2. DMA transfer to board memory
        // 3. Run SNN inference
        // 4. Read results via PCIe
        
        // For now, use mock inference that mimics Akida behavior
        let mut kept_count = 0;
        
        for kmer in kmers {
            // Convert to one-hot encoding (would be spike train in production)
            let _encoding = to_one_hot(kmer);
            
            // Mock inference: apply same logic as CPU but simulate Akida
            // In production, SNN would do this via spike propagation
            if mock_akida_inference(kmer) {
                kept_count += 1;
            }
        }
        
        Ok(kept_count)
    }
    
    /// Measure current power consumption of all boards
    fn measure_power(&self) -> f64 {
        // Sum power across all boards
        self.mesh.boards.iter().map(|b| b.power_watts).sum()
    }
    
    /// Get number of available boards
    pub fn board_count(&self) -> usize {
        self.mesh.boards.len()
    }
}

/// Mock Akida inference (simplified for demonstration)
fn mock_akida_inference(kmer: &[u8]) -> bool {
    use crate::kmer::{gc_content, is_adapter, is_low_complexity};
    
    // Same logic as CPU but representing what SNN learned
    let gc = gc_content(kmer);
    
    if gc < 0.4 || gc > 0.6 {
        return false;
    }
    
    if is_low_complexity(kmer) {
        return false;
    }
    
    if is_adapter(kmer) {
        return false;
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_akida_filter_creation() {
        // This test will fail without actual hardware, which is expected
        let result = AkidaFilter::new().await;
        
        match result {
            Ok(filter) => {
                println!("Akida filter created with {} board(s)", filter.board_count());
            }
            Err(e) => {
                println!("Expected: No Akida boards detected - {}", e);
            }
        }
    }
    
    #[test]
    fn test_mock_inference() {
        // Good k-mer
        let good = b"ACGTACGTACGTACGTACGTACGTACGTACG";
        assert!(mock_akida_inference(good));
        
        // Bad k-mer (low GC)
        let bad = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert!(!mock_akida_inference(bad));
    }
}

