// SPDX-License-Identifier: AGPL-3.0-only
//! Load average via `/proc/loadavg`.

use crate::error::{Result, SysmonError};

/// System load averages.
#[derive(Debug, Clone, Copy)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

/// Read load averages from `/proc/loadavg`.
///
/// # Errors
///
/// Returns an error if `/proc/loadavg` cannot be read.
pub fn load_average() -> Result<LoadAverage> {
    let content = std::fs::read_to_string("/proc/loadavg")
        .map_err(|e| SysmonError::new("/proc/loadavg", e))?;
    Ok(parse_loadavg(&content))
}

fn parse_loadavg(content: &str) -> LoadAverage {
    let mut fields = content.split_whitespace();
    let one = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let five = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let fifteen = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    LoadAverage { one, five, fifteen }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_average_non_negative() {
        let la = load_average().unwrap();
        assert!(la.one >= 0.0);
        assert!(la.five >= 0.0);
        assert!(la.fifteen >= 0.0);
    }

    #[test]
    fn test_parse_loadavg() {
        let la = parse_loadavg("1.23 0.45 0.67 2/500 12345");
        assert!((la.one - 1.23).abs() < f64::EPSILON);
        assert!((la.five - 0.45).abs() < f64::EPSILON);
        assert!((la.fifteen - 0.67).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_loadavg_empty() {
        let la = parse_loadavg("");
        assert_eq!(la.one, 0.0);
        assert_eq!(la.five, 0.0);
        assert_eq!(la.fifteen, 0.0);
    }

    #[test]
    fn test_parse_loadavg_partial() {
        let la = parse_loadavg("5.0");
        assert!((la.one - 5.0).abs() < f64::EPSILON);
        assert_eq!(la.five, 0.0);
        assert_eq!(la.fifteen, 0.0);
    }

    #[test]
    fn test_parse_loadavg_malformed_numbers() {
        let la = parse_loadavg("abc xyz def 1/2 3");
        assert_eq!(la.one, 0.0);
        assert_eq!(la.five, 0.0);
        assert_eq!(la.fifteen, 0.0);
    }

    #[test]
    fn test_parse_loadavg_whitespace_only() {
        let la = parse_loadavg("   \n\t  ");
        assert_eq!(la.one, 0.0);
        assert_eq!(la.five, 0.0);
        assert_eq!(la.fifteen, 0.0);
    }

    #[test]
    fn test_parse_loadavg_extra_fields_ignored() {
        let la = parse_loadavg("1.0 2.0 3.0 4/5 6 extra stuff");
        assert!((la.one - 1.0).abs() < f64::EPSILON);
        assert!((la.five - 2.0).abs() < f64::EPSILON);
        assert!((la.fifteen - 3.0).abs() < f64::EPSILON);
    }
}
