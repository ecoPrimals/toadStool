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
