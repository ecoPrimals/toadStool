//! Serde support for `std::time::SystemTime`
//!
//! Serializes as Unix timestamp (seconds since epoch) for portable JSON compatibility.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::SystemTime;

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
