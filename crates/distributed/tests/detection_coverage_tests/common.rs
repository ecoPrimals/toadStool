// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Serialize, de::DeserializeOwned};

pub fn serde_json_roundtrip<T>(value: &T) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}
