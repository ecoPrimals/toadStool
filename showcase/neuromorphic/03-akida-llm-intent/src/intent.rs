//! Intent classification logic

use crate::{IntentCategory, ClassificationResult};
use anyhow::Result;
use std::time::Instant;

/// Simple rule-based classifier for comparison
pub struct RuleBasedClassifier;

impl RuleBasedClassifier {
    pub fn new() -> Self {
        Self
    }
    
    /// Classify user input using simple rules
    pub fn classify(&self, input: &str) -> Result<ClassificationResult> {
        let start = Instant::now();
        let input_lower = input.to_lowercase();
        
        let category = if input_lower.contains("write") && (input_lower.contains("code") || input_lower.contains("function")) {
            IntentCategory::CodeGeneration
        } else if input_lower.contains("error") || input_lower.contains("bug") || input_lower.contains("fix") {
            IntentCategory::Debugging
        } else if input_lower.contains("explain") || input_lower.contains("what is") || input_lower.contains("how does") {
            IntentCategory::Explanation
        } else if input_lower.contains("refactor") || input_lower.contains("improve") || input_lower.contains("optimize") {
            IntentCategory::Refactoring
        } else if input_lower.contains("config") || input_lower.contains("settings") || input_lower.contains("install") {
            IntentCategory::SystemConfig
        } else if input_lower.contains("file") || input_lower.contains("read") || input_lower.contains("create") {
            IntentCategory::FileOperation
        } else if input_lower.ends_with('?') && !input_lower.contains("code") {
            IntentCategory::Conversation
        } else {
            IntentCategory::Unknown
        };
        
        let latency = start.elapsed().as_micros() as u64;
        
        Ok(ClassificationResult {
            category,
            confidence: 0.8, // Rule-based confidence
            latency_us: latency,
            power_consumption_mw: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rule_based_classifier() {
        let classifier = RuleBasedClassifier::new();
        
        let result = classifier.classify("Write a function to sort an array").unwrap();
        assert_eq!(result.category, IntentCategory::CodeGeneration);
        
        let result = classifier.classify("I'm getting a null pointer error").unwrap();
        assert_eq!(result.category, IntentCategory::Debugging);
        
        let result = classifier.classify("What is a closure?").unwrap();
        assert_eq!(result.category, IntentCategory::Explanation);
    }
}

