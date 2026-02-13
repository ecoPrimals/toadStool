//! Pre-defined model architectures
//!
//! This module provides common model architectures that can be loaded
//! from HuggingFace Hub or local files.

pub mod bert;
pub mod whisper;
pub mod vision;

use crate::Result;

/// Model trait for unified interface
pub trait Model {
    /// Model input type
    type Input;
    /// Model output type
    type Output;
    
    /// Run inference
    fn forward(&self, input: &Self::Input) -> Result<Self::Output>;
    
    /// Get model name
    fn name(&self) -> &str;
    
    /// Get number of parameters
    fn num_parameters(&self) -> usize;
}

/// Common model metadata
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub num_parameters: usize,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
}
