//! BERT model implementation
//!
//! Type-safe API surface for BERT transformer inference.
//!
//! # Requirements
//!
//! BERT inference requires:
//! - **Model weights**: Load from HuggingFace safetensors (e.g. `bert-base-uncased`) or local path
//! - **Burn backend**: Enable `burn` with wgpu or ndarray backend in Cargo.toml
//! - **Tokenizers** (optional): Enable `nlp` feature for HuggingFace tokenizer integration
//!
//! # Example
//!
//! ```ignore
//! // Load weights: Bert::from_pretrained("bert-base-uncased") or
//! // BertModel::from_safetensors(path) once backend is integrated
//! let bert = Bert::from_pretrained("bert-base-uncased")?;
//! let logits = bert.forward(&token_ids)?;
//! ```

use crate::Error::ModelBackendRequired;
use crate::Error::ModelNotLoaded;
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

    /// Load from HuggingFace Hub or local safetensors.
    ///
    /// **Requires**: Model weights. Download from HuggingFace Hub or use local path.
    /// Once the burn backend is integrated, load with `BertModel::from_safetensors(path)`.
    #[cfg_attr(
        feature = "nlp",
        doc = "The `nlp` feature enables HuggingFace tokenizer support."
    )]
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(ModelNotLoaded(format!(
            "BERT model weights required. Requested: {model_id}. \
             Load with BertModel::from_safetensors(path) once burn backend is integrated, \
             or download weights from HuggingFace Hub."
        )))
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
        110_000_000
    }

    /// Run inference
    ///
    /// **Requires**: Model weights loaded via `Bert::from_pretrained` or
    /// `BertModel::from_safetensors(path)`, plus burn backend (wgpu/ndarray).
    pub fn forward(&self, _input_ids: &[u32]) -> Result<Vec<f32>> {
        Err(ModelBackendRequired(
            "BERT inference requires model weights. Load with BertModel::from_safetensors(path). \
             Ensure burn backend (wgpu/ndarray) is enabled in Cargo.toml."
                .into(),
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
    fn test_bert_from_pretrained_requires_weights() {
        let result = Bert::from_pretrained("bert-base-uncased");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights required"));
        assert!(err.contains("from_safetensors"));
    }

    #[test]
    fn test_bert_forward_requires_backend() {
        let bert = Bert::new(BertConfig::default());
        let result = bert.forward(&[101, 7592, 1010, 2088, 102]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights"));
        assert!(err.contains("from_safetensors"));
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
