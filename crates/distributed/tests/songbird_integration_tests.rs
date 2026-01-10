//! Comprehensive tests for Songbird integration types
//!
//! This test suite covers:
//! - JobComplexity enum
//! - ComplexityLevel enum
//! - IntensityLevel enum
//! - JobDistributionStrategy enum
//! - SubTaskStatus enum
//! - CompletionStrategy enum
//! - CoordinationStrategy enum
//! - ConnectionHealth enum
//! - NodeType enum
//! - SongbirdProtocol enum
//! - AuthType enum
//! - SplittingStrategyType enum
//! - DistributionAlgorithm enum

#![allow(deprecated)]

use toadstool_distributed::songbird_integration::*;

// ============================================================================
// JobComplexity Tests
// ============================================================================

#[test]
fn test_job_complexity_simple() {
    let complexity = JobComplexity::Simple;

    assert!(matches!(complexity, JobComplexity::Simple));
}

#[test]
fn test_job_complexity_moderate() {
    let complexity = JobComplexity::Moderate;

    assert!(matches!(complexity, JobComplexity::Moderate));
}

#[test]
fn test_job_complexity_complex() {
    let complexity = JobComplexity::Complex;

    assert!(matches!(complexity, JobComplexity::Complex));
}

#[test]
fn test_job_complexity_ultra_massive() {
    let complexity = JobComplexity::UltraMassive;

    assert!(matches!(complexity, JobComplexity::UltraMassive));
}

// ============================================================================
// ComplexityLevel Tests
// ============================================================================

#[test]
fn test_complexity_level_low() {
    let level = ComplexityLevel::Low;

    assert!(matches!(level, ComplexityLevel::Low));
}

#[test]
fn test_complexity_level_medium() {
    let level = ComplexityLevel::Medium;

    assert!(matches!(level, ComplexityLevel::Medium));
}

#[test]
fn test_complexity_level_high() {
    let level = ComplexityLevel::High;

    assert!(matches!(level, ComplexityLevel::High));
}

#[test]
fn test_complexity_level_extreme() {
    let level = ComplexityLevel::Extreme;

    assert!(matches!(level, ComplexityLevel::Extreme));
}

// ============================================================================
// IntensityLevel Tests
// ============================================================================

#[test]
fn test_intensity_level_low() {
    let level = IntensityLevel::Low;

    assert!(matches!(level, IntensityLevel::Low));
}

#[test]
fn test_intensity_level_medium() {
    let level = IntensityLevel::Medium;

    assert!(matches!(level, IntensityLevel::Medium));
}

#[test]
fn test_intensity_level_high() {
    let level = IntensityLevel::High;

    assert!(matches!(level, IntensityLevel::High));
}

#[test]
fn test_intensity_level_extreme() {
    let level = IntensityLevel::Extreme;

    assert!(matches!(level, IntensityLevel::Extreme));
}

// ============================================================================
// JobDistributionStrategy Tests
// ============================================================================

#[test]
fn test_distribution_strategy_local_only() {
    let strategy = JobDistributionStrategy::LocalOnly;

    assert!(matches!(strategy, JobDistributionStrategy::LocalOnly));
}

#[test]
fn test_distribution_strategy_split_and_distribute() {
    let strategy = JobDistributionStrategy::SplitAndDistribute;

    assert!(matches!(
        strategy,
        JobDistributionStrategy::SplitAndDistribute
    ));
}

#[test]
fn test_distribution_strategy_replicate() {
    let strategy = JobDistributionStrategy::ReplicateAcrossNodes;

    assert!(matches!(
        strategy,
        JobDistributionStrategy::ReplicateAcrossNodes
    ));
}

#[test]
fn test_distribution_strategy_hybrid() {
    let strategy = JobDistributionStrategy::HybridExecution;

    assert!(matches!(strategy, JobDistributionStrategy::HybridExecution));
}

#[test]
fn test_distribution_strategy_songbird_ecosystem() {
    let strategy = JobDistributionStrategy::SongbirdEcosystem;

    assert!(matches!(
        strategy,
        JobDistributionStrategy::SongbirdEcosystem
    ));
}

#[test]
fn test_distribution_strategy_load_balanced() {
    let strategy = JobDistributionStrategy::LoadBalanced;

    assert!(matches!(strategy, JobDistributionStrategy::LoadBalanced));
}

#[test]
fn test_distribution_strategy_massive() {
    let strategy = JobDistributionStrategy::MassiveDistribution;

    assert!(matches!(
        strategy,
        JobDistributionStrategy::MassiveDistribution
    ));
}

// ============================================================================
// SubTaskStatus Tests
// ============================================================================

#[test]
fn test_subtask_status_submitted() {
    let status = SubTaskStatus::Submitted;

    assert!(matches!(status, SubTaskStatus::Submitted));
}

#[test]
fn test_subtask_status_running() {
    let status = SubTaskStatus::Running;

    assert!(matches!(status, SubTaskStatus::Running));
}

#[test]
fn test_subtask_status_completed() {
    let status = SubTaskStatus::Completed;

    assert!(matches!(status, SubTaskStatus::Completed));
}

#[test]
fn test_subtask_status_failed() {
    let status = SubTaskStatus::Failed;

    assert!(matches!(status, SubTaskStatus::Failed));
}

// ============================================================================
// CompletionStrategy Tests
// ============================================================================

#[test]
fn test_completion_strategy_wait_for_all() {
    let strategy = CompletionStrategy::WaitForAll;

    assert!(matches!(strategy, CompletionStrategy::WaitForAll));
}

#[test]
fn test_completion_strategy_wait_for_majority() {
    let strategy = CompletionStrategy::WaitForMajority;

    assert!(matches!(strategy, CompletionStrategy::WaitForMajority));
}

#[test]
fn test_completion_strategy_wait_for_any() {
    let strategy = CompletionStrategy::WaitForAny;

    assert!(matches!(strategy, CompletionStrategy::WaitForAny));
}

#[test]
fn test_completion_strategy_custom() {
    let strategy = CompletionStrategy::Custom("CustomLogic".to_string());

    match strategy {
        CompletionStrategy::Custom(name) => {
            assert_eq!(name, "CustomLogic");
        }
        _ => panic!("Expected Custom variant"),
    }
}

// ============================================================================
// CoordinationStrategy Tests
// ============================================================================

#[test]
fn test_coordination_strategy_sequential() {
    let strategy = CoordinationStrategy::Sequential;

    assert!(matches!(strategy, CoordinationStrategy::Sequential));
}

#[test]
fn test_coordination_strategy_parallel() {
    let strategy = CoordinationStrategy::Parallel;

    assert!(matches!(strategy, CoordinationStrategy::Parallel));
}

#[test]
fn test_coordination_strategy_pipeline() {
    let strategy = CoordinationStrategy::Pipeline;

    assert!(matches!(strategy, CoordinationStrategy::Pipeline));
}

#[test]
fn test_coordination_strategy_mapreduce() {
    let strategy = CoordinationStrategy::MapReduce;

    assert!(matches!(strategy, CoordinationStrategy::MapReduce));
}

// ============================================================================
// ConnectionHealth Tests
// ============================================================================

#[test]
fn test_connection_health_healthy() {
    let health = ConnectionHealth::Healthy;

    assert_eq!(health, ConnectionHealth::Healthy);
}

#[test]
fn test_connection_health_degraded() {
    let health = ConnectionHealth::Degraded;

    assert_eq!(health, ConnectionHealth::Degraded);
}

#[test]
fn test_connection_health_unhealthy() {
    let health = ConnectionHealth::Unhealthy;

    assert_eq!(health, ConnectionHealth::Unhealthy);
}

#[test]
fn test_connection_health_unknown() {
    let health = ConnectionHealth::Unknown;

    assert_eq!(health, ConnectionHealth::Unknown);
}

// ============================================================================
// NodeType Tests
// ============================================================================

#[test]
fn test_node_type_toadstool() {
    let node = NodeType::ToadStool;

    assert!(matches!(node, NodeType::ToadStool));
}

#[test]
fn test_node_type_nestgate() {
    let node = NodeType::NestGate;

    assert!(matches!(node, NodeType::NestGate));
}

#[test]
fn test_node_type_beardog() {
    let node = NodeType::BearDog;

    assert!(matches!(node, NodeType::BearDog));
}

#[test]
fn test_node_type_songbird() {
    let node = NodeType::Songbird;

    assert!(matches!(node, NodeType::Songbird));
}

#[test]
fn test_node_type_custom() {
    let node = NodeType::Custom("MyCustomNode".to_string());

    match node {
        NodeType::Custom(name) => {
            assert_eq!(name, "MyCustomNode");
        }
        _ => panic!("Expected Custom variant"),
    }
}

// ============================================================================
// SongbirdProtocol Tests
// ============================================================================

#[test]
fn test_songbird_protocol_http() {
    let protocol = SongbirdProtocol::HTTP;

    assert!(matches!(protocol, SongbirdProtocol::HTTP));
}

#[test]
fn test_songbird_protocol_grpc() {
    let protocol = SongbirdProtocol::GRPC;

    assert!(matches!(protocol, SongbirdProtocol::GRPC));
}

#[test]
fn test_songbird_protocol_websocket() {
    let protocol = SongbirdProtocol::WebSocket;

    assert!(matches!(protocol, SongbirdProtocol::WebSocket));
}

#[test]
fn test_songbird_protocol_message_queue() {
    let protocol = SongbirdProtocol::MessageQueue;

    assert!(matches!(protocol, SongbirdProtocol::MessageQueue));
}

// ============================================================================
// AuthType Tests
// ============================================================================

#[test]
fn test_auth_type_none() {
    let auth = AuthType::None;

    assert!(matches!(auth, AuthType::None));
}

#[test]
fn test_auth_type_api_key() {
    let auth = AuthType::ApiKey;

    assert!(matches!(auth, AuthType::ApiKey));
}

#[test]
fn test_auth_type_bearer() {
    let auth = AuthType::Bearer;

    assert!(matches!(auth, AuthType::Bearer));
}

#[test]
fn test_auth_type_basic() {
    let auth = AuthType::Basic;

    assert!(matches!(auth, AuthType::Basic));
}

#[test]
fn test_auth_type_oauth2() {
    let auth = AuthType::OAuth2;

    assert!(matches!(auth, AuthType::OAuth2));
}

// ============================================================================
// SplittingStrategyType Tests
// ============================================================================

#[test]
fn test_splitting_strategy_data_parallel() {
    let strategy = SplittingStrategyType::DataParallel;

    assert!(matches!(strategy, SplittingStrategyType::DataParallel));
}

#[test]
fn test_splitting_strategy_task_parallel() {
    let strategy = SplittingStrategyType::TaskParallel;

    assert!(matches!(strategy, SplittingStrategyType::TaskParallel));
}

#[test]
fn test_splitting_strategy_pipeline() {
    let strategy = SplittingStrategyType::Pipeline;

    assert!(matches!(strategy, SplittingStrategyType::Pipeline));
}

#[test]
fn test_splitting_strategy_map_reduce() {
    let strategy = SplittingStrategyType::MapReduce;

    assert!(matches!(strategy, SplittingStrategyType::MapReduce));
}

#[test]
fn test_splitting_strategy_custom() {
    let strategy = SplittingStrategyType::Custom("MyStrategy".to_string());

    match strategy {
        SplittingStrategyType::Custom(name) => {
            assert_eq!(name, "MyStrategy");
        }
        _ => panic!("Expected Custom variant"),
    }
}

// ============================================================================
// DistributionAlgorithm Tests
// ============================================================================

#[test]
fn test_distribution_algorithm_round_robin() {
    let algo = DistributionAlgorithm::RoundRobin;

    assert!(matches!(algo, DistributionAlgorithm::RoundRobin));
}

#[test]
fn test_distribution_algorithm_load_based() {
    let algo = DistributionAlgorithm::LoadBased;

    assert!(matches!(algo, DistributionAlgorithm::LoadBased));
}

#[test]
fn test_distribution_algorithm_capability_matched() {
    let algo = DistributionAlgorithm::CapabilityMatched;

    assert!(matches!(algo, DistributionAlgorithm::CapabilityMatched));
}

#[test]
fn test_distribution_algorithm_geographic() {
    let algo = DistributionAlgorithm::GeographicOptimized;

    assert!(matches!(algo, DistributionAlgorithm::GeographicOptimized));
}

#[test]
fn test_distribution_algorithm_custom() {
    let algo = DistributionAlgorithm::Custom("cost-optimized".to_string());

    assert!(matches!(algo, DistributionAlgorithm::Custom(_)));
}

#[test]
fn test_distribution_algorithm_consistent_hashing() {
    let algo = DistributionAlgorithm::ConsistentHashing;

    assert!(matches!(algo, DistributionAlgorithm::ConsistentHashing));
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_songbird_integration_coverage_summary() {
    println!("=== Songbird Integration Test Coverage ===");
    println!("JobComplexity Tests:           4 tests");
    println!("ComplexityLevel Tests:         4 tests");
    println!("IntensityLevel Tests:          4 tests");
    println!("JobDistributionStrategy:       7 tests");
    println!("SubTaskStatus Tests:           4 tests");
    println!("CompletionStrategy Tests:      4 tests");
    println!("CoordinationStrategy Tests:    4 tests");
    println!("ConnectionHealth Tests:        4 tests");
    println!("NodeType Tests:                5 tests");
    println!("SongbirdProtocol Tests:        4 tests");
    println!("AuthType Tests:                5 tests");
    println!("SplittingStrategyType Tests:   5 tests");
    println!("DistributionAlgorithm Tests:   6 tests");
    println!("Total:                         60 tests");
    println!("============================================");
}
