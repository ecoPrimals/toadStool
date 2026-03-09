// SPDX-License-Identifier: AGPL-3.0-only
//! Monitoring display and export formatting

use std::collections::HashMap;

use crate::monitoring::types::TimeSeries;

/// Format metrics as Prometheus text exposition format
pub fn format_prometheus(series: &HashMap<String, TimeSeries>) -> String {
    let mut output = String::new();

    for (name, time_series) in series {
        if let Some(latest) = time_series.data_points.last() {
            output.push_str(&format!("# TYPE {name} gauge\n"));
            output.push_str(&format!("{name} {}\n", latest.value));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::types::DataPoint;

    #[test]
    fn test_format_prometheus_empty() {
        let series = HashMap::new();
        assert_eq!(format_prometheus(&series), "");
    }

    #[test]
    fn test_format_prometheus_single_metric() {
        let mut series = HashMap::new();
        series.insert(
            "cpu_usage".to_string(),
            TimeSeries {
                name: "cpu_usage".to_string(),
                data_points: vec![DataPoint {
                    timestamp: std::time::SystemTime::now(),
                    value: 42.5,
                }],
                labels: HashMap::new(),
            },
        );
        let out = format_prometheus(&series);
        assert!(out.contains("# TYPE cpu_usage gauge"));
        assert!(out.contains("cpu_usage 42.5"));
    }

    #[test]
    fn test_format_prometheus_uses_latest_point() {
        let mut series = HashMap::new();
        series.insert(
            "mem".to_string(),
            TimeSeries {
                name: "mem".to_string(),
                data_points: vec![
                    DataPoint {
                        timestamp: std::time::SystemTime::now(),
                        value: 10.0,
                    },
                    DataPoint {
                        timestamp: std::time::SystemTime::now(),
                        value: 99.0,
                    },
                ],
                labels: HashMap::new(),
            },
        );
        let out = format_prometheus(&series);
        assert!(out.contains("mem 99"));
        assert!(!out.contains("mem 10"));
    }

    #[test]
    fn test_format_prometheus_empty_series_skipped() {
        let mut series = HashMap::new();
        series.insert(
            "empty".to_string(),
            TimeSeries {
                name: "empty".to_string(),
                data_points: vec![],
                labels: HashMap::new(),
            },
        );
        assert_eq!(format_prometheus(&series), "");
    }
}
