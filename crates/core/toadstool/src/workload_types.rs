// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! AI/ML and CUDA workload type definitions
//!
//! This module defines enhanced workload types for intelligent backend selection
//! based on workload characteristics rather than hardware availability.
//!
//! # Philosophy
//!
//! ToadStool is **workload-centric**, not hardware-centric:
//! - Analyze what the workload needs
//! - Select optimal backend for those needs
//! - Adapt to available hardware
//! - Provide transparent fallbacks
//!
//! # Example
//!
//! ```rust
//! use toadstool::workload_types::{AiMlWorkload, AiFramework, AiOperation, ModelSize};
//!
//! let workload = AiMlWorkload {
//!     framework: AiFramework::PyTorch,
//!     operation: AiOperation::Training,
//!     model_size: ModelSize::Large,
//!     batch_size: 64,
//! };
//!
//! // ToadStool will automatically select:
//! // - Native CUDA if NVIDIA GPU available
//! // - Translated GPU if AMD/Intel/Apple GPU available
//! // - CPU parallel if high-core-count CPU available
//! // - CPU sequential as final fallback
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// AI/ML workload specification
///
/// Describes an AI/ML workload for intelligent backend selection.
/// The framework, operation type, model size, and batch size inform
/// which backend will provide optimal performance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AiMlWorkload {
    /// ML framework being used
    pub framework: AiFramework,

    /// Operation type (training, inference, etc.)
    pub operation: AiOperation,

    /// Model size category
    pub model_size: ModelSize,

    /// Batch size for the operation
    pub batch_size: usize,
}

impl AiMlWorkload {
    /// Create a new AI/ML workload specification
    pub fn new(
        framework: AiFramework,
        operation: AiOperation,
        model_size: ModelSize,
        batch_size: usize,
    ) -> Self {
        Self {
            framework,
            operation,
            model_size,
            batch_size,
        }
    }

    /// Estimate memory requirements in bytes
    pub fn estimate_memory_bytes(&self) -> u64 {
        let base_memory = match self.model_size {
            ModelSize::Small => 100 * 1024 * 1024,       // 100 MB
            ModelSize::Medium => 500 * 1024 * 1024,      // 500 MB
            ModelSize::Large => 5 * 1024 * 1024 * 1024,  // 5 GB
            ModelSize::XLarge => 20 * 1024 * 1024 * 1024, // 20 GB
            ModelSize::XXLarge => 100 * 1024 * 1024 * 1024, // 100 GB
        };

        // Training needs more memory for gradients, optimizer state
        let multiplier = match self.operation {
            AiOperation::Training => 3,
            AiOperation::FineTuning => 2,
            _ => 1,
        };

        // Batch size increases memory linearly
        let batch_multiplier = (self.batch_size.max(1) as f64 / 32.0).max(1.0);

        (base_memory as f64 * multiplier as f64 * batch_multiplier) as u64
    }
}

impl fmt::Display for AiMlWorkload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {} on {:?} model (batch={})",
            self.framework, self.operation, self.model_size, self.batch_size
        )
    }
}

/// Machine learning framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiFramework {
    /// PyTorch (most popular, strong GPU support)
    PyTorch,

    /// TensorFlow (Google's framework)
    TensorFlow,

    /// JAX (Google's high-performance ML)
    JAX,

    /// ONNX Runtime (cross-platform inference)
    ONNX,

    /// Burn (Rust ML framework)
    Burn,

    /// Candle (Rust ML framework by HuggingFace)
    Candle,

    /// Custom or unknown framework
    Custom,
}

impl fmt::Display for AiFramework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PyTorch => write!(f, "PyTorch"),
            Self::TensorFlow => write!(f, "TensorFlow"),
            Self::JAX => write!(f, "JAX"),
            Self::ONNX => write!(f, "ONNX"),
            Self::Burn => write!(f, "Burn"),
            Self::Candle => write!(f, "Candle"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// Type of AI/ML operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiOperation {
    /// Model training (most compute-intensive)
    Training,

    /// Inference (forward pass only)
    Inference,

    /// Fine-tuning a pre-trained model
    FineTuning,

    /// Model evaluation/validation
    Evaluation,
}

impl fmt::Display for AiOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Training => write!(f, "training"),
            Self::Inference => write!(f, "inference"),
            Self::FineTuning => write!(f, "fine-tuning"),
            Self::Evaluation => write!(f, "evaluation"),
        }
    }
}

/// Model size category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModelSize {
    /// Small models (<100MB)
    Small,

    /// Medium models (100MB-1GB)
    Medium,

    /// Large models (1-10GB)
    Large,

    /// Extra large models (10-100GB)
    XLarge,

    /// Huge models (100GB+)
    XXLarge,
}

impl fmt::Display for ModelSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Small => write!(f, "Small (<100MB)"),
            Self::Medium => write!(f, "Medium (100MB-1GB)"),
            Self::Large => write!(f, "Large (1-10GB)"),
            Self::XLarge => write!(f, "XLarge (10-100GB)"),
            Self::XXLarge => write!(f, "XXLarge (100GB+)"),
        }
    }
}

/// CUDA workload specification
///
/// Describes a CUDA kernel for compatibility layer routing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CudaWorkload {
    /// CUDA source code or compiled PTX
    pub kernel_source: CudaSource,

    /// Required compute capability (e.g., "7.5", "8.6")
    pub compute_capability: Option<String>,

    /// User's preferred backend (optional)
    pub preferred_backend: Option<CudaBackend>,
}

impl CudaWorkload {
    /// Create a new CUDA workload
    pub fn new(kernel_source: CudaSource) -> Self {
        Self {
            kernel_source,
            compute_capability: None,
            preferred_backend: None,
        }
    }

    /// Set compute capability requirement
    /// ✅ ZERO-COPY: Accept any string-like type to avoid unnecessary clones
    #[must_use]
    pub fn with_compute_capability(mut self, capability: impl Into<String>) -> Self {
        self.compute_capability = Some(capability.into());
        self
    }

    /// Set preferred backend
    #[must_use]
    pub fn with_preferred_backend(mut self, backend: CudaBackend) -> Self {
        self.preferred_backend = Some(backend);
        self
    }
}

/// CUDA kernel source format
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CudaSource {
    /// CUDA C++ source code
    SourceCode(String),

    /// Compiled PTX (parallel thread execution)
    Ptx(Vec<u8>),

    /// Compiled CUBIN (CUDA binary)
    Cubin(Vec<u8>),

    /// Pre-compiled FATBIN (multi-architecture)
    Fatbin(Vec<u8>),
}

/// CUDA execution backend
///
/// Represents different ways to execute CUDA code on available hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CudaBackend {
    /// Native NVIDIA GPU with real CUDA runtime (100% compatibility)
    NativeNvidia,

    /// Non-NVIDIA GPU via ToadStool translation layer (80-95% performance)
    TranslatedGpu,

    /// Multi-core CPU with parallel execution (50-70% of GPU)
    CpuParallel,

    /// Single-threaded CPU fallback (5-10% of GPU, always works)
    CpuSequential,
}

impl fmt::Display for CudaBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeNvidia => write!(f, "Native NVIDIA CUDA"),
            Self::TranslatedGpu => write!(f, "Translated GPU"),
            Self::CpuParallel => write!(f, "CPU Parallel"),
            Self::CpuSequential => write!(f, "CPU Sequential"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aiml_workload_creation() {
        let workload = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64,
        );

        assert_eq!(workload.framework, AiFramework::PyTorch);
        assert_eq!(workload.operation, AiOperation::Training);
        assert_eq!(workload.model_size, ModelSize::Large);
        assert_eq!(workload.batch_size, 64);
    }

    #[test]
    fn test_aiml_memory_estimation_small() {
        let workload = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            32,
        );

        let memory = workload.estimate_memory_bytes();
        assert_eq!(memory, 100 * 1024 * 1024); // 100 MB base
    }

    #[test]
    fn test_aiml_memory_estimation_training() {
        let workload = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Medium,
            32,
        );

        let memory = workload.estimate_memory_bytes();
        // 500 MB base * 3 (training multiplier) = 1.5 GB
        assert_eq!(memory, 1_500_000_000);
    }

    #[test]
    fn test_aiml_memory_estimation_large_batch() {
        let workload = AiMlWorkload::new(
            AiFramework::TensorFlow,
            AiOperation::Inference,
            ModelSize::Large,
            128, // 4x the base batch size
        );

        let memory = workload.estimate_memory_bytes();
        // 5 GB base * 1 (inference) * 4 (batch multiplier) = 20 GB
        assert_eq!(memory, 20 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cuda_workload_creation() {
        let source = CudaSource::SourceCode("__global__ void kernel() {}".to_string());
        let workload = CudaWorkload::new(source)
            .with_compute_capability("8.6".to_string())
            .with_preferred_backend(CudaBackend::NativeNvidia);

        assert!(workload.compute_capability.is_some());
        assert_eq!(
            workload.compute_capability.as_deref(),
            Some("8.6"),
            "Compute capability should be 8.6"
        );
        assert_eq!(workload.preferred_backend, Some(CudaBackend::NativeNvidia));
    }

    #[test]
    fn test_model_size_ordering() {
        assert!(ModelSize::Small < ModelSize::Medium);
        assert!(ModelSize::Medium < ModelSize::Large);
        assert!(ModelSize::Large < ModelSize::XLarge);
        assert!(ModelSize::XLarge < ModelSize::XXLarge);
    }

    #[test]
    fn test_aiml_workload_display() {
        let workload = AiMlWorkload::new(
            AiFramework::Burn,
            AiOperation::Inference,
            ModelSize::Medium,
            16,
        );

        let display = format!("{}", workload);
        assert!(display.contains("Burn"));
        assert!(display.contains("inference"));
        assert!(display.contains("Medium"));
        assert!(display.contains("16"));
    }

    #[test]
    fn test_serialization() {
        let workload = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64,
        );

        let json = serde_json::to_string(&workload).expect("Serialization should succeed in test");
        let deserialized: AiMlWorkload =
            serde_json::from_str(&json).expect("Deserialization should succeed in test");

        assert_eq!(workload, deserialized);
    }
}

