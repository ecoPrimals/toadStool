//! Serde support for `std::time::SystemTime`
//!
//! Serializes as Unix timestamp (seconds since epoch) for portable JSON compatibility.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::SystemTime;

/// Format SystemTime for display (e.g. "2025-02-27 12:00")
#[must_use]
pub fn format_display(t: SystemTime) -> String {
    let rfc = format_rfc3339(t);
    rfc.replace('T', " ").chars().take(16).collect()
}

/// Format SystemTime as RFC3339 string (e.g. "2025-02-27T12:00:00Z")
#[must_use]
pub fn format_rfc3339(t: SystemTime) -> String {
    let d = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();

    const SECS_PER_DAY: u64 = 86400;
    let days = (secs / SECS_PER_DAY) as u32;
    let time_of_day = secs % SECS_PER_DAY;
    let hour = (time_of_day / 3600) as u8;
    let minute = ((time_of_day % 3600) / 60) as u8;
    let second = (time_of_day % 60) as u8;

    let (year, month, day) = days_since_epoch_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// Convert days since 1970-01-01 to (year, month, day) in UTC
fn days_since_epoch_to_ymd(days: u32) -> (u32, u8, u8) {
    // Use simplified Gregorian calendar conversion
    const EPOCH: u32 = 719_163; // Rata Die for 1970-01-01
    let rd = days + EPOCH;

    let (year, doy) = rd_to_year_doy(rd);
    let (month, day) = doy_to_month_day(year, doy);

    (year, month, day)
}

#[allow(unused_assignments)]
fn rd_to_year_doy(rd: u32) -> (u32, u16) {
    let mut year = 0u32;
    let mut doy = rd;

    // Approximate year
    year = (doy as u64 * 400 / 146_097) as u32;
    doy -= (year as u64 * 146_097 / 400) as u32;

    // Refine
    year += (doy / 36524) * 100;
    doy %= 36524;
    year += (doy / 1461) * 4;
    doy %= 1461;
    year += doy / 365;
    doy %= 365;

    if doy == 0 {
        year -= 1;
        doy = 365;
    }

    (year, doy as u16)
}

fn doy_to_month_day(year: u32, doy: u16) -> (u8, u8) {
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_month: [u16; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut d = doy;
    for (i, &dim) in days_in_month.iter().enumerate() {
        if d <= dim {
            return ((i + 1) as u8, d as u8);
        }
        d -= dim;
    }
    (12, 31)
}

/// Serialize SystemTime as Unix timestamp (seconds since epoch)
pub fn serialize<S>(t: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(serde::ser::Error::custom)?;
    secs.serialize(serializer)
}

/// Deserialize SystemTime from Unix timestamp (seconds since epoch)
pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

/// Serde support for `Option<SystemTime>` - serializes as null or Unix timestamp
pub mod opt {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::SystemTime;

    use super::serialize as serialize_system_time;

    pub fn serialize<S>(opt: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            None => serializer.serialize_none(),
            Some(t) => serialize_system_time(t, serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(deserializer).map(|opt| {
            opt.map(|secs| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        })
    }
}
