// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::VecDeque;
use std::time::SystemTime;

use crate::types::AnalyticsDataPoint;

pub const MAX_BUFFER_SIZE: usize = 10_000;

pub fn query_data_points<'a>(
    buffer: &'a VecDeque<AnalyticsDataPoint>,
    metric_name: &str,
    since: SystemTime,
) -> Vec<&'a AnalyticsDataPoint> {
    buffer
        .iter()
        .filter(|dp| dp.metric_name == metric_name && dp.timestamp >= since)
        .collect()
}
