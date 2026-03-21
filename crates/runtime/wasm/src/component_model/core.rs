// SPDX-License-Identifier: AGPL-3.0-only
//! Core component model types and interfaces

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WebAssembly Component Model Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentModelConfig {
    /// Enable component model support
    pub enabled: bool,
    /// Maximum number of component instances
    pub max_instances: usize,
    /// Component linking timeout in milliseconds
    pub linking_timeout_ms: u64,
    /// Enable component composition
    pub composition_enabled: bool,
    /// Interface definition language support
    pub wit_support: bool,
}

impl Default for ComponentModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_instances: 1000,
            linking_timeout_ms: 5000,
            composition_enabled: true,
            wit_support: true,
        }
    }
}

/// WebAssembly Component Interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInterface {
    /// Interface name
    pub name: String,
    /// Interface version
    pub version: String,
    /// Exported functions
    pub exports: Vec<InterfaceFunction>,
    /// Imported functions
    pub imports: Vec<InterfaceFunction>,
    /// Type definitions
    pub types: Vec<InterfaceType>,
}

/// Interface function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceFunction {
    /// Function name
    pub name: String,
    /// Parameters
    pub params: Vec<InterfaceType>,
    /// Return type
    pub return_type: Option<InterfaceType>,
    /// Function documentation
    pub docs: Option<String>,
}

/// Interface type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterfaceType {
    /// Boolean scalar
    Bool,
    /// Unsigned 8-bit integer
    U8,
    /// Unsigned 16-bit integer
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer
    U64,
    /// Signed 8-bit integer
    S8,
    /// Signed 16-bit integer
    S16,
    /// Signed 32-bit integer
    S32,
    /// Signed 64-bit integer
    S64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// UTF-8 string
    String,
    /// Homogeneous list of element type
    List(Box<Self>),
    /// Named field record
    Record(Vec<(String, Self)>),
    /// Tagged variant with optional payload
    Variant(Vec<(String, Option<Self>)>),
    /// Optional wrapper
    Option(Box<Self>),
    /// Result type (ok, err)
    Result(Box<Self>, Box<Self>),
    /// User-defined type reference
    Custom(String),
}

/// Component value type for function parameters and returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentValue {
    /// Boolean value
    Bool(bool),
    /// Unsigned 8-bit value
    U8(u8),
    /// Unsigned 16-bit value
    U16(u16),
    /// Unsigned 32-bit value
    U32(u32),
    /// Unsigned 64-bit value
    U64(u64),
    /// Signed 8-bit value
    S8(i8),
    /// Signed 16-bit value
    S16(i16),
    /// Signed 32-bit value
    S32(i32),
    /// Signed 64-bit value
    S64(i64),
    /// 32-bit float value
    F32(f32),
    /// 64-bit float value
    F64(f64),
    /// String value
    String(String),
    /// List of values
    List(Vec<Self>),
    /// Record with named fields
    Record(HashMap<String, Self>),
    /// Optional value
    Option(Option<Box<Self>>),
    /// Tagged variant with optional payload
    Variant(String, Option<Box<Self>>),
}

impl ComponentValue {
    /// Check if value matches the expected type
    #[must_use]
    pub fn matches_type(&self, expected: &InterfaceType) -> bool {
        match (self, expected) {
            (Self::Bool(_), InterfaceType::Bool)
            | (Self::U8(_), InterfaceType::U8)
            | (Self::U16(_), InterfaceType::U16)
            | (Self::U32(_), InterfaceType::U32)
            | (Self::U64(_), InterfaceType::U64)
            | (Self::S8(_), InterfaceType::S8)
            | (Self::S16(_), InterfaceType::S16)
            | (Self::S32(_), InterfaceType::S32)
            | (Self::S64(_), InterfaceType::S64)
            | (Self::F32(_), InterfaceType::F32)
            | (Self::F64(_), InterfaceType::F64)
            | (Self::String(_), InterfaceType::String)
            | (Self::Option(None), InterfaceType::Option(_)) => true,
            (Self::List(values), InterfaceType::List(element_type)) => {
                values.iter().all(|v| v.matches_type(element_type))
            }
            (Self::Option(Some(value)), InterfaceType::Option(inner_type)) => {
                value.matches_type(inner_type)
            }
            _ => false,
        }
    }
}
