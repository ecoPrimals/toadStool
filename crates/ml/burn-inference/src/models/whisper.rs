//! Whisper speech recognition model
//!
//! Type-safe API surface for Whisper ASR inference.
//!
//! # Requirements
//!
//! Whisper inference requires:
//! - **Model weights**: Load from HuggingFace (e.g. `openai/whisper-tiny`) or local safetensors
//! - **Burn backend**: Enable `burn` with wgpu or ndarray backend in Cargo.toml
//! - **Audio**: 16 kHz mono PCM float32 input
//!
//! # Example
//!
//! ```ignore
//! let whisper = Whisper::from_pretrained("openai/whisper-tiny")?;
//! let text = whisper.transcribe(&audio, 16000)?;
//! ```

use crate::Error::ModelBackendRequired;
use crate::Error::ModelNotLoaded;
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

    /// Load from HuggingFace Hub or local safetensors.
    ///
    /// **Requires**: Model weights. Load with `WhisperModel::from_safetensors(path)` once integrated.
    pub fn from_pretrained(model_id: &str) -> Result<Self> {
        Err(ModelNotLoaded(format!(
            "Whisper model weights required. Requested: {model_id}. \
             Load with WhisperModel::from_safetensors(path) once burn backend is integrated."
        )))
    }

    /// Transcribe audio (16 kHz mono PCM float32).
    ///
    /// **Requires**: Model weights loaded via `Whisper::from_pretrained` or
    /// `WhisperModel::from_safetensors(path)`, plus burn backend (wgpu/ndarray).
    pub fn transcribe(&self, _audio: &[f32], _sample_rate: u32) -> Result<String> {
        Err(ModelBackendRequired(
            "Whisper inference requires model weights. Load with WhisperModel::from_safetensors(path). \
             Ensure burn backend (wgpu/ndarray) is enabled in Cargo.toml.".into(),
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
    fn test_whisper_from_pretrained_requires_weights() {
        let result = Whisper::from_pretrained("openai/whisper-tiny");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights required"));
        assert!(err.contains("from_safetensors"));
    }

    #[test]
    fn test_whisper_transcribe_requires_backend() {
        let w = Whisper::new(WhisperConfig::default());
        let result = w.transcribe(&[0.0f32; 16000], 16000);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("model weights"));
        assert!(err.contains("from_safetensors"));
    }
}
