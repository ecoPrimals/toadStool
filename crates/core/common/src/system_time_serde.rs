// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serde support for [`SystemTime`]
//!
//! Serializes as Unix timestamp (seconds since epoch) for portable JSON compatibility.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::SystemTime;

/// Format [`SystemTime`] for display (e.g. "2025-02-27 12:00")
#[must_use]
pub fn format_display(t: SystemTime) -> String {
    let rfc = format_rfc3339(t);
    rfc.replace('T', " ").chars().take(16).collect()
}

/// Format [`SystemTime`] as RFC3339 string (e.g. "2025-02-27T12:00:00Z")
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
pub fn format_rfc3339(t: SystemTime) -> String {
    const SECS_PER_DAY: u64 = 86_400;
    let d = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    // days since epoch fits u32 for all realistic dates (up to year ~11_761_191)
    let days = (secs / SECS_PER_DAY) as u32;
    let time_of_day = secs % SECS_PER_DAY;
    let hour = (time_of_day / 3600) as u8;
    let minute = ((time_of_day % 3600) / 60) as u8;
    let second = (time_of_day % 60) as u8;

    let (year, month, day) = days_since_epoch_to_ymd(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert days since 1970-01-01 to (year, month, day) in UTC
fn days_since_epoch_to_ymd(days: u32) -> (u32, u8, u8) {
    const EPOCH: u32 = 719_163; // Rata Die for 1970-01-01
    let rd = days + EPOCH;

    let (year, doy) = rd_to_year_doy(rd);
    let (month, day) = doy_to_month_day(year, doy);

    (year, month, day)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
fn rd_to_year_doy(rd: u32) -> (u32, u16) {
    // Intermediate u64 arithmetic avoids overflow on large Rata Die values
    let mut year = (u64::from(rd) * 400 / 146_097) as u32;
    let mut doy = rd - (u64::from(year) * 146_097 / 400) as u32;

    year += (doy / 36_524) * 100;
    doy %= 36_524;
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
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
            // month 1..=12, day 1..=31 — both guaranteed ≤ 255
            return ((i + 1) as u8, d as u8);
        }
        d -= dim;
    }
    (12, 31)
}

/// Serialize [`SystemTime`] as Unix timestamp (seconds since epoch)
///
/// # Errors
///
/// Returns `S::Error` if the timestamp is before `UNIX_EPOCH` or serialization fails.
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

/// Deserialize [`SystemTime`] from Unix timestamp (seconds since epoch)
///
/// # Errors
///
/// Returns `D::Error` if deserialization fails.
pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

/// Serde support for `Option<SystemTime>` — serializes as null or Unix timestamp
pub mod opt {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::SystemTime;

    use super::serialize as serialize_system_time;

    /// # Errors
    ///
    /// Returns `S::Error` if the timestamp is before `UNIX_EPOCH` or serialization fails.
    pub fn serialize<S>(opt: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            None => serializer.serialize_none(),
            Some(t) => serialize_system_time(t, serializer),
        }
    }

    /// # Errors
    ///
    /// Returns `D::Error` if deserialization fails.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(deserializer).map(|opt| {
            opt.map(|secs| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        })
    }
}
