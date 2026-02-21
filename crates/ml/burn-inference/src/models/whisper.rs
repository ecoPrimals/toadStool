//! Whisper speech recognition model
//!
//! Type-safe API surface for Whisper ASR inference.
//! Inference methods return `NotImplemented` until a model backend is integrated.

use crate::Result;

/// Whisper configuration
#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub d_model: usize,
    pub encoder_layers: usize,
    pub decoder_layers: usize,
    pub encoder_attention_heads: usize,
    pub decoder_attention_heads: usize,
    pub vocab_size: usize,
}

impl WhisperConfig {
    /// Whisper-tiny configuration
    pub fn tiny() -> Self {
        Self {
            d_model: 384,
            encoder_layers: 4,
            decoder_layers: 4,
            encoder_attention_heads: 6,
            decoder_attention_heads: 6,
            vocab_size: 51865,
        }
    }

    /// Whisper-base configuration
    pub fn base() -> Self {
        Self {
            d_model: 512,
            encoder_layers: 6,
            decoder_layers: 6,
            encoder_attention_heads: 8,
            decoder_attention_heads: 8,
            vocab_size: 51865,
        }
    }
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self::tiny()
    }
}

/// Whisper model
#[derive(Debug)]
pub struct Whisper {
    config: WhisperConfig,
}

impl Whisper {
    /// Create new Whisper model
    pub fn new(config: WhisperConfig) -> Self {
        Self { config }
    }

    /// Load from HuggingFace Hub
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(crate::Error::NotImplemented(format!(
            "Whisper model loading not yet integrated (requested: {model_id})"
        )))
    }

    /// Transcribe audio
    pub fn transcribe(&self, _audio: &[f32], _sample_rate: u32) -> Result<String> {
        Err(crate::Error::NotImplemented(
            "Whisper inference requires a model backend (burn/onnx/wgsl)".into(),
        ))
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
        match self.config.d_model {
            384 => 39_000_000, // tiny
            512 => 74_000_000, // base
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_config_tiny() {
        let cfg = WhisperConfig::tiny();
        assert_eq!(cfg.d_model, 384);
        assert_eq!(cfg.encoder_layers, 4);
        assert_eq!(cfg.vocab_size, 51865);
    }

    #[test]
    fn test_whisper_config_base() {
        let cfg = WhisperConfig::base();
        assert_eq!(cfg.d_model, 512);
        assert_eq!(cfg.encoder_layers, 6);
    }

    #[test]
    fn test_whisper_config_default_is_tiny() {
        let cfg = WhisperConfig::default();
        assert_eq!(cfg.d_model, 384);
    }

    #[test]
    fn test_whisper_num_parameters_tiny() {
        let w = Whisper::new(WhisperConfig::tiny());
        assert_eq!(w.num_parameters(), 39_000_000);
    }

    #[test]
    fn test_whisper_num_parameters_base() {
        let w = Whisper::new(WhisperConfig::base());
        assert_eq!(w.num_parameters(), 74_000_000);
    }

    #[test]
    fn test_whisper_from_pretrained_not_implemented() {
        let result = Whisper::from_pretrained("openai/whisper-tiny");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not yet integrated"));
    }

    #[test]
    fn test_whisper_transcribe_not_implemented() {
        let w = Whisper::new(WhisperConfig::default());
        let result = w.transcribe(&[0.0f32; 16000], 16000);
        assert!(result.is_err());
    }
}
