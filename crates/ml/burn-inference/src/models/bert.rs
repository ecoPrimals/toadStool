//! BERT model implementation
//!
//! Type-safe API surface for BERT transformer inference.
//! Inference methods return `NotImplemented` until a model backend is integrated.

use crate::Result;

/// BERT configuration
#[derive(Debug, Clone)]
pub struct BertConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
}

impl Default for BertConfig {
    fn default() -> Self {
        // BERT-base configuration
        Self {
            vocab_size: 30522,
            hidden_size: 768,
            num_hidden_layers: 12,
            num_attention_heads: 12,
            intermediate_size: 3072,
            max_position_embeddings: 512,
        }
    }
}

/// BERT model
#[derive(Debug)]
pub struct Bert {
    config: BertConfig,
}

impl Bert {
    /// Create new BERT model
    pub fn new(config: BertConfig) -> Self {
        Self { config }
    }

    /// Access model configuration
    pub fn config(&self) -> &BertConfig {
        &self.config
    }

    /// Load from HuggingFace Hub
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(crate::Error::NotImplemented(format!(
            "BERT model loading not yet integrated (requested: {model_id})"
        )))
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
        110_000_000
    }

    /// Run inference
    pub fn forward(&self, _input_ids: &[u32]) -> Result<Vec<f32>> {
        Err(crate::Error::NotImplemented(
            "BERT inference requires a model backend (burn/onnx/wgsl)".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bert_config_default() {
        let cfg = BertConfig::default();
        assert_eq!(cfg.vocab_size, 30522);
        assert_eq!(cfg.hidden_size, 768);
        assert_eq!(cfg.num_hidden_layers, 12);
        assert_eq!(cfg.num_attention_heads, 12);
        assert_eq!(cfg.intermediate_size, 3072);
        assert_eq!(cfg.max_position_embeddings, 512);
    }

    #[test]
    fn test_bert_new_and_parameters() {
        let bert = Bert::new(BertConfig::default());
        assert_eq!(bert.num_parameters(), 110_000_000);
    }

    #[test]
    fn test_bert_from_pretrained_not_implemented() {
        let result = Bert::from_pretrained("bert-base-uncased");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not yet integrated"));
    }

    #[test]
    fn test_bert_forward_not_implemented() {
        let bert = Bert::new(BertConfig::default());
        let result = bert.forward(&[101, 7592, 1010, 2088, 102]);
        assert!(result.is_err());
    }

    #[test]
    fn test_bert_custom_config() {
        let cfg = BertConfig {
            vocab_size: 10000,
            hidden_size: 256,
            num_hidden_layers: 4,
            num_attention_heads: 4,
            intermediate_size: 512,
            max_position_embeddings: 128,
        };
        let bert = Bert::new(cfg);
        assert_eq!(bert.num_parameters(), 110_000_000);
    }
}
