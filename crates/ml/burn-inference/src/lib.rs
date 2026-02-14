//! Burn-based ML inference for ToadStool
//!
//! This crate provides cross-platform ML inference using Burn with wgpu backend.
//! It enables running HuggingFace models on NVIDIA, AMD, or any wgpu-compatible hardware.
//!
//! # Example
//!
//! ```ignore
//! use burn_inference::{BurnDevice, InferenceEngine};
//!
//! let device = BurnDevice::auto_select();
//! let engine = InferenceEngine::new(device);
//! let result = engine.infer(&input).await?;
//! ```

pub mod device;
pub mod engine;
pub mod loaders;
pub mod models;

pub use device::{BurnDevice, DeviceInfo};
pub use engine::InferenceEngine;

/// Crate-level error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Device initialization failed: {0}")]
    DeviceInit(String),

    #[error("Model loading failed: {0}")]
    ModelLoad(String),

    #[error("Inference failed: {0}")]
    Inference(String),

    #[error("Unsupported model type: {0}")]
    UnsupportedModel(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
