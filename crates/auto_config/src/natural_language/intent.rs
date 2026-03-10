// SPDX-License-Identifier: AGPL-3.0-only
//! Intent recognition for natural language configuration
//!
//! This module handles recognizing user intent from natural language descriptions
//! and mapping them to configuration templates.

use std::collections::HashMap;
use tracing::{debug, info};

use super::types::{
    ConfigurationIntent, ExplicitPreferences, IntentAnalysis, PerformancePreference,
    SecurityPreference,
};
use crate::ToadStoolResult;

/// Create default intent patterns for common workload types
#[must_use]
pub fn create_intent_patterns() -> HashMap<String, ConfigurationIntent> {
    let mut patterns = HashMap::new();

    // Machine Learning Intent
    patterns.insert(
        "machine_learning".to_string(),
        ConfigurationIntent {
            keywords: vec![
                "machine learning".to_string(),
                "ml".to_string(),
                "ai".to_string(),
                "neural network".to_string(),
                "model training".to_string(),
                "tensorflow".to_string(),
                "pytorch".to_string(),
                "jupyter".to_string(),
                "data science".to_string(),
                "gpu acceleration".to_string(),
            ],
            priority_features: vec![
                "gpu_runtime".to_string(),
                "high_memory".to_string(),
                "python_runtime".to_string(),
            ],
            performance_preference: PerformancePreference::MaximumPerformance,
            security_preference: SecurityPreference::Balanced,
        },
    );

    // Web Development Intent
    patterns.insert(
        "web_development".to_string(),
        ConfigurationIntent {
            keywords: vec![
                "web".to_string(),
                "website".to_string(),
                "web app".to_string(),
                "frontend".to_string(),
                "backend".to_string(),
                "api".to_string(),
                "rest".to_string(),
                "graphql".to_string(),
                "react".to_string(),
                "vue".to_string(),
                "angular".to_string(),
                "node".to_string(),
                "express".to_string(),
            ],
            priority_features: vec![
                "container_runtime".to_string(),
                "wasm_runtime".to_string(),
                "fast_startup".to_string(),
            ],
            performance_preference: PerformancePreference::Balanced,
            security_preference: SecurityPreference::High,
        },
    );

    // Data Processing Intent
    patterns.insert(
        "data_processing".to_string(),
        ConfigurationIntent {
            keywords: vec![
                "data".to_string(),
                "etl".to_string(),
                "pipeline".to_string(),
                "batch".to_string(),
                "processing".to_string(),
                "analytics".to_string(),
                "spark".to_string(),
                "hadoop".to_string(),
            ],
            priority_features: vec![
                "high_memory".to_string(),
                "high_cpu".to_string(),
                "container_runtime".to_string(),
            ],
            performance_preference: PerformancePreference::HighPerformance,
            security_preference: SecurityPreference::Balanced,
        },
    );

    // Gaming Intent
    patterns.insert(
        "gaming".to_string(),
        ConfigurationIntent {
            keywords: vec![
                "game".to_string(),
                "gaming".to_string(),
                "unity".to_string(),
                "unreal".to_string(),
                "graphics".to_string(),
                "rendering".to_string(),
            ],
            priority_features: vec!["gpu_runtime".to_string(), "high_performance".to_string()],
            performance_preference: PerformancePreference::MaximumPerformance,
            security_preference: SecurityPreference::Minimal,
        },
    );

    patterns
}

/// Analyze natural language text to extract user intent
#[expect(
    clippy::implicit_hasher,
    reason = "standard HashMap sufficient for intent patterns"
)]
pub fn analyze_intent(
    text: &str,
    patterns: &HashMap<String, ConfigurationIntent>,
) -> ToadStoolResult<IntentAnalysis> {
    info!("🔍 Analyzing intent from natural language input");

    let text_lower = text.to_lowercase();
    let mut intent_scores: HashMap<String, (f64, Vec<String>)> = HashMap::new();

    // Score each intent based on keyword matches
    for (intent_name, intent) in patterns {
        let mut score = 0.0;
        let mut matched = Vec::new();

        for keyword in &intent.keywords {
            if text_lower.contains(&keyword.to_lowercase()) {
                score += 1.0;
                matched.push(keyword.clone());
            }
        }

        if score > 0.0 {
            intent_scores.insert(intent_name.clone(), (score, matched));
        }
    }

    // Find the highest scoring intent
    // ✅ FIXED: Use total_cmp to handle NaN gracefully (treats NaN as less than all values)
    let (primary_intent, (confidence, matched_keywords)) = intent_scores
        .iter()
        .max_by(|a, b| a.1 .0.total_cmp(&b.1 .0))
        .map_or_else(
            || ("general_purpose".to_string(), (0.0, Vec::new())),
            |(name, (score, keywords))| (name.clone(), (*score, keywords.clone())),
        );

    // Extract explicit preferences from text
    let explicit_preferences = extract_explicit_preferences(&text_lower);

    // Calculate secondary intents
    let mut secondary_intents: Vec<(String, f64)> = intent_scores
        .iter()
        .filter(|(name, _)| **name != primary_intent)
        .map(|(name, (score, _))| (name.clone(), *score))
        .collect();

    // ✅ FIXED: Use total_cmp to handle NaN gracefully
    secondary_intents.sort_by(|a, b| b.1.total_cmp(&a.1));

    debug!(
        "Primary intent: {} (confidence: {:.2})",
        primary_intent, confidence
    );

    Ok(IntentAnalysis {
        primary_intent,
        confidence,
        matched_keywords,
        secondary_intents,
        explicit_preferences,
    })
}

/// Extract explicit preferences from natural language text
#[must_use]
pub fn extract_explicit_preferences(text: &str) -> ExplicitPreferences {
    let mut prefs = ExplicitPreferences::default();

    // Performance keywords
    if text.contains("fast") || text.contains("high performance") || text.contains("maximum") {
        prefs.performance_priority = Some("high".to_string());
    } else if text.contains("power saver") || text.contains("efficient") {
        prefs.performance_priority = Some("low".to_string());
    }

    // Security keywords
    if text.contains("secure") || text.contains("security") || text.contains("isolated") {
        prefs.security_priority = Some("high".to_string());
    }

    // GPU preference
    if text.contains("gpu") {
        prefs.use_gpu = Some(true);
    }

    // Container preference
    if text.contains("container") || text.contains("docker") {
        prefs.use_containers = Some(true);
    }

    // Memory usage
    if text.contains("low memory") {
        prefs.memory_usage = Some("low".to_string());
    } else if text.contains("high memory") {
        prefs.memory_usage = Some("high".to_string());
    }

    prefs
}
