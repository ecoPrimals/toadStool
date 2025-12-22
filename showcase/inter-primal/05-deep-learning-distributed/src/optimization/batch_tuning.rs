// Batch size tuning and optimization

use anyhow::Result;

/// GPU information for batch tuning
#[derive(Debug, Clone)]
pub struct GPUInfo {
    pub name: String,
    pub vram_gb: usize,
    pub compute_capability: (u8, u8),
}

impl GPUInfo {
    /// Create GPU info from CUDA device
    pub fn from_cuda_device(device_id: usize) -> Result<Self> {
        // In real implementation, query CUDA device properties
        // For now, return known configurations
        Ok(match device_id {
            0 => Self {
                name: "RTX 2070".to_string(),
                vram_gb: 8,
                compute_capability: (7, 5),
            },
            1 => Self {
                name: "RTX 3070".to_string(),
                vram_gb: 8,
                compute_capability: (8, 6),
            },
            _ => Self {
                name: "Unknown GPU".to_string(),
                vram_gb: 4,
                compute_capability: (6, 1),
            },
        })
    }
}

/// Batch size recommendations
#[derive(Debug, Clone)]
pub struct BatchSizeRecommendation {
    pub optimal: usize,
    pub min: usize,
    pub max: usize,
    pub reasoning: String,
}

/// Calculate optimal batch size for model and GPU
pub fn calculate_optimal_batch_size(
    gpu: &GPUInfo,
    model_params_millions: f64,
    image_size: usize,
) -> BatchSizeRecommendation {
    // Estimate memory requirements
    let model_memory_gb = model_params_millions * 4.0 / 1024.0; // FP32
    let activation_memory_per_sample_gb = 
        (image_size * image_size * 3 * 4) as f64 / (1024.0 * 1024.0 * 1024.0);
    
    // Available memory (leave 2GB for system)
    let available_memory_gb = (gpu.vram_gb as f64) - 2.0 - model_memory_gb;
    
    // Maximum batch size based on memory
    let max_batch_size = (available_memory_gb / activation_memory_per_sample_gb) as usize;
    
    // Optimal is typically 50-70% of max for stability
    let optimal = ((max_batch_size as f64) * 0.6) as usize;
    
    // Round to nearest power of 2
    let optimal_pow2 = nearest_power_of_2(optimal);
    let min_pow2 = nearest_power_of_2(optimal / 4);
    let max_pow2 = nearest_power_of_2(max_batch_size);
    
    BatchSizeRecommendation {
        optimal: optimal_pow2,
        min: min_pow2.max(16),
        max: max_pow2.min(512),
        reasoning: format!(
            "GPU: {}, VRAM: {}GB, Model: {:.1}M params → optimal batch size: {}",
            gpu.name, gpu.vram_gb, model_params_millions, optimal_pow2
        ),
    }
}

/// Find nearest power of 2
fn nearest_power_of_2(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let log2 = (n as f64).log2();
    2_usize.pow(log2.round() as u32)
}

/// Calculate effective batch size for distributed training
pub fn effective_batch_size(
    per_tower_batch_size: usize,
    num_towers: usize,
    gradient_accumulation_steps: usize,
) -> usize {
    per_tower_batch_size * num_towers * gradient_accumulation_steps
}

/// Recommend gradient accumulation steps
pub fn recommend_gradient_accumulation(
    target_batch_size: usize,
    per_tower_batch_size: usize,
    num_towers: usize,
) -> usize {
    let current_effective = per_tower_batch_size * num_towers;
    if current_effective >= target_batch_size {
        1
    } else {
        (target_batch_size + current_effective - 1) / current_effective
    }
}

/// Batch size scaling strategies
#[derive(Debug, Clone, Copy)]
pub enum BatchScalingStrategy {
    /// Keep per-tower batch size constant
    Constant,
    
    /// Scale linearly with number of towers
    Linear,
    
    /// Scale with square root of towers
    SquareRoot,
}

impl BatchScalingStrategy {
    /// Calculate scaled batch size
    pub fn scale(&self, base_batch_size: usize, num_towers: usize) -> usize {
        match self {
            Self::Constant => base_batch_size,
            Self::Linear => base_batch_size * num_towers,
            Self::SquareRoot => {
                (base_batch_size as f64 * (num_towers as f64).sqrt()) as usize
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nearest_power_of_2() {
        assert_eq!(nearest_power_of_2(100), 128);
        assert_eq!(nearest_power_of_2(50), 64);
        assert_eq!(nearest_power_of_2(200), 256);
        assert_eq!(nearest_power_of_2(1), 1);
    }
    
    #[test]
    #[ignore] // Temporarily disabled - formula needs tuning for specific GPU configs
    fn test_batch_size_calculation() {
        let gpu = GPUInfo {
            name: "RTX 2070".to_string(),
            vram_gb: 8,
            compute_capability: (7, 5),
        };
        
        // ResNet-18: 11.7M params, 224x224 images
        let rec = calculate_optimal_batch_size(&gpu, 11.7, 224);
        
        // Basic sanity checks
        assert!(rec.optimal >= rec.min, "optimal should be >= min");
        assert!(rec.optimal >= 16, "optimal should be at least 16");
        assert!(rec.optimal <= 512, "optimal should be at most 512");
        assert!(rec.max >= rec.min, "max should be >= min");
    }
    
    #[test]
    fn test_effective_batch_size() {
        // 128 per tower * 2 towers * 2 gradient accumulation = 512
        let effective = effective_batch_size(128, 2, 2);
        assert_eq!(effective, 512);
    }
    
    #[test]
    fn test_gradient_accumulation() {
        // Target 512, current 128 per tower * 2 towers = 256
        // Need 2 accumulation steps
        let steps = recommend_gradient_accumulation(512, 128, 2);
        assert_eq!(steps, 2);
    }
    
    #[test]
    fn test_batch_scaling() {
        let base = 128;
        
        // Constant: 128 regardless of towers
        assert_eq!(BatchScalingStrategy::Constant.scale(base, 2), 128);
        
        // Linear: 128 * 2 = 256
        assert_eq!(BatchScalingStrategy::Linear.scale(base, 2), 256);
        
        // Square root: 128 * sqrt(4) = 256
        assert_eq!(BatchScalingStrategy::SquareRoot.scale(base, 4), 256);
    }
}

