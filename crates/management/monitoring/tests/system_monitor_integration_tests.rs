//! Integration tests for SystemResourceMonitor
//!
//! Tests actual function execution to increase coverage

use std::path::Path;
use toadstool_management_monitoring::*;

#[cfg(test)]
mod system_resource_monitor_tests {
    use super::*;

    #[test]
    fn test_system_resource_monitor_new() {
        let monitor = SystemResourceMonitor::new();
        // Verify it creates successfully
        assert!(format!("{:?}", monitor).contains("SystemResourceMonitor"));
    }

    #[test]
    fn test_system_resource_monitor_with_default_config() {
        let config = MonitoringConfig::default();
        let monitor = SystemResourceMonitor::with_config(config);
        assert!(format!("{:?}", monitor).contains("SystemResourceMonitor"));
    }

    #[test]
    fn test_system_resource_monitor_with_custom_config() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Log,
            metrics_retention: std::time::Duration::from_secs(7200),
        };

        let monitor = SystemResourceMonitor::with_config(config);
        assert!(format!("{:?}", monitor).contains("SystemResourceMonitor"));
    }

    #[test]
    fn test_system_resource_monitor_with_all_granularities() {
        let granularities = vec![
            MonitoringGranularity::SubMillisecond,
            MonitoringGranularity::Millisecond,
            MonitoringGranularity::HighFrequency,
            MonitoringGranularity::Standard,
            MonitoringGranularity::LowFrequency,
        ];

        for granularity in granularities {
            let config = MonitoringConfig {
                granularity,
                ..Default::default()
            };
            let monitor = SystemResourceMonitor::with_config(config);
            assert!(format!("{:?}", monitor).contains("SystemResourceMonitor"));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_process() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("/usr/bin/test");

        let result = monitor.register_process("test-workload", 12345, path).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_multiple_processes() {
        let monitor = SystemResourceMonitor::new();

        let processes = vec![
            ("workload-1", 10001, "/usr/bin/app1"),
            ("workload-2", 10002, "/usr/bin/app2"),
            ("workload-3", 10003, "/usr/bin/app3"),
        ];

        for (id, pid, path) in processes {
            let result = monitor.register_process(id, pid, Path::new(path)).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_unregister_process() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("/usr/bin/test");

        // Register first
        monitor
            .register_process("test-workload", 12345, path)
            .await
            .unwrap();

        // Then unregister
        let result = monitor.unregister_process("test-workload").await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_unregister_nonexistent_process() {
        let monitor = SystemResourceMonitor::new();

        let result = monitor.unregister_process("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_set_thresholds() {
        let monitor = SystemResourceMonitor::new();

        let requirements = toadstool::resources::ResourceRequirements::default();

        let result = monitor.set_thresholds("test-workload", requirements).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_set_thresholds_multiple_workloads() {
        let monitor = SystemResourceMonitor::new();

        let workloads = vec![
            ("workload-1", 1.0, 512),
            ("workload-2", 2.0, 1024),
            ("workload-3", 4.0, 2048),
        ];

        for (id, _cpu, _mem) in workloads {
            let requirements = toadstool::resources::ResourceRequirements::default();

            let result = monitor.set_thresholds(id, requirements).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_metrics_nonexistent_workload() {
        let monitor = SystemResourceMonitor::new();

        let result = monitor.get_metrics_async("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_stop_monitoring_loop() {
        let monitor = SystemResourceMonitor::new();

        let result = monitor.stop_monitoring_loop().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_unregister_cycle() {
        let monitor = SystemResourceMonitor::new();
        let path = Path::new("/usr/bin/test");

        // Register
        monitor
            .register_process("cycle-test", 99999, path)
            .await
            .unwrap();

        // Unregister
        monitor.unregister_process("cycle-test").await.unwrap();

        // Try to get metrics (should fail since unregistered)
        let result = monitor.get_metrics_async("cycle-test").await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_process_registration_with_different_paths() {
        let monitor = SystemResourceMonitor::new();

        let paths = vec![
            "/usr/bin/app",
            "/usr/local/bin/service",
            "/opt/myapp/executable",
            "relative/path/app",
        ];

        for (i, path_str) in paths.iter().enumerate() {
            let workload_id = format!("workload-{i}");
            let result = monitor
                .register_process(&workload_id, (20000 + i) as u32, Path::new(path_str))
                .await;
            assert!(result.is_ok());
        }
    }
}

#[cfg(test)]
mod threshold_action_integration_tests {
    use super::*;

    #[test]
    fn test_threshold_action_in_config() {
        let actions = vec![
            ThresholdAction::Log,
            ThresholdAction::Alert,
            ThresholdAction::Terminate,
        ];

        for action in actions {
            let config = MonitoringConfig {
                threshold_action: action,
                ..Default::default()
            };

            let _monitor = SystemResourceMonitor::with_config(config);
            // Successfully creates with each action type
        }
    }
}

#[cfg(test)]
mod granularity_integration_tests {
    use super::*;

    #[test]
    fn test_all_granularities_create_monitor() {
        let granularities = vec![
            MonitoringGranularity::SubMillisecond,
            MonitoringGranularity::Millisecond,
            MonitoringGranularity::HighFrequency,
            MonitoringGranularity::Standard,
            MonitoringGranularity::LowFrequency,
            MonitoringGranularity::Custom(std::time::Duration::from_millis(500)),
        ];

        for gran in granularities {
            let config = MonitoringConfig {
                granularity: gran,
                ..Default::default()
            };
            let _monitor = SystemResourceMonitor::with_config(config);
        }
    }

    #[test]
    fn test_custom_granularity_durations() {
        let custom_durations = vec![
            std::time::Duration::from_micros(50),
            std::time::Duration::from_millis(5),
            std::time::Duration::from_millis(250),
            std::time::Duration::from_secs(2),
        ];

        for duration in custom_durations {
            let granularity = MonitoringGranularity::Custom(duration);
            assert_eq!(granularity.to_duration(), duration);

            let config = MonitoringConfig {
                granularity,
                ..Default::default()
            };
            let _monitor = SystemResourceMonitor::with_config(config);
        }
    }
}
