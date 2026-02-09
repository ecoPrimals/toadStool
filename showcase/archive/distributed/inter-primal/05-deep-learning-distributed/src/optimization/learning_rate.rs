// Learning rate scheduling and scaling

/// Learning rate scheduler
#[derive(Debug, Clone)]
pub enum LRScheduler {
    /// Constant learning rate
    Constant,
    
    /// Step decay: multiply by factor every N epochs
    StepDecay {
        step_size: usize,
        gamma: f64,
    },
    
    /// Exponential decay
    Exponential {
        gamma: f64,
    },
    
    /// Cosine annealing
    CosineAnnealing {
        t_max: usize,
        eta_min: f64,
    },
    
    /// Warmup then decay
    WarmupDecay {
        warmup_epochs: usize,
        total_epochs: usize,
    },
    
    /// One cycle policy
    OneCycle {
        max_lr: f64,
        total_steps: usize,
        pct_start: f64,
    },
}

impl LRScheduler {
    /// Calculate learning rate for given epoch
    pub fn get_lr(&self, epoch: usize, base_lr: f64) -> f64 {
        match self {
            Self::Constant => base_lr,
            
            Self::StepDecay { step_size, gamma } => {
                let num_decays = epoch / step_size;
                base_lr * gamma.powi(num_decays as i32)
            }
            
            Self::Exponential { gamma } => {
                base_lr * gamma.powi(epoch as i32)
            }
            
            Self::CosineAnnealing { t_max, eta_min } => {
                let progress = (epoch as f64) / (*t_max as f64);
                let progress = progress.min(1.0);
                eta_min + (base_lr - eta_min) * 
                    (1.0 + (std::f64::consts::PI * progress).cos()) / 2.0
            }
            
            Self::WarmupDecay { warmup_epochs, total_epochs } => {
                if epoch < *warmup_epochs {
                    // Linear warmup
                    base_lr * (epoch as f64 + 1.0) / (*warmup_epochs as f64)
                } else {
                    // Cosine decay
                    let progress = (epoch - warmup_epochs) as f64 / 
                        (total_epochs - warmup_epochs) as f64;
                    let progress = progress.min(1.0);
                    base_lr * (1.0 + (std::f64::consts::PI * progress).cos()) / 2.0
                }
            }
            
            Self::OneCycle { max_lr, total_steps, pct_start } => {
                let step = epoch;
                let warmup_steps = (*total_steps as f64 * pct_start) as usize;
                
                if step < warmup_steps {
                    // Warmup phase
                    base_lr + (max_lr - base_lr) * (step as f64) / (warmup_steps as f64)
                } else {
                    // Annealing phase
                    let progress = (step - warmup_steps) as f64 / 
                        (*total_steps - warmup_steps) as f64;
                    let progress = progress.min(1.0);
                    max_lr * (1.0 + (std::f64::consts::PI * progress).cos()) / 2.0
                }
            }
        }
    }
    
    /// Get recommended scheduler for CIFAR-10
    pub fn cifar10_default(total_epochs: usize) -> Self {
        Self::WarmupDecay {
            warmup_epochs: 5,
            total_epochs,
        }
    }
    
    /// Get recommended scheduler for ImageNet
    pub fn imagenet_default(_total_epochs: usize) -> Self {
        Self::StepDecay {
            step_size: 30,
            gamma: 0.1,
        }
    }
}

/// Learning rate finder (to find optimal LR)
pub struct LRFinder {
    start_lr: f64,
    end_lr: f64,
    num_steps: usize,
}

impl LRFinder {
    pub fn new(start_lr: f64, end_lr: f64, num_steps: usize) -> Self {
        Self {
            start_lr,
            end_lr,
            num_steps,
        }
    }
    
    /// Get learning rate for step
    pub fn get_lr(&self, step: usize) -> f64 {
        let ratio = (step as f64) / (self.num_steps as f64);
        self.start_lr * (self.end_lr / self.start_lr).powf(ratio)
    }
    
    /// Find optimal LR from loss curve
    pub fn find_optimal(&self, losses: &[f64]) -> f64 {
        // Find LR with steepest descent
        let mut best_lr = self.start_lr;
        let mut best_gradient = 0.0;
        
        for i in 1..losses.len() {
            let gradient = (losses[i-1] - losses[i]) / losses[i-1];
            if gradient > best_gradient {
                best_gradient = gradient;
                best_lr = self.get_lr(i);
            }
        }
        
        best_lr
    }
}

/// Calculate learning rate for distributed training
pub fn distributed_lr(
    base_lr: f64,
    batch_size: usize,
    base_batch_size: usize,
    num_towers: usize,
    strategy: LRScalingStrategy,
) -> f64 {
    match strategy {
        LRScalingStrategy::Linear => {
            // Linear scaling rule: lr ∝ batch_size
            base_lr * (batch_size as f64) / (base_batch_size as f64)
        }
        
        LRScalingStrategy::SquareRoot => {
            // Square root scaling
            base_lr * ((batch_size as f64) / (base_batch_size as f64)).sqrt()
        }
        
        LRScalingStrategy::TowerBased => {
            // Scale by number of towers
            base_lr * (num_towers as f64)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LRScalingStrategy {
    Linear,
    SquareRoot,
    TowerBased,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constant_scheduler() {
        let scheduler = LRScheduler::Constant;
        let base_lr = 0.1;
        
        assert_eq!(scheduler.get_lr(0, base_lr), 0.1);
        assert_eq!(scheduler.get_lr(10, base_lr), 0.1);
        assert_eq!(scheduler.get_lr(100, base_lr), 0.1);
    }
    
    #[test]
    fn test_step_decay() {
        let scheduler = LRScheduler::StepDecay {
            step_size: 10,
            gamma: 0.1,
        };
        let base_lr = 0.1;
        
        assert!((scheduler.get_lr(0, base_lr) - 0.1).abs() < 1e-9);
        assert!((scheduler.get_lr(9, base_lr) - 0.1).abs() < 1e-9);
        assert!((scheduler.get_lr(10, base_lr) - 0.01).abs() < 1e-9);
        assert!((scheduler.get_lr(20, base_lr) - 0.001).abs() < 1e-9);
    }
    
    #[test]
    fn test_cosine_annealing() {
        let scheduler = LRScheduler::CosineAnnealing {
            t_max: 100,
            eta_min: 0.0,
        };
        let base_lr = 0.1;
        
        let lr_start = scheduler.get_lr(0, base_lr);
        let lr_mid = scheduler.get_lr(50, base_lr);
        let lr_end = scheduler.get_lr(100, base_lr);
        
        assert!(lr_start > lr_mid);
        assert!(lr_mid > lr_end);
        assert!((lr_start - 0.1).abs() < 0.001);
        assert!(lr_end < 0.01);
    }
    
    #[test]
    fn test_warmup_decay() {
        let scheduler = LRScheduler::WarmupDecay {
            warmup_epochs: 5,
            total_epochs: 100,
        };
        let base_lr = 0.1;
        
        // During warmup, LR should increase
        let lr1 = scheduler.get_lr(1, base_lr);
        let lr3 = scheduler.get_lr(3, base_lr);
        assert!(lr3 > lr1);
        
        // After warmup, LR should decrease
        let lr10 = scheduler.get_lr(10, base_lr);
        let lr50 = scheduler.get_lr(50, base_lr);
        assert!(lr10 > lr50);
    }
    
    #[test]
    fn test_distributed_lr() {
        let base_lr = 0.1;
        let base_batch_size = 128;
        
        // Linear: 2x batch size → 2x LR
        let lr = distributed_lr(
            base_lr,
            256,
            base_batch_size,
            2,
            LRScalingStrategy::Linear,
        );
        assert_eq!(lr, 0.2);
        
        // Square root: 4x batch size → 2x LR
        let lr = distributed_lr(
            base_lr,
            512,
            base_batch_size,
            2,
            LRScalingStrategy::SquareRoot,
        );
        assert!((lr - 0.2).abs() < 0.001);
    }
    
    #[test]
    fn test_lr_finder() {
        let finder = LRFinder::new(1e-5, 1.0, 100);
        
        let lr_start = finder.get_lr(0);
        let lr_mid = finder.get_lr(50);
        let lr_end = finder.get_lr(100);
        
        assert!(lr_start < lr_mid);
        assert!(lr_mid < lr_end);
        assert!((lr_start - 1e-5).abs() < 1e-6);
        assert!((lr_end - 1.0).abs() < 0.01);
    }
}

