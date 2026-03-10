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
    /// Basic types
    Bool,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    String,
    /// Complex types
    List(Box<InterfaceType>),
    Record(Vec<(String, InterfaceType)>),
    Variant(Vec<(String, Option<InterfaceType>)>),
    Option(Box<InterfaceType>),
    Result(Box<InterfaceType>, Box<InterfaceType>),
    /// Custom types
    Custom(String),
}

/// Component value type for function parameters and returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    S8(i8),
    S16(i16),
    S32(i32),
    S64(i64),
    F32(f32),
    F64(f64),
    String(String),
    List(Vec<ComponentValue>),
    Record(HashMap<String, ComponentValue>),
    Option(Option<Box<ComponentValue>>),
    Variant(String, Option<Box<ComponentValue>>),
}

impl ComponentValue {
    /// Check if value matches the expected type
    #[must_use]
    pub fn matches_type(&self, expected: &InterfaceType) -> bool {
        match (self, expected) {
            (ComponentValue::Bool(_), InterfaceType::Bool)
            | (ComponentValue::U8(_), InterfaceType::U8)
            | (ComponentValue::U16(_), InterfaceType::U16)
            | (ComponentValue::U32(_), InterfaceType::U32)
            | (ComponentValue::U64(_), InterfaceType::U64)
            | (ComponentValue::S8(_), InterfaceType::S8)
            | (ComponentValue::S16(_), InterfaceType::S16)
            | (ComponentValue::S32(_), InterfaceType::S32)
            | (ComponentValue::S64(_), InterfaceType::S64)
            | (ComponentValue::F32(_), InterfaceType::F32)
            | (ComponentValue::F64(_), InterfaceType::F64)
            | (ComponentValue::String(_), InterfaceType::String)
            | (ComponentValue::Option(None), InterfaceType::Option(_)) => true,
            (ComponentValue::List(values), InterfaceType::List(element_type)) => {
                values.iter().all(|v| v.matches_type(element_type))
            }
            (ComponentValue::Option(Some(value)), InterfaceType::Option(inner_type)) => {
                value.matches_type(inner_type)
            }
            _ => false,
        }
    }
}
