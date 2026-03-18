// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::VecDeque;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::types::{AnalyticsDataPoint, Dashboard};

pub fn build_dashboard_json(
    dashboard: &Dashboard,
    buffer: &VecDeque<AnalyticsDataPoint>,
) -> ToadStoolResult<serde_json::Value> {
    let mut dashboard_data = serde_json::Map::new();
    dashboard_data.insert(
        "dashboard".to_string(),
        serde_json::to_value(dashboard).map_err(|e| {
            tracing::error!("Failed to serialize dashboard data: {}", e);
            ToadStoolError::runtime(format!("Dashboard serialization failed: {e}"))
        })?,
    );

    let mut panel_data = serde_json::Map::new();

    for panel in &dashboard.panels {
        let mut metrics_data = Vec::new();

        for metric_name in &panel.metrics {
            let data_points: Vec<serde_json::Value> = buffer
                .iter()
                .filter(|dp| {
                    dp.metric_name == *metric_name
                        && dp.timestamp >= panel.time_range.from
                        && dp.timestamp <= panel.time_range.to
                })
                .map(|dp| {
                    serde_json::json!({
                        "timestamp": toadstool_common::system_time_serde::format_rfc3339(dp.timestamp),
                        "value": dp.value
                    })
                })
                .collect();

            metrics_data.push(serde_json::json!({
                "metric_name": metric_name,
                "data": data_points
            }));
        }

        panel_data.insert(panel.id.clone(), serde_json::json!(metrics_data));
    }

    dashboard_data.insert("data".to_string(), serde_json::Value::Object(panel_data));

    Ok(serde_json::Value::Object(dashboard_data))
}
