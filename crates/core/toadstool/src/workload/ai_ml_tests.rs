// SPDX-License-Identifier: AGPL-3.0-or-later

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
