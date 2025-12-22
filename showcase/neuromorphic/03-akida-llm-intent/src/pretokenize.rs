//! Pre-tokenization and feature extraction for Akida

use ndarray::{Array1, Array2};

/// Simple tokenizer for intent classification
pub struct SimpleTokenizer {
    vocab_size: usize,
}

impl SimpleTokenizer {
    pub fn new(vocab_size: usize) -> Self {
        Self { vocab_size }
    }
    
    /// Tokenize text into character-level features
    pub fn tokenize(&self, text: &str) -> anyhow::Result<Vec<usize>> {
        let tokens: Vec<usize> = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| (c as usize) % self.vocab_size)
            .collect();
        
        Ok(tokens)
    }
    
    /// Convert tokens to spike train (for SNN input)
    pub fn to_spike_train(&self, tokens: &[usize], max_len: usize) -> anyhow::Result<Array2<f32>> {
        let mut spike_train = Array2::zeros((max_len, self.vocab_size));
        
        for (i, &token) in tokens.iter().take(max_len).enumerate() {
            if token < self.vocab_size {
                spike_train[[i, token]] = 1.0;
            }
        }
        
        Ok(spike_train)
    }
    
    /// Convert tokens to dense feature vector
    pub fn to_feature_vector(&self, tokens: &[usize]) -> anyhow::Result<Array1<f32>> {
        let mut features = Array1::zeros(self.vocab_size);
        
        // Bag-of-tokens representation
        for &token in tokens {
            if token < self.vocab_size {
                features[token] += 1.0;
            }
        }
        
        // Normalize
        let sum: f32 = features.sum();
        if sum > 0.0 {
            features /= sum;
        }
        
        Ok(features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tokenizer() {
        let tokenizer = SimpleTokenizer::new(128);
        
        let tokens = tokenizer.tokenize("hello world").unwrap();
        assert!(!tokens.is_empty());
        assert!(tokens.iter().all(|&t| t < 128));
    }
    
    #[test]
    fn test_spike_train() {
        let tokenizer = SimpleTokenizer::new(128);
        let tokens = tokenizer.tokenize("test").unwrap();
        
        let spike_train = tokenizer.to_spike_train(&tokens, 10).unwrap();
        assert_eq!(spike_train.shape(), &[10, 128]);
    }
    
    #[test]
    fn test_feature_vector() {
        let tokenizer = SimpleTokenizer::new(128);
        let tokens = tokenizer.tokenize("test").unwrap();
        
        let features = tokenizer.to_feature_vector(&tokens).unwrap();
        assert_eq!(features.len(), 128);
        
        // Should be normalized
        let sum: f32 = features.sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }
}

