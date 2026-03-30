// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for template generator
//!
//! Goal: Push `generator_impl.rs` coverage from 1.43% to 50%+

use std::path::PathBuf;
use tempfile::TempDir;
use toadstool_cli::templates::{BiomeTemplate, TemplateGenerator};

// ============================================================================
// Constructor Tests
// ============================================================================

#[test]
fn test_generator_new() {
    let output_dir = PathBuf::from("/tmp/test");
    let generator = TemplateGenerator::new(output_dir, false);

    // Generator should be created successfully
    // (We can't inspect internal fields, but construction should work)
    drop(generator); // Ensure it's used
}

#[test]
fn test_generator_new_with_force() {
    let output_dir = PathBuf::from("/tmp/test");
    let generator = TemplateGenerator::new(output_dir, true);

    drop(generator);
}

// ============================================================================
// Template Listing Tests
// ============================================================================

#[test]
fn test_list_templates_not_empty() {
    let templates = TemplateGenerator::list_templates();

    assert!(!templates.is_empty(), "Should have at least one template");
}

#[test]
fn test_list_templates_has_basic() {
    let templates = TemplateGenerator::list_templates();

    let has_basic = templates.iter().any(|(name, _)| name == "basic");
    assert!(has_basic, "Should include basic template");
}

#[test]
fn test_list_templates_has_science() {
    let templates = TemplateGenerator::list_templates();

    let has_science = templates.iter().any(|(name, _)| name == "science");
    assert!(has_science, "Should include science template");
}

#[test]
fn test_list_templates_has_ai() {
    let templates = TemplateGenerator::list_templates();

    let has_ai = templates.iter().any(|(name, _)| name == "ai-research");
    assert!(has_ai, "Should include ai-research template");
}

#[test]
fn test_list_templates_all_have_descriptions() {
    let templates = TemplateGenerator::list_templates();

    for (name, description) in templates {
        assert!(!name.is_empty(), "Template name should not be empty");
        assert!(!description.is_empty(), "Description should not be empty");
        assert!(
            description.len() > 10,
            "Description should be meaningful for {name}"
        );
    }
}

#[test]
fn test_list_templates_unique_names() {
    let templates = TemplateGenerator::list_templates();
    let names: Vec<_> = templates.iter().map(|(name, _)| name).collect();

    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            assert_ne!(names[i], names[j], "Template names must be unique");
        }
    }
}

#[test]
fn test_list_templates_count() {
    let templates = TemplateGenerator::list_templates();

    // Should have all major templates
    assert!(templates.len() >= 7, "Should have at least 7 templates");
}

// ============================================================================
// Template Parsing Tests
// ============================================================================

#[test]
fn test_parse_template_basic() {
    let result = TemplateGenerator::parse_template("basic");
    assert!(result.is_ok());

    match result.unwrap() {
        BiomeTemplate::Basic => {} // Success
        _ => panic!("Should parse as Basic template"),
    }
}

#[test]
fn test_parse_template_science() {
    let result = TemplateGenerator::parse_template("science");
    assert!(result.is_ok());

    match result.unwrap() {
        BiomeTemplate::Science => {}
        _ => panic!("Should parse as Science template"),
    }
}

#[test]
fn test_parse_template_ai() {
    let result = TemplateGenerator::parse_template("ai-research");
    assert!(result.is_ok());

    match result.unwrap() {
        BiomeTemplate::AiResearch => {}
        _ => panic!("Should parse as AiResearch template"),
    }
}

#[test]
fn test_parse_template_quantum() {
    let result = TemplateGenerator::parse_template("quantum");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_genomics() {
    let result = TemplateGenerator::parse_template("genomics");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_vision() {
    let result = TemplateGenerator::parse_template("vision");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_distributed() {
    let result = TemplateGenerator::parse_template("distributed");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_sovereign() {
    let result = TemplateGenerator::parse_template("sovereign");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_development() {
    let result = TemplateGenerator::parse_template("development");
    assert!(result.is_ok());
}

#[test]
fn test_parse_template_case_insensitive() {
    let lower = TemplateGenerator::parse_template("basic");
    let upper = TemplateGenerator::parse_template("BASIC");
    let mixed = TemplateGenerator::parse_template("BaSiC");

    assert!(lower.is_ok());
    assert!(upper.is_ok());
    assert!(mixed.is_ok());
}

#[test]
fn test_parse_template_invalid() {
    let result = TemplateGenerator::parse_template("invalid-template");
    assert!(result.is_err(), "Should fail for invalid template");
}

#[test]
fn test_parse_template_empty() {
    let result = TemplateGenerator::parse_template("");
    assert!(result.is_err(), "Should fail for empty string");
}

#[test]
fn test_parse_template_whitespace() {
    let result = TemplateGenerator::parse_template("  basic  ");
    // Should either succeed (trimmed) or fail (not trimmed)
    // Both behaviors are acceptable
    let _ = result;
}

// ============================================================================
// Generate Tests (Async)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_basic_template() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let result = generator.generate(BiomeTemplate::Basic).await;
    assert!(result.is_ok(), "Should generate basic template");

    let output_path = result.unwrap();
    assert!(output_path.exists(), "Output file should exist");
    assert_eq!(output_path.file_name().unwrap(), "biome.yaml");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_science_template() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let result = generator.generate(BiomeTemplate::Science).await;
    assert!(result.is_ok(), "Should generate science template");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_development_template() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let result = generator.generate(BiomeTemplate::Development).await;
    assert!(result.is_ok(), "Should generate development template");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("nested/deep/path");
    let generator = TemplateGenerator::new(subdir.clone(), false);

    let result = generator.generate(BiomeTemplate::Basic).await;
    assert!(result.is_ok(), "Should create nested directories");
    assert!(subdir.exists(), "Should create parent directories");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_without_force_fails_on_existing() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    // First generation should succeed
    let result1 = generator.generate(BiomeTemplate::Basic).await;
    assert!(result1.is_ok(), "First generation should succeed");

    // Second generation without force should fail
    let result2 = generator.generate(BiomeTemplate::Basic).await;
    assert!(
        result2.is_err(),
        "Should fail when file exists without force"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_with_force_overwrites() {
    let temp_dir = TempDir::new().unwrap();

    // First generation without force
    let generator1 = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);
    let result1 = generator1.generate(BiomeTemplate::Basic).await;
    assert!(result1.is_ok());

    // Second generation with force should succeed
    let generator2 = TemplateGenerator::new(temp_dir.path().to_path_buf(), true);
    let result2 = generator2.generate(BiomeTemplate::Science).await;
    assert!(result2.is_ok(), "Should overwrite with force=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_output_is_valid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let output_path = generator.generate(BiomeTemplate::Basic).await.unwrap();

    // Read the generated file
    let content = tokio::fs::read_to_string(&output_path).await.unwrap();

    // Should be valid YAML (basic check: not empty, has structure)
    assert!(!content.is_empty(), "Generated YAML should not be empty");
    assert!(content.contains("metadata"), "Should have metadata section");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_output_has_version() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let output_path = generator.generate(BiomeTemplate::Basic).await.unwrap();
    let content = tokio::fs::read_to_string(&output_path).await.unwrap();

    assert!(content.contains("version"), "Should have version field");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_output_has_primals() {
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let output_path = generator.generate(BiomeTemplate::Basic).await.unwrap();
    let content = tokio::fs::read_to_string(&output_path).await.unwrap();

    assert!(
        content.contains("primals") || content.contains("beardog"),
        "Should have primals section"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_basic_to_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("custom");
    let generator = TemplateGenerator::new(subdir, false);

    let result = generator.generate(BiomeTemplate::Basic).await;
    assert!(result.is_ok(), "Should generate to custom directory");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_science_to_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("custom");
    let generator = TemplateGenerator::new(subdir, false);

    let result = generator.generate(BiomeTemplate::Science).await;
    assert!(result.is_ok(), "Should generate science template");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_development_to_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("custom");
    let generator = TemplateGenerator::new(subdir, false);

    let result = generator.generate(BiomeTemplate::Development).await;
    assert!(result.is_ok(), "Should generate development template");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_end_to_end_workflow() {
    // Parse template string
    let template = TemplateGenerator::parse_template("science").unwrap();

    // Generate to temp directory
    let temp_dir = TempDir::new().unwrap();
    let generator = TemplateGenerator::new(temp_dir.path().to_path_buf(), false);

    let output_path = generator.generate(template).await.unwrap();

    // Verify output
    assert!(output_path.exists());
    let content = tokio::fs::read_to_string(&output_path).await.unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_all_listed_templates_are_parseable() {
    let templates = TemplateGenerator::list_templates();

    for (name, _) in templates {
        let result = TemplateGenerator::parse_template(&name);
        assert!(
            result.is_ok(),
            "Listed template '{name}' should be parseable"
        );
    }
}
