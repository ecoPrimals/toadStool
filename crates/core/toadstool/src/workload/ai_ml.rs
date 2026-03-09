// SPDX-License-Identifier: AGPL-3.0-only
//! AI/ML workload types and characteristics
//!
//! This module defines workload-centric AI/ML types that enable intelligent
//! backend selection based on computational requirements rather than hardware.

use serde::{Deserialize, Serialize};
use std::fmt;

/// AI/ML framework identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AiFramework {
    /// PyTorch (Python ecosystem, CUDA-dependent in 2025)
    PyTorch,

    /// TensorFlow (Python ecosystem, CUDA-dependent in 2025)
    TensorFlow,

    /// JAX (Google's accelerated NumPy, CUDA-dependent)
    JAX,

    /// ONNX Runtime (cross-platform inference)
    ONNX,

    /// Burn (Rust ML framework, WebGPU-native)
    Burn,

    /// Candle (Rust ML framework by HuggingFace)
    Candle,

    /// Custom implementation
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
#[non_exhaustive]
pub enum AiOperation {
    /// Model training (compute-intensive, requires gradients)
    Training,

    /// Model inference (forward pass only)
    Inference,

    /// Fine-tuning pre-trained model
    FineTuning,

    /// Model evaluation/validation
    Evaluation,

    /// Quantization or compression
    Quantization,
}

impl fmt::Display for AiOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Training => write!(f, "Training"),
            Self::Inference => write!(f, "Inference"),
            Self::FineTuning => write!(f, "Fine-tuning"),
            Self::Evaluation => write!(f, "Evaluation"),
            Self::Quantization => write!(f, "Quantization"),
        }
    }
}

/// Model size classification for resource estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModelSize {
    /// Small models (<100MB) - MobileNet, small CNNs
    Small,

    /// Medium models (100MB-1GB) - ResNet-50, BERT-base
    Medium,

    /// Large models (1-10GB) - ResNet-152, GPT-2
    Large,

    /// Extra-large models (10-100GB) - GPT-3, large vision transformers
    XLarge,

    /// XXL models (100GB+) - GPT-4, massive multimodal models
    XXLarge,
}

impl ModelSize {
    /// Estimate memory footprint in bytes
    #[must_use]
    pub const fn estimate_memory_bytes(&self) -> u64 {
        match self {
            Self::Small => 50 * 1024 * 1024,           // 50MB
            Self::Medium => 500 * 1024 * 1024,         // 500MB
            Self::Large => 5 * 1024 * 1024 * 1024,     // 5GB
            Self::XLarge => 50 * 1024 * 1024 * 1024,   // 50GB
            Self::XXLarge => 200 * 1024 * 1024 * 1024, // 200GB
        }
    }

    /// Get human-readable size description
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Small => "Small (<100MB)",
            Self::Medium => "Medium (100MB-1GB)",
            Self::Large => "Large (1-10GB)",
            Self::XLarge => "XLarge (10-100GB)",
            Self::XXLarge => "XXLarge (100GB+)",
        }
    }
}

impl fmt::Display for ModelSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Complete AI/ML workload specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiMlWorkload {
    /// Framework being used
    pub framework: AiFramework,

    /// Type of operation
    pub operation: AiOperation,

    /// Model size classification
    pub model_size: ModelSize,

    /// Batch size for processing
    pub batch_size: usize,

    /// Optional: Specific model name/identifier
    pub model_name: Option<String>,

    /// Optional: Precision requirements (fp32, fp16, int8)
    pub precision: Option<Precision>,

    /// Optional: Minimum throughput requirement (samples/sec)
    pub min_throughput: Option<f64>,

    /// Optional: Maximum latency requirement (milliseconds)
    pub max_latency_ms: Option<u64>,
}

/// Numerical precision for computations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Precision {
    /// 32-bit floating point (standard)
    FP32,

    /// 16-bit floating point (mixed precision training)
    FP16,

    /// Brain floating point (ML-optimized)
    BF16,

    /// 8-bit integer (quantized inference)
    INT8,

    /// 4-bit integer (extreme quantization)
    INT4,
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FP32 => write!(f, "FP32"),
            Self::FP16 => write!(f, "FP16"),
            Self::BF16 => write!(f, "BF16"),
            Self::INT8 => write!(f, "INT8"),
            Self::INT4 => write!(f, "INT4"),
        }
    }
}

impl AiMlWorkload {
    /// Create new AI/ML workload specification
    #[must_use]
    pub const fn new(
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
            model_name: None,
            precision: None,
            min_throughput: None,
            max_latency_ms: None,
        }
    }

    /// Set model name
    #[must_use]
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Set precision requirement
    #[must_use]
    pub const fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = Some(precision);
        self
    }

    /// Set minimum throughput requirement
    #[must_use]
    pub const fn with_min_throughput(mut self, throughput: f64) -> Self {
        self.min_throughput = Some(throughput);
        self
    }

    /// Set maximum latency requirement
    #[must_use]
    pub const fn with_max_latency_ms(mut self, latency_ms: u64) -> Self {
        self.max_latency_ms = Some(latency_ms);
        self
    }

    /// Estimate total memory required (model + batch)
    #[must_use]
    pub fn estimate_total_memory_bytes(&self) -> u64 {
        let model_memory = self.model_size.estimate_memory_bytes();

        // Estimate batch memory (rough approximation)
        #[allow(clippy::cast_possible_truncation)]
        let batch_memory = match self.model_size {
            ModelSize::Small => self.batch_size as u64 * 1024 * 1024, // 1MB per sample
            ModelSize::Medium => self.batch_size as u64 * 10 * 1024 * 1024, // 10MB per sample
            ModelSize::Large => self.batch_size as u64 * 50 * 1024 * 1024, // 50MB per sample
            ModelSize::XLarge => self.batch_size as u64 * 100 * 1024 * 1024, // 100MB per sample
            ModelSize::XXLarge => self.batch_size as u64 * 500 * 1024 * 1024, // 500MB per sample
        };

        // Training needs additional memory for gradients and optimizer state
        let multiplier = match self.operation {
            AiOperation::Training => 3, // Model + gradients + optimizer state
            AiOperation::FineTuning => 3,
            AiOperation::Inference => 1,
            AiOperation::Evaluation => 1,
            AiOperation::Quantization => 2, // Need both full and quantized
        };

        model_memory
            .saturating_add(batch_memory)
            .saturating_mul(multiplier)
    }

    /// Check if workload is compute-intensive (likely needs GPU)
    #[must_use]
    pub const fn is_compute_intensive(&self) -> bool {
        matches!(
            (self.operation, self.model_size),
            (
                AiOperation::Training | AiOperation::FineTuning,
                ModelSize::Large | ModelSize::XLarge | ModelSize::XXLarge
            ) | (
                AiOperation::Inference,
                ModelSize::XLarge | ModelSize::XXLarge
            )
        )
    }

    /// Check if CPU execution is viable
    #[must_use]
    pub const fn is_cpu_viable(&self) -> bool {
        matches!(
            (self.operation, self.model_size, self.batch_size),
            (AiOperation::Inference, ModelSize::Small | ModelSize::Medium, bs) if bs <= 32
        ) || matches!(
            (self.operation, self.model_size),
            (
                AiOperation::Evaluation,
                ModelSize::Small | ModelSize::Medium
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size_memory_estimation() {
        assert_eq!(ModelSize::Small.estimate_memory_bytes(), 50 * 1024 * 1024);
        assert_eq!(
            ModelSize::XXLarge.estimate_memory_bytes(),
            200 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn test_workload_memory_estimation() {
        let workload = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Medium,
            32,
        );

        // Training should need 3x memory (model + gradients + optimizer)
        let estimated = workload.estimate_total_memory_bytes();
        assert!(estimated > ModelSize::Medium.estimate_memory_bytes());
    }

    #[test]
    fn test_compute_intensive_detection() {
        let intensive = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64,
        );
        assert!(intensive.is_compute_intensive());

        let light = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            1,
        );
        assert!(!light.is_compute_intensive());
    }

    #[test]
    fn test_cpu_viability() {
        let cpu_viable = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            16,
        );
        assert!(cpu_viable.is_cpu_viable());

        let gpu_needed = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::XLarge,
            128,
        );
        assert!(!gpu_needed.is_cpu_viable());
    }

    #[test]
    fn test_builder_pattern() {
        let workload = AiMlWorkload::new(
            AiFramework::Burn,
            AiOperation::Inference,
            ModelSize::Medium,
            32,
        )
        .with_model_name("resnet50")
        .with_precision(Precision::FP16)
        .with_max_latency_ms(100);

        assert_eq!(workload.model_name.as_deref(), Some("resnet50"));
        assert_eq!(workload.precision, Some(Precision::FP16));
        assert_eq!(workload.max_latency_ms, Some(100));
    }

    #[test]
    fn test_ai_framework_display() {
        assert_eq!(AiFramework::PyTorch.to_string(), "PyTorch");
        assert_eq!(AiFramework::TensorFlow.to_string(), "TensorFlow");
        assert_eq!(AiFramework::JAX.to_string(), "JAX");
        assert_eq!(AiFramework::ONNX.to_string(), "ONNX");
        assert_eq!(AiFramework::Burn.to_string(), "Burn");
        assert_eq!(AiFramework::Candle.to_string(), "Candle");
        assert_eq!(AiFramework::Custom.to_string(), "Custom");
    }

    #[test]
    fn test_ai_operation_display() {
        assert_eq!(AiOperation::Training.to_string(), "Training");
        assert_eq!(AiOperation::Inference.to_string(), "Inference");
        assert_eq!(AiOperation::FineTuning.to_string(), "Fine-tuning");
        assert_eq!(AiOperation::Evaluation.to_string(), "Evaluation");
        assert_eq!(AiOperation::Quantization.to_string(), "Quantization");
    }

    #[test]
    fn test_model_size_display_and_as_str() {
        assert_eq!(ModelSize::Small.as_str(), "Small (<100MB)");
        assert_eq!(ModelSize::Medium.as_str(), "Medium (100MB-1GB)");
        assert_eq!(ModelSize::Large.as_str(), "Large (1-10GB)");
        assert_eq!(ModelSize::XLarge.as_str(), "XLarge (10-100GB)");
        assert_eq!(ModelSize::XXLarge.as_str(), "XXLarge (100GB+)");
        assert_eq!(ModelSize::Small.to_string(), ModelSize::Small.as_str());
    }

    #[test]
    fn test_model_size_ordering() {
        assert!(ModelSize::Small < ModelSize::Medium);
        assert!(ModelSize::Medium < ModelSize::Large);
        assert!(ModelSize::Large < ModelSize::XLarge);
        assert!(ModelSize::XLarge < ModelSize::XXLarge);
    }

    #[test]
    fn test_model_size_memory_all_variants() {
        assert_eq!(ModelSize::Small.estimate_memory_bytes(), 50 * 1024 * 1024);
        assert_eq!(ModelSize::Medium.estimate_memory_bytes(), 500 * 1024 * 1024);
        assert_eq!(
            ModelSize::Large.estimate_memory_bytes(),
            5 * 1024 * 1024 * 1024
        );
        assert_eq!(
            ModelSize::XLarge.estimate_memory_bytes(),
            50 * 1024 * 1024 * 1024
        );
        assert_eq!(
            ModelSize::XXLarge.estimate_memory_bytes(),
            200 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn test_precision_display() {
        assert_eq!(Precision::FP32.to_string(), "FP32");
        assert_eq!(Precision::FP16.to_string(), "FP16");
        assert_eq!(Precision::BF16.to_string(), "BF16");
        assert_eq!(Precision::INT8.to_string(), "INT8");
        assert_eq!(Precision::INT4.to_string(), "INT4");
    }

    #[test]
    fn test_workload_constructor() {
        let w = AiMlWorkload::new(
            AiFramework::Candle,
            AiOperation::Inference,
            ModelSize::Small,
            1,
        );
        assert_eq!(w.framework, AiFramework::Candle);
        assert_eq!(w.operation, AiOperation::Inference);
        assert_eq!(w.model_size, ModelSize::Small);
        assert_eq!(w.batch_size, 1);
        assert!(w.model_name.is_none());
        assert!(w.precision.is_none());
        assert!(w.min_throughput.is_none());
        assert!(w.max_latency_ms.is_none());
    }

    #[test]
    fn test_workload_with_min_throughput() {
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Inference,
            ModelSize::Medium,
            16,
        )
        .with_min_throughput(1000.0);
        assert_eq!(w.min_throughput, Some(1000.0));
    }

    #[test]
    fn test_memory_estimation_inference_multiplier() {
        let inf = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            8,
        );
        let train = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Training,
            ModelSize::Small,
            8,
        );
        assert!(train.estimate_total_memory_bytes() > inf.estimate_total_memory_bytes());
    }

    #[test]
    fn test_compute_intensive_finetuning_xlarge() {
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::FineTuning,
            ModelSize::XLarge,
            4,
        );
        assert!(w.is_compute_intensive());
    }

    #[test]
    fn test_compute_intensive_inference_xxlarge() {
        let w = AiMlWorkload::new(
            AiFramework::JAX,
            AiOperation::Inference,
            ModelSize::XXLarge,
            1,
        );
        assert!(w.is_compute_intensive());
    }

    #[test]
    fn test_cpu_viable_evaluation_small() {
        let w = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Evaluation,
            ModelSize::Small,
            64,
        );
        assert!(w.is_cpu_viable());
    }

    #[test]
    fn test_cpu_viable_inference_small_batch_32() {
        let w = AiMlWorkload::new(
            AiFramework::Burn,
            AiOperation::Inference,
            ModelSize::Small,
            32,
        );
        assert!(w.is_cpu_viable());
    }

    #[test]
    fn test_cpu_not_viable_inference_small_batch_64() {
        let w = AiMlWorkload::new(
            AiFramework::Burn,
            AiOperation::Inference,
            ModelSize::Small,
            64,
        );
        assert!(!w.is_cpu_viable());
    }

    #[test]
    fn test_cpu_not_viable_training_large() {
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            8,
        );
        assert!(!w.is_cpu_viable());
    }

    #[test]
    fn test_workload_serialization_roundtrip() {
        let workload = AiMlWorkload::new(
            AiFramework::Burn,
            AiOperation::Inference,
            ModelSize::Medium,
            32,
        )
        .with_model_name("test-model")
        .with_precision(Precision::BF16)
        .with_min_throughput(500.0)
        .with_max_latency_ms(50);

        let json = serde_json::to_string(&workload).unwrap();
        let deserialized: AiMlWorkload = serde_json::from_str(&json).unwrap();

        assert_eq!(workload.framework, deserialized.framework);
        assert_eq!(workload.operation, deserialized.operation);
        assert_eq!(workload.model_size, deserialized.model_size);
        assert_eq!(workload.batch_size, deserialized.batch_size);
        assert_eq!(workload.model_name, deserialized.model_name);
        assert_eq!(workload.precision, deserialized.precision);
        assert_eq!(workload.min_throughput, deserialized.min_throughput);
        assert_eq!(workload.max_latency_ms, deserialized.max_latency_ms);
    }

    #[test]
    fn test_ai_framework_equality() {
        assert_eq!(AiFramework::PyTorch, AiFramework::PyTorch);
        assert_ne!(AiFramework::PyTorch, AiFramework::TensorFlow);
    }

    #[test]
    fn test_model_size_equality() {
        assert_eq!(ModelSize::Small, ModelSize::Small);
        assert_ne!(ModelSize::Small, ModelSize::Large);
    }
}
