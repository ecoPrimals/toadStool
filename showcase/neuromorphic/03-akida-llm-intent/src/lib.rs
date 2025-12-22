//! Akida LLM Intent Classification
//!
//! Demonstrates using Akida neuromorphic chip for ultra-low-latency
//! intent classification in LLM pipelines.

pub mod intent;
pub mod pretokenize;
pub mod akida_classifier;
pub mod benchmark;

/// Intent classification categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IntentCategory {
    /// Code generation request
    CodeGeneration,
    
    /// Debug/error help
    Debugging,
    
    /// Explanation/documentation
    Explanation,
    
    /// Refactoring suggestion
    Refactoring,
    
    /// General conversation
    Conversation,
    
    /// System/configuration
    SystemConfig,
    
    /// File operation
    FileOperation,
    
    /// Unknown/other
    Unknown,
}

impl IntentCategory {
    /// Get all categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::CodeGeneration,
            Self::Debugging,
            Self::Explanation,
            Self::Refactoring,
            Self::Conversation,
            Self::SystemConfig,
            Self::FileOperation,
            Self::Unknown,
        ]
    }
    
    /// Convert to index for neural network
    pub fn to_index(&self) -> usize {
        match self {
            Self::CodeGeneration => 0,
            Self::Debugging => 1,
            Self::Explanation => 2,
            Self::Refactoring => 3,
            Self::Conversation => 4,
            Self::SystemConfig => 5,
            Self::FileOperation => 6,
            Self::Unknown => 7,
        }
    }
    
    /// Create from index
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::CodeGeneration,
            1 => Self::Debugging,
            2 => Self::Explanation,
            3 => Self::Refactoring,
            4 => Self::Conversation,
            5 => Self::SystemConfig,
            6 => Self::FileOperation,
            _ => Self::Unknown,
        }
    }
}

/// Classification result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassificationResult {
    pub category: IntentCategory,
    pub confidence: f32,
    pub latency_us: u64,
    pub power_consumption_mw: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intent_category_roundtrip() {
        for category in IntentCategory::all() {
            let idx = category.to_index();
            let recovered = IntentCategory::from_index(idx);
            assert_eq!(category, recovered);
        }
    }
}

