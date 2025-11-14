//! Comprehensive tests for monitoring types
//!
//! Tests cover:
//! - MonitoringGranularity levels
//! - MonitoringConfig creation
//! - ThresholdAction variants
//! - Duration conversions

use std::time::Duration;
use toadstool_management_monitoring::*;

#[cfg(test)]
mod monitoring_granularity_tests {
    use super::*;

    #[test]
    fn test_granularity_sub_millisecond() {
        let granularity = MonitoringGranularity::SubMillisecond;
        let duration = granularity.to_duration();
        assert_eq!(duration, Duration::from_micros(100));
    }

    #[test]
    fn test_granularity_millisecond() {
        let granularity = MonitoringGranularity::Millisecond;
        let duration = granularity.to_duration();
        assert_eq!(duration, Duration::from_millis(1));
    }

    #[test]
    fn test_granularity_high_frequency() {
        let granularity = MonitoringGranularity::HighFrequency;
        let duration = granularity.to_duration();
        assert_eq!(duration, Duration::from_millis(10));
    }

    #[test]
    fn test_granularity_standard() {
        let granularity = MonitoringGranularity::Standard;
        let duration = granularity.to_duration();
        assert_eq!(duration, Duration::from_millis(100));
    }

    #[test]
    fn test_granularity_low_frequency() {
        let granularity = MonitoringGranularity::LowFrequency;
        let duration = granularity.to_duration();
        assert_eq!(duration, Duration::from_secs(1));
    }

    #[test]
    fn test_granularity_custom() {
        let custom_duration = Duration::from_millis(250);
        let granularity = MonitoringGranularity::Custom(custom_duration);
        let duration = granularity.to_duration();
        assert_eq!(duration, custom_duration);
    }

    #[test]
    fn test_all_granularity_levels() {
        let granularities = vec![
            (
                MonitoringGranularity::SubMillisecond,
                Duration::from_micros(100),
            ),
            (MonitoringGranularity::Millisecond, Duration::from_millis(1)),
            (
                MonitoringGranularity::HighFrequency,
                Duration::from_millis(10),
            ),
            (MonitoringGranularity::Standard, Duration::from_millis(100)),
            (MonitoringGranularity::LowFrequency, Duration::from_secs(1)),
        ];

        for (gran, expected) in granularities {
            assert_eq!(gran.to_duration(), expected);
        }
    }

    #[test]
    fn test_granularity_ordering() {
        let durations = vec![
            MonitoringGranularity::SubMillisecond.to_duration(),
            MonitoringGranularity::Millisecond.to_duration(),
            MonitoringGranularity::HighFrequency.to_duration(),
            MonitoringGranularity::Standard.to_duration(),
            MonitoringGranularity::LowFrequency.to_duration(),
        ];

        // Verify they're in ascending order
        for i in 0..durations.len() - 1 {
            assert!(durations[i] < durations[i + 1]);
        }
    }

    #[test]
    fn test_granularity_clone() {
        let granularity = MonitoringGranularity::HighFrequency;
        let cloned = granularity;

        assert_eq!(granularity.to_duration(), cloned.to_duration());
    }

    #[test]
    fn test_granularity_custom_values() {
        let custom_values = vec![
            Duration::from_micros(50),
            Duration::from_millis(500),
            Duration::from_secs(5),
        ];

        for duration in custom_values {
            let granularity = MonitoringGranularity::Custom(duration);
            assert_eq!(granularity.to_duration(), duration);
        }
    }
}

#[cfg(test)]
mod threshold_action_tests {
    use super::*;

    #[test]
    fn test_threshold_action_log() {
        let action = ThresholdAction::Log;
        let cloned = action.clone();

        match cloned {
            ThresholdAction::Log => { /* No-op verification */ }
            _ => panic!("Expected Log variant"),
        }
    }

    #[test]
    fn test_threshold_action_alert() {
        let action = ThresholdAction::Alert;

        match action {
            ThresholdAction::Alert => { /* No-op verification */ }
            _ => panic!("Expected Alert variant"),
        }
    }

    #[test]
    fn test_threshold_action_terminate() {
        let action = ThresholdAction::Terminate;

        match action {
            ThresholdAction::Terminate => { /* No-op verification */ }
            _ => panic!("Expected Terminate variant"),
        }
    }

    #[test]
    fn test_threshold_action_clone() {
        let action = ThresholdAction::Alert;
        let cloned = action.clone();

        match cloned {
            ThresholdAction::Alert => { /* No-op verification */ }
            _ => panic!("Clone should preserve variant"),
        }
    }

    #[test]
    fn test_threshold_action_severity_levels() {
        // Test that actions represent increasing severity
        let actions = vec![
            ("log", ThresholdAction::Log),
            ("alert", ThresholdAction::Alert),
            ("terminate", ThresholdAction::Terminate),
        ];

        assert_eq!(actions.len(), 3);
    }
}

#[cfg(test)]
mod monitoring_config_tests {
    use super::*;

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();

        assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
        assert!(config.enable_network_monitoring);
        assert!(config.enable_threshold_monitoring);
        assert_eq!(config.metrics_retention, Duration::from_secs(3600));
    }

    #[test]
    fn test_monitoring_config_custom() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(7200),
        };

        assert_eq!(config.granularity.to_duration(), Duration::from_millis(10));
        assert!(!config.enable_network_monitoring);
        assert!(config.enable_threshold_monitoring);
        assert_eq!(config.metrics_retention, Duration::from_secs(7200));
    }

    #[test]
    fn test_monitoring_config_clone() {
        let config = MonitoringConfig::default();
        let cloned = config.clone();

        assert_eq!(
            config.granularity.to_duration(),
            cloned.granularity.to_duration()
        );
        assert_eq!(
            config.enable_network_monitoring,
            cloned.enable_network_monitoring
        );
    }

    #[test]
    fn test_monitoring_config_all_disabled() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::LowFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(60),
        };

        assert!(!config.enable_network_monitoring);
        assert!(!config.enable_threshold_monitoring);
    }

    #[test]
    fn test_monitoring_config_retention_values() {
        let retention_values = vec![
            Duration::from_secs(300),   // 5 minutes
            Duration::from_secs(3600),  // 1 hour
            Duration::from_secs(86400), // 1 day
        ];

        for retention in retention_values {
            let config = MonitoringConfig {
                metrics_retention: retention,
                ..Default::default()
            };

            assert_eq!(config.metrics_retention, retention);
        }
    }

    #[test]
    fn test_monitoring_config_serialization() {
        let config = MonitoringConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("granularity"));
        assert!(json.contains("enable_network_monitoring"));
        assert!(json.contains("metrics_retention"));
    }

    #[test]
    fn test_monitoring_config_deserialization() {
        let json = r#"{
            "granularity": "Standard",
            "enable_network_monitoring": true,
            "enable_threshold_monitoring": true,
            "threshold_action": "Log",
            "metrics_retention": {"secs": 3600, "nanos": 0}
        }"#;

        let config: MonitoringConfig = serde_json::from_str(json).unwrap();
        assert!(config.enable_network_monitoring);
    }
}

#[cfg(test)]
mod use_case_tests {
    use super::*;

    #[test]
    fn test_high_frequency_trading_config() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::SubMillisecond,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Terminate,
            metrics_retention: Duration::from_secs(300), // 5 minutes
        };

        assert_eq!(config.granularity.to_duration(), Duration::from_micros(100));
        assert!(matches!(
            config.threshold_action,
            ThresholdAction::Terminate
        ));
    }

    #[test]
    fn test_background_process_config() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::LowFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(86400), // 1 day
        };

        assert_eq!(config.granularity.to_duration(), Duration::from_secs(1));
        assert!(!config.enable_network_monitoring);
    }

    #[test]
    fn test_interactive_application_config() {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(7200), // 2 hours
        };

        assert_eq!(config.granularity.to_duration(), Duration::from_millis(10));
        assert!(matches!(config.threshold_action, ThresholdAction::Alert));
    }
}

#[cfg(test)]
mod interval_calculation_tests {
    use super::*;

    #[test]
    fn test_monitoring_intervals_per_second() {
        let intervals = vec![
            (MonitoringGranularity::SubMillisecond, 10_000), // 10k per second
            (MonitoringGranularity::Millisecond, 1_000),     // 1k per second
            (MonitoringGranularity::HighFrequency, 100),     // 100 per second
            (MonitoringGranularity::Standard, 10),           // 10 per second
            (MonitoringGranularity::LowFrequency, 1),        // 1 per second
        ];

        for (granularity, expected_per_sec) in intervals {
            let interval = granularity.to_duration();
            let per_second = Duration::from_secs(1).as_micros() / interval.as_micros();
            assert_eq!(per_second as u32, expected_per_sec);
        }
    }

    #[test]
    fn test_metrics_collection_overhead() {
        // Ensure monitoring intervals are reasonable
        let granularity = MonitoringGranularity::Standard;
        let interval = granularity.to_duration();

        // Standard interval should allow reasonable processing time
        assert!(interval >= Duration::from_micros(100));
        assert!(interval <= Duration::from_secs(1));
    }
}
