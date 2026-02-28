// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Property type implementations for common testing patterns

use std::fmt::Debug;
use toadstool::ToadStoolResult as Result;

use super::traits::Property;

/// Invariant property that tests a predicate holds for all inputs
pub struct InvariantProperty<T, F> {
    _name: String,
    predicate: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> InvariantProperty<T, F>
where
    F: Fn(&T) -> Result<()>,
{
    /// Create a new invariant property
    #[must_use]
    pub fn new(name: impl Into<String>, predicate: F) -> Self {
        Self {
            _name: name.into(),
            predicate,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F> Property<T> for InvariantProperty<T, F>
where
    F: Fn(&T) -> Result<()>,
{
    fn test(&self, input: &T) -> Result<()> {
        (self.predicate)(input)
    }

    fn name(&self) -> &str {
        &self._name
    }
}

/// Round-trip property for testing encode/decode cycles
pub struct RoundTripProperty<T, F1, F2> {
    _name: String,
    encode: F1,
    decode: F2,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F1, F2> RoundTripProperty<T, F1, F2>
where
    F1: Fn(&T) -> Result<Vec<u8>>,
    F2: Fn(&[u8]) -> Result<T>,
    T: PartialEq + Debug,
{
    /// Create a new round-trip property
    #[must_use]
    pub fn new(name: impl Into<String>, encode: F1, decode: F2) -> Self {
        Self {
            _name: name.into(),
            encode,
            decode,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F1, F2> Property<T> for RoundTripProperty<T, F1, F2>
where
    F1: Fn(&T) -> Result<Vec<u8>>,
    F2: Fn(&[u8]) -> Result<T>,
    T: PartialEq + Debug,
{
    fn test(&self, input: &T) -> Result<()> {
        let encoded = (self.encode)(input)?;
        let decoded = (self.decode)(&encoded)?;

        if *input == decoded {
            Ok(())
        } else {
            Err(toadstool::ToadStoolError::runtime(format!(
                "Round-trip property failed: {input:?} != {decoded:?}"
            )))
        }
    }

    fn name(&self) -> &str {
        &self._name
    }
}

/// Monotonic property for testing order preservation
pub struct MonotonicProperty<T, F> {
    _name: String,
    _function: F,
    _phantom: std::marker::PhantomData<T>,
}

/// Utility macros for common property tests
#[macro_export]
macro_rules! invariant {
    ($name:expr, $predicate:expr) => {
        InvariantProperty::new($name.to_string(), $predicate)
    };
}

#[macro_export]
macro_rules! round_trip {
    ($name:expr, $encode:expr, $decode:expr) => {
        RoundTripProperty::new($name.to_string(), $encode, $decode)
    };
}
