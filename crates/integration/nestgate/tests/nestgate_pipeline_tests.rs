//! Comprehensive tests for NestGate pipeline types

use std::collections::HashMap;
use toadstool_integration_nestgate::*;

// ============================================================================
// InputType Tests
// ============================================================================

#[test]
fn test_input_type_filesystem() {
    let input = InputType::FileSystem {
        path: "/data/input".to_string(),
    };

    match input {
        InputType::FileSystem { path } => assert_eq!(path, "/data/input"),
        _ => panic!("Expected FileSystem variant"),
    }
}

#[test]
fn test_input_type_http() {
    let input = InputType::Http {
        url: "https://api.example.com/data".to_string(),
    };

    match input {
        InputType::Http { url } => assert!(url.starts_with("https://")),
        _ => panic!("Expected Http variant"),
    }
}

#[test]
fn test_input_type_database() {
    let input = InputType::Database {
        connection: "postgresql://localhost/db".to_string(),
        query: "SELECT * FROM users".to_string(),
    };

    match input {
        InputType::Database { connection, query } => {
            assert!(connection.starts_with("postgresql://"));
            assert!(query.contains("SELECT"));
        }
        _ => panic!("Expected Database variant"),
    }
}

#[test]
fn test_input_type_stream() {
    let input = InputType::Stream {
        topic: "events".to_string(),
    };

    match input {
        InputType::Stream { topic } => assert_eq!(topic, "events"),
        _ => panic!("Expected Stream variant"),
    }
}

#[test]
fn test_input_type_artifact() {
    let input = InputType::Artifact {
        artifact_id: "artifact-123".to_string(),
    };

    match input {
        InputType::Artifact { artifact_id } => assert_eq!(artifact_id, "artifact-123"),
        _ => panic!("Expected Artifact variant"),
    }
}

#[test]
fn test_input_type_serialization() {
    let input = InputType::FileSystem {
        path: "/test".to_string(),
    };

    let json = serde_json::to_string(&input).unwrap();
    let deserialized: InputType = serde_json::from_str(&json).unwrap();

    match deserialized {
        InputType::FileSystem { path } => assert_eq!(path, "/test"),
        _ => panic!("Deserialization failed"),
    }
}

// ============================================================================
// OutputType Tests
// ============================================================================

#[test]
fn test_output_type_filesystem() {
    let output = OutputType::FileSystem {
        path: "/data/output".to_string(),
    };

    match output {
        OutputType::FileSystem { path } => assert_eq!(path, "/data/output"),
        _ => panic!("Expected FileSystem variant"),
    }
}

#[test]
fn test_output_type_http() {
    let output = OutputType::Http {
        url: "https://webhook.example.com".to_string(),
    };

    match output {
        OutputType::Http { url } => assert!(url.starts_with("https://")),
        _ => panic!("Expected Http variant"),
    }
}

#[test]
fn test_output_type_database() {
    let output = OutputType::Database {
        connection: "postgresql://localhost/db".to_string(),
        table: "results".to_string(),
    };

    match output {
        OutputType::Database { connection, table } => {
            assert!(connection.starts_with("postgresql://"));
            assert_eq!(table, "results");
        }
        _ => panic!("Expected Database variant"),
    }
}

#[test]
fn test_output_type_stream() {
    let output = OutputType::Stream {
        topic: "results".to_string(),
    };

    match output {
        OutputType::Stream { topic } => assert_eq!(topic, "results"),
        _ => panic!("Expected Stream variant"),
    }
}

#[test]
fn test_output_type_artifact() {
    let output = OutputType::Artifact {
        artifact_type: ArtifactType::ExecutionOutput,
    };

    match output {
        OutputType::Artifact { artifact_type } => {
            assert!(matches!(artifact_type, ArtifactType::ExecutionOutput))
        }
        _ => panic!("Expected Artifact variant"),
    }
}

// ============================================================================
// PipelineInput Tests
// ============================================================================

#[test]
fn test_pipeline_input_creation() {
    let input = PipelineInput {
        id: "input-1".to_string(),
        input_type: InputType::FileSystem {
            path: "/data".to_string(),
        },
        config: HashMap::new(),
    };

    assert_eq!(input.id, "input-1");
}

#[test]
fn test_pipeline_input_with_config() {
    let mut config = HashMap::new();
    config.insert("format".to_string(), serde_json::json!("json"));

    let input = PipelineInput {
        id: "input-2".to_string(),
        input_type: InputType::Http {
            url: "https://api.example.com".to_string(),
        },
        config,
    };

    assert_eq!(input.config.len(), 1);
    assert_eq!(input.config.get("format").unwrap(), "json");
}

#[test]
fn test_pipeline_input_serialization() {
    let input = PipelineInput {
        id: "test".to_string(),
        input_type: InputType::Stream {
            topic: "events".to_string(),
        },
        config: HashMap::new(),
    };

    let json = serde_json::to_string(&input).unwrap();
    let deserialized: PipelineInput = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test");
}

// ============================================================================
// PipelineOutput Tests
// ============================================================================

#[test]
fn test_pipeline_output_creation() {
    let output = PipelineOutput {
        id: "output-1".to_string(),
        output_type: OutputType::FileSystem {
            path: "/results".to_string(),
        },
        config: HashMap::new(),
    };

    assert_eq!(output.id, "output-1");
}

#[test]
fn test_pipeline_output_with_config() {
    let mut config = HashMap::new();
    config.insert("compression".to_string(), serde_json::json!("gzip"));

    let output = PipelineOutput {
        id: "output-2".to_string(),
        output_type: OutputType::Artifact {
            artifact_type: ArtifactType::Model,
        },
        config,
    };

    assert_eq!(output.config.len(), 1);
}

// ============================================================================
// StepType Tests
// ============================================================================

#[test]
fn test_step_type_transform() {
    let step = StepType::Transform {
        script: "data.map(x => x * 2)".to_string(),
        language: "javascript".to_string(),
    };

    match step {
        StepType::Transform { script, language } => {
            assert!(script.contains("map"));
            assert_eq!(language, "javascript");
        }
        _ => panic!("Expected Transform variant"),
    }
}

#[test]
fn test_step_type_filter() {
    let step = StepType::Filter {
        condition: "value > 100".to_string(),
    };

    match step {
        StepType::Filter { condition } => assert!(condition.contains(">")),
        _ => panic!("Expected Filter variant"),
    }
}

#[test]
fn test_step_type_aggregate() {
    let step = StepType::Aggregate {
        fields: vec!["count".to_string(), "sum".to_string()],
        operation: "sum".to_string(),
    };

    match step {
        StepType::Aggregate { fields, operation } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(operation, "sum");
        }
        _ => panic!("Expected Aggregate variant"),
    }
}

#[test]
fn test_step_type_toadstool() {
    let step = StepType::ToadStool {
        workload: "python script.py".to_string(),
        runtime: Some("python".to_string()),
    };

    match step {
        StepType::ToadStool { workload, runtime } => {
            assert!(workload.contains("python"));
            assert!(runtime.is_some());
        }
        _ => panic!("Expected ToadStool variant"),
    }
}

#[test]
fn test_step_type_custom() {
    let step = StepType::Custom {
        processor: "custom-processor".to_string(),
    };

    match step {
        StepType::Custom { processor } => assert_eq!(processor, "custom-processor"),
        _ => panic!("Expected Custom variant"),
    }
}

// ============================================================================
// PipelineStep Tests
// ============================================================================

#[test]
fn test_pipeline_step_basic() {
    let step = PipelineStep {
        id: "step-1".to_string(),
        name: "Transform Data".to_string(),
        step_type: StepType::Transform {
            script: "x => x".to_string(),
            language: "javascript".to_string(),
        },
        depends_on: vec![],
        config: HashMap::new(),
    };

    assert_eq!(step.id, "step-1");
    assert!(step.depends_on.is_empty());
}

#[test]
fn test_pipeline_step_with_dependencies() {
    let step = PipelineStep {
        id: "step-2".to_string(),
        name: "Aggregate Results".to_string(),
        step_type: StepType::Aggregate {
            fields: vec!["total".to_string()],
            operation: "sum".to_string(),
        },
        depends_on: vec!["step-1".to_string()],
        config: HashMap::new(),
    };

    assert_eq!(step.depends_on.len(), 1);
    assert_eq!(step.depends_on[0], "step-1");
}

#[test]
fn test_pipeline_step_serialization() {
    let step = PipelineStep {
        id: "test".to_string(),
        name: "Test Step".to_string(),
        step_type: StepType::Filter {
            condition: "true".to_string(),
        },
        depends_on: vec![],
        config: HashMap::new(),
    };

    let json = serde_json::to_string(&step).unwrap();
    let deserialized: PipelineStep = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test");
}

// ============================================================================
// ScheduleType Tests
// ============================================================================

#[test]
fn test_schedule_type_once() {
    let schedule = ScheduleType::Once;
    assert!(matches!(schedule, ScheduleType::Once));
}

#[test]
fn test_schedule_type_cron() {
    let schedule = ScheduleType::Cron {
        expression: "0 0 * * *".to_string(),
    };

    match schedule {
        ScheduleType::Cron { expression } => assert!(expression.starts_with("0 0")),
        _ => panic!("Expected Cron variant"),
    }
}

#[test]
fn test_schedule_type_interval() {
    use std::time::Duration;
    let schedule = ScheduleType::Interval {
        duration: Duration::from_secs(3600), // 1 hour
    };

    match schedule {
        ScheduleType::Interval { duration } => assert_eq!(duration.as_secs(), 3600),
        _ => panic!("Expected Interval variant"),
    }
}

#[test]
fn test_schedule_type_event() {
    let schedule = ScheduleType::Event {
        trigger: "artifact_created".to_string(),
    };

    match schedule {
        ScheduleType::Event { trigger } => assert_eq!(trigger, "artifact_created"),
        _ => panic!("Expected Event variant"),
    }
}

// ============================================================================
// PipelineSchedule Tests
// ============================================================================

#[test]
fn test_pipeline_schedule_basic() {
    let schedule = PipelineSchedule {
        schedule_type: ScheduleType::Once,
        timezone: None,
        max_concurrent: None,
    };

    assert!(matches!(schedule.schedule_type, ScheduleType::Once));
}

#[test]
fn test_pipeline_schedule_with_timezone() {
    let schedule = PipelineSchedule {
        schedule_type: ScheduleType::Cron {
            expression: "0 0 * * *".to_string(),
        },
        timezone: Some("America/New_York".to_string()),
        max_concurrent: Some(1),
    };

    assert_eq!(schedule.timezone.unwrap(), "America/New_York");
    assert_eq!(schedule.max_concurrent.unwrap(), 1);
}

#[test]
fn test_pipeline_schedule_concurrent_limit() {
    use std::time::Duration;
    let schedule = PipelineSchedule {
        schedule_type: ScheduleType::Interval {
            duration: Duration::from_secs(60),
        },
        timezone: None,
        max_concurrent: Some(5),
    };

    assert_eq!(schedule.max_concurrent.unwrap(), 5);
}

// ============================================================================
// PipelineResources Tests
// ============================================================================

#[test]
fn test_pipeline_resources_basic() {
    let resources = PipelineResources {
        cpu_cores: Some(2.0),
        memory_bytes: Some(4096 * 1024 * 1024),  // 4 GB
        storage_bytes: Some(1024 * 1024 * 1024), // 1 GB
        network_bandwidth: Some(1_000_000),      // 1 Mbps
    };

    assert_eq!(resources.cpu_cores.unwrap(), 2.0);
    assert_eq!(resources.memory_bytes.unwrap(), 4096 * 1024 * 1024);
}

#[test]
fn test_pipeline_resources_minimal() {
    let resources = PipelineResources {
        cpu_cores: None,
        memory_bytes: None,
        storage_bytes: Some(512 * 1024 * 1024), // 512 MB
        network_bandwidth: None,
    };

    assert!(resources.cpu_cores.is_none());
    assert_eq!(resources.storage_bytes.unwrap(), 512 * 1024 * 1024);
}

// ============================================================================
// PipelineExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_pending() {
    let status = PipelineExecutionStatus::Pending;
    assert!(matches!(status, PipelineExecutionStatus::Pending));
}

#[test]
fn test_execution_status_running() {
    let status = PipelineExecutionStatus::Running;
    assert!(matches!(status, PipelineExecutionStatus::Running));
}

#[test]
fn test_execution_status_completed() {
    let status = PipelineExecutionStatus::Completed;
    assert!(matches!(status, PipelineExecutionStatus::Completed));
}

#[test]
fn test_execution_status_failed() {
    let status = PipelineExecutionStatus::Failed;
    assert!(matches!(status, PipelineExecutionStatus::Failed));
}

#[test]
fn test_execution_status_cancelled() {
    let status = PipelineExecutionStatus::Cancelled;
    assert!(matches!(status, PipelineExecutionStatus::Cancelled));
}

// ============================================================================
// StepExecutionStatus Tests
// ============================================================================

#[test]
fn test_step_status_pending() {
    let status = StepExecutionStatus::Pending;
    assert!(matches!(status, StepExecutionStatus::Pending));
}

#[test]
fn test_step_status_running() {
    let status = StepExecutionStatus::Running;
    assert!(matches!(status, StepExecutionStatus::Running));
}

#[test]
fn test_step_status_completed() {
    let status = StepExecutionStatus::Completed;
    assert!(matches!(status, StepExecutionStatus::Completed));
}

#[test]
fn test_step_status_failed() {
    let status = StepExecutionStatus::Failed;
    assert!(matches!(status, StepExecutionStatus::Failed));
}

#[test]
fn test_step_status_skipped() {
    let status = StepExecutionStatus::Skipped;
    assert!(matches!(status, StepExecutionStatus::Skipped));
}

// ============================================================================
// Integration Scenarios
// ============================================================================

#[test]
fn test_scenario_etl_pipeline() {
    let pipeline = PipelineConfig {
        pipeline_id: "etl-001".to_string(),
        name: "ETL Pipeline".to_string(),
        inputs: vec![PipelineInput {
            id: "source".to_string(),
            input_type: InputType::Database {
                connection: "postgresql://db/analytics".to_string(),
                query: "SELECT * FROM raw_data".to_string(),
            },
            config: HashMap::new(),
        }],
        outputs: vec![PipelineOutput {
            id: "destination".to_string(),
            output_type: OutputType::Artifact {
                artifact_type: ArtifactType::DataFile,
            },
            config: HashMap::new(),
        }],
        steps: vec![
            PipelineStep {
                id: "transform".to_string(),
                name: "Transform Data".to_string(),
                step_type: StepType::Transform {
                    script: "clean(data)".to_string(),
                    language: "python".to_string(),
                },
                depends_on: vec![],
                config: HashMap::new(),
            },
            PipelineStep {
                id: "aggregate".to_string(),
                name: "Aggregate".to_string(),
                step_type: StepType::Aggregate {
                    fields: vec!["count".to_string()],
                    operation: "sum".to_string(),
                },
                depends_on: vec!["transform".to_string()],
                config: HashMap::new(),
            },
        ],
        schedule: Some(PipelineSchedule {
            schedule_type: ScheduleType::Cron {
                expression: "0 0 * * *".to_string(),
            },
            timezone: Some("UTC".to_string()),
            max_concurrent: Some(1),
        }),
        resources: Some(PipelineResources {
            cpu_cores: Some(4.0),
            memory_bytes: Some(8192 * 1024 * 1024), // 8 GB
            storage_bytes: Some(10 * 1024 * 1024 * 1024), // 10 GB
            network_bandwidth: Some(10_000_000),    // 10 Mbps
        }),
    };

    assert_eq!(pipeline.inputs.len(), 1);
    assert_eq!(pipeline.outputs.len(), 1);
    assert_eq!(pipeline.steps.len(), 2);
    assert!(pipeline.schedule.is_some());
    assert!(pipeline.resources.is_some());
}

#[test]
fn test_scenario_ml_training_pipeline() {
    let pipeline = PipelineConfig {
        pipeline_id: "ml-train-001".to_string(),
        name: "ML Training".to_string(),
        inputs: vec![PipelineInput {
            id: "training-data".to_string(),
            input_type: InputType::Artifact {
                artifact_id: "dataset-v1".to_string(),
            },
            config: HashMap::new(),
        }],
        outputs: vec![PipelineOutput {
            id: "model".to_string(),
            output_type: OutputType::Artifact {
                artifact_type: ArtifactType::Model,
            },
            config: HashMap::new(),
        }],
        steps: vec![PipelineStep {
            id: "train".to_string(),
            name: "Train Model".to_string(),
            step_type: StepType::ToadStool {
                workload: "python train.py".to_string(),
                runtime: Some("python".to_string()),
            },
            depends_on: vec![],
            config: HashMap::new(),
        }],
        schedule: None,
        resources: Some(PipelineResources {
            cpu_cores: Some(8.0),
            memory_bytes: Some(16384 * 1024 * 1024), // 16 GB
            storage_bytes: Some(50 * 1024 * 1024 * 1024), // 50 GB
            network_bandwidth: Some(100_000_000),    // 100 Mbps
        }),
    };

    assert_eq!(pipeline.pipeline_id, "ml-train-001");
    assert!(pipeline.schedule.is_none());
}

#[test]
fn test_scenario_stream_processing_pipeline() {
    let pipeline = PipelineConfig {
        pipeline_id: "stream-001".to_string(),
        name: "Real-time Processing".to_string(),
        inputs: vec![PipelineInput {
            id: "events".to_string(),
            input_type: InputType::Stream {
                topic: "user-events".to_string(),
            },
            config: HashMap::new(),
        }],
        outputs: vec![PipelineOutput {
            id: "processed".to_string(),
            output_type: OutputType::Stream {
                topic: "processed-events".to_string(),
            },
            config: HashMap::new(),
        }],
        steps: vec![PipelineStep {
            id: "filter".to_string(),
            name: "Filter Events".to_string(),
            step_type: StepType::Filter {
                condition: "event.type == 'important'".to_string(),
            },
            depends_on: vec![],
            config: HashMap::new(),
        }],
        schedule: Some(PipelineSchedule {
            schedule_type: ScheduleType::Event {
                trigger: "stream_data_available".to_string(),
            },
            timezone: None,
            max_concurrent: Some(10),
        }),
        resources: None,
    };

    assert_eq!(pipeline.steps.len(), 1);
    assert_eq!(
        pipeline.schedule.as_ref().unwrap().max_concurrent.unwrap(),
        10
    );
}
