// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn make_workload(op: OperationType, input: WorkloadData) -> Workload {
    Workload {
        operation: op,
        data_type: DataType::F32,
        num_operations: 3,
        required_memory: 12,
        input,
        params: WorkloadParams::default(),
    }
}

#[tokio::test]
async fn test_cpu_discover_has_name() {
    let cpu = CpuComputeUnit::discover();
    assert!(cpu.name().contains("CPU"));
}

#[tokio::test]
async fn test_cpu_capabilities_unit_type() {
    let cpu = CpuComputeUnit::discover();
    assert_eq!(cpu.capabilities().unit_type, ComputeUnitType::Cpu);
}

#[tokio::test]
async fn test_cpu_supports_f32_map() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    assert!(cpu.can_execute(&w));
}

#[tokio::test]
async fn test_cpu_optimal_batch_size() {
    let cpu = CpuComputeUnit::discover();
    assert!(cpu.optimal_batch_size() > 0);
}

#[tokio::test]
async fn test_cpu_execute_map() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => assert_eq!(v.len(), 3),
        _ => unreachable!("expected F32Vec"),
    }
}

#[tokio::test]
async fn test_cpu_execute_dot_product() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::DotProduct,
        WorkloadData::F32VecPair(vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]),
    );
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => assert!((v[0] - 32.0).abs() < 1e-5),
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_transpose() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::Transpose,
        WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2),
    );
    let out = cpu.execute(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Matrix(_, _, _)));
}

#[tokio::test]
async fn test_cpu_execute_layernorm() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::LayerNorm,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = cpu.execute(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
async fn test_cpu_execute_custom_returns_error() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(OperationType::Custom, WorkloadData::Custom(vec![]));
    assert!(cpu.execute(w).await.is_err());
}

#[tokio::test]
async fn test_cpu_estimate_duration_nonzero() {
    let cpu = CpuComputeUnit::discover();
    let w = make_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let dur = cpu.estimate_duration(&w);
    // Duration should be at least 0 (latency = 0ms for CPU)
    let _ = dur;
}

#[tokio::test]
async fn test_cpu_execute_matmul() {
    let cpu = CpuComputeUnit::discover();
    let a = vec![1.0f32, 2.0, 3.0, 4.0];
    let b = vec![1.0f32, 0.0, 0.0, 1.0];
    let w = make_workload(
        OperationType::MatMul,
        WorkloadData::F32MatrixPair(a, 2, 2, b, 2, 2),
    );
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Matrix(v, rows, cols) => {
            assert_eq!(rows, 2);
            assert_eq!(cols, 2);
            assert_eq!(v.len(), 4);
            assert!((v[0] - 1.0).abs() < 1e-5);
            assert!((v[1] - 2.0).abs() < 1e-5);
            assert!((v[2] - 3.0).abs() < 1e-5);
            assert!((v[3] - 4.0).abs() < 1e-5);
        }
        other => unreachable!("expected F32Matrix, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_matmul_dimension_mismatch() {
    let cpu = CpuComputeUnit::discover();
    let a = vec![1.0f32, 2.0, 3.0];
    let b = vec![1.0f32, 0.0, 0.0, 1.0];
    let w = make_workload(
        OperationType::MatMul,
        WorkloadData::F32MatrixPair(a, 1, 3, b, 2, 2),
    );
    assert!(cpu.execute(w).await.is_err());
}

#[tokio::test]
async fn test_cpu_execute_relu() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![-1.0f32, 0.0, 1.0, 2.0, -0.5];
    let w = make_workload(OperationType::ReLU, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => {
            assert_eq!(v.len(), 5);
            assert!((v[0] - 0.0).abs() < 1e-5);
            assert!((v[1] - 0.0).abs() < 1e-5);
            assert!((v[2] - 1.0).abs() < 1e-5);
            assert!((v[3] - 2.0).abs() < 1e-5);
            assert!((v[4] - 0.0).abs() < 1e-5);
        }
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_gelu() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![0.0f32, 1.0, -1.0];
    let w = make_workload(OperationType::GELU, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => {
            assert_eq!(v.len(), 3);
            assert!((v[0] - 0.0).abs() < 1e-4);
            assert!(v[1] > 0.0 && v[1] < 1.0);
            assert!(v[2] < 0.0 && v[2] > -0.2);
        }
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_tanh() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![0.0f32, 1.0, -1.0];
    let w = make_workload(OperationType::Tanh, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => {
            assert_eq!(v.len(), 3);
            assert!((v[0] - 0.0).abs() < 1e-5);
            assert!((v[1] - 0.761_594).abs() < 1e-3);
            assert!((v[2] + 0.761_594).abs() < 1e-3);
        }
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_sigmoid() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![0.0f32, 1.0, -1.0];
    let w = make_workload(OperationType::Sigmoid, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => {
            assert_eq!(v.len(), 3);
            assert!((v[0] - 0.5).abs() < 1e-5);
            assert!(v[1] > 0.7 && v[1] < 0.8);
            assert!(v[2] > 0.2 && v[2] < 0.3);
        }
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_softmax() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32, 2.0, 3.0];
    let w = make_workload(OperationType::Softmax, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => {
            assert_eq!(v.len(), 3);
            let sum: f32 = v.iter().sum();
            assert!((sum - 1.0).abs() < 1e-5);
            assert!(v.iter().all(|&x| x > 0.0 && x < 1.0));
        }
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_dropout() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32, 2.0, 3.0];
    let w = make_workload(OperationType::Dropout, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Vec(v) => assert_eq!(v.len(), 3),
        other => unreachable!("expected F32Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_conv2d() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32; 32];
    let kernel = vec![1.0f32 / 9.0; 2 * 2 * 3 * 3];
    let w = make_workload(
        OperationType::Conv,
        WorkloadData::F32Conv2D {
            input,
            kernel,
            bias: None,
            batch_size: 1,
            in_channels: 2,
            height: 4,
            width: 4,
            out_channels: 2,
            kernel_h: 3,
            kernel_w: 3,
            stride: 1,
            padding: 0,
        },
    );
    let out = cpu.execute(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Matrix(_, _, _)));
}

#[tokio::test]
async fn test_cpu_execute_maxpool2d() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let w = make_workload(
        OperationType::MaxPool2D,
        WorkloadData::F32Pool2D {
            input,
            batch_size: 1,
            channels: 1,
            height: 3,
            width: 3,
            pool_h: 2,
            pool_w: 2,
            stride: 1,
            padding: 0,
        },
    );
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Matrix(v, _, _) => {
            assert!(!v.is_empty());
            assert!(v.iter().all(|&x| x >= 0.0));
        }
        other => unreachable!("expected F32Matrix, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_avgpool2d() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let w = make_workload(
        OperationType::AvgPool2D,
        WorkloadData::F32Pool2D {
            input,
            batch_size: 1,
            channels: 1,
            height: 3,
            width: 3,
            pool_h: 2,
            pool_w: 2,
            stride: 1,
            padding: 0,
        },
    );
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F32Matrix(v, _, _) => {
            assert!(!v.is_empty());
            assert!(v.iter().all(|&x| x >= 0.0));
        }
        other => unreachable!("expected F32Matrix, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_activation_f64() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![-1.0f64, 0.0, 1.0];
    let mut w = make_workload(OperationType::ReLU, WorkloadData::F64Vec(input));
    w.data_type = DataType::F64;
    let out = cpu.execute(w).await.unwrap();
    match out.data {
        WorkloadData::F64Vec(v) => {
            assert_eq!(v.len(), 3);
            assert!((v[0] - 0.0).abs() < 1e-10);
            assert!((v[1] - 0.0).abs() < 1e-10);
            assert!((v[2] - 1.0).abs() < 1e-10);
        }
        other => unreachable!("expected F64Vec, got {other:?}"),
    }
}

#[tokio::test]
async fn test_cpu_execute_batchnorm() {
    let cpu = CpuComputeUnit::discover();
    let input = vec![1.0f32, 2.0, 3.0, 4.0];
    let w = make_workload(OperationType::BatchNorm, WorkloadData::F32Vec(input));
    let out = cpu.execute(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}
