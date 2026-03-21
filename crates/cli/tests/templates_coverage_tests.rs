// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for CLI templates module
//! Exercises `TemplateGenerator`, `parse_template`, `list_templates`, and template creation.

use std::path::PathBuf;

use toadstool_cli::templates::{BiomeTemplate, CustomTemplateSpec, TemplateGenerator};

#[test]
fn test_template_generator_new() {
    let generator = TemplateGenerator::new(PathBuf::from("/tmp"), false);
    // Constructor doesn't panic
    drop(generator);
}

#[test]
fn test_list_templates_returns_all() {
    let templates = TemplateGenerator::list_templates();
    assert!(!templates.is_empty());
    assert!(templates.len() >= 9);

    let names: Vec<&str> = templates.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"basic"));
    assert!(names.contains(&"science"));
    assert!(names.contains(&"ai-research"));
    assert!(names.contains(&"quantum"));
    assert!(names.contains(&"genomics"));
    assert!(names.contains(&"vision"));
    assert!(names.contains(&"distributed"));
    assert!(names.contains(&"sovereign"));
    assert!(names.contains(&"development"));
}

#[test]
fn test_list_templates_descriptions_non_empty() {
    let templates = TemplateGenerator::list_templates();
    for (name, desc) in &templates {
        assert!(!desc.is_empty(), "Template {name} should have description");
    }
}

#[test]
fn test_parse_template_basic() {
    let t = TemplateGenerator::parse_template("basic").unwrap();
    assert!(matches!(t, BiomeTemplate::Basic));
}

#[test]
fn test_parse_template_science() {
    let t = TemplateGenerator::parse_template("science").unwrap();
    assert!(matches!(t, BiomeTemplate::Science));
}

#[test]
fn test_parse_template_ai_research() {
    let t = TemplateGenerator::parse_template("ai-research").unwrap();
    assert!(matches!(t, BiomeTemplate::AiResearch));
}

#[test]
fn test_parse_template_ai_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("ai").unwrap(),
        BiomeTemplate::AiResearch
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("ml").unwrap(),
        BiomeTemplate::AiResearch
    ));
}

#[test]
fn test_parse_template_quantum() {
    let t = TemplateGenerator::parse_template("quantum").unwrap();
    assert!(matches!(t, BiomeTemplate::Quantum));
}

#[test]
fn test_parse_template_genomics_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("genomics").unwrap(),
        BiomeTemplate::Genomics
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("bio").unwrap(),
        BiomeTemplate::Genomics
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("bioinformatics").unwrap(),
        BiomeTemplate::Genomics
    ));
}

#[test]
fn test_parse_template_vision_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("vision").unwrap(),
        BiomeTemplate::Vision
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("cv").unwrap(),
        BiomeTemplate::Vision
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("imaging").unwrap(),
        BiomeTemplate::Vision
    ));
}

#[test]
fn test_parse_template_distributed_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("distributed").unwrap(),
        BiomeTemplate::Distributed
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("cluster").unwrap(),
        BiomeTemplate::Distributed
    ));
}

#[test]
fn test_parse_template_sovereign_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("sovereign").unwrap(),
        BiomeTemplate::Sovereign
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("security").unwrap(),
        BiomeTemplate::Sovereign
    ));
}

#[test]
fn test_parse_template_development_aliases() {
    assert!(matches!(
        TemplateGenerator::parse_template("development").unwrap(),
        BiomeTemplate::Development
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("dev").unwrap(),
        BiomeTemplate::Development
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("test").unwrap(),
        BiomeTemplate::Development
    ));
}

#[test]
fn test_parse_template_case_insensitive() {
    assert!(matches!(
        TemplateGenerator::parse_template("BASIC").unwrap(),
        BiomeTemplate::Basic
    ));
    assert!(matches!(
        TemplateGenerator::parse_template("Science").unwrap(),
        BiomeTemplate::Science
    ));
}

#[test]
fn test_parse_template_unknown_returns_error() {
    let result = TemplateGenerator::parse_template("unknown-template-type");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Unknown template type"));
    assert!(err.to_string().contains("unknown-template-type"));
}

#[test]
fn test_parse_template_empty_returns_error() {
    let result = TemplateGenerator::parse_template("");
    assert!(result.is_err());
}

#[test]
fn test_parse_template_custom() {
    let spec = CustomTemplateSpec {
        name: "my-custom".to_string(),
        description: "Custom template".to_string(),
        primals: vec![],
        services: vec![],
        security_level: "standard".to_string(),
        resource_profile: "default".to_string(),
    };
    let t = BiomeTemplate::Custom(spec);
    assert!(matches!(t, BiomeTemplate::Custom(_)));
}

#[tokio::test]
async fn test_generate_basic_template() {
    let temp_dir =
        std::env::temp_dir().join(format!("toadstool_template_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let generator = TemplateGenerator::new(temp_dir.clone(), true);

    let path = generator.generate(BiomeTemplate::Basic).await.unwrap();
    assert!(path.exists());
    assert!(path.ends_with("biome.yaml"));

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("name:"));
    assert!(content.contains("version:"));
    assert!(content.contains("primals") || content.contains("services"));

    std::fs::remove_dir_all(temp_dir).ok();
}

#[tokio::test]
async fn test_generate_without_force_fails_if_exists() {
    let temp_dir = std::env::temp_dir().join(format!(
        "toadstool_template_force_test_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let existing = temp_dir.join("biome.yaml");
    std::fs::write(&existing, "# existing").unwrap();

    let generator = TemplateGenerator::new(temp_dir.clone(), false);
    let result = generator.generate(BiomeTemplate::Basic).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already exists") || err.to_string().contains("overwrite"));

    std::fs::remove_file(existing).ok();
    std::fs::remove_dir_all(temp_dir).ok();
}

#[tokio::test]
async fn test_generate_science_template() {
    let temp_dir =
        std::env::temp_dir().join(format!("toadstool_science_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let generator = TemplateGenerator::new(temp_dir.clone(), true);

    let path = generator.generate(BiomeTemplate::Science).await.unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.is_empty());

    std::fs::remove_dir_all(temp_dir).ok();
}

#[tokio::test]
async fn test_generate_development_template() {
    let temp_dir = std::env::temp_dir().join(format!("toadstool_dev_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let generator = TemplateGenerator::new(temp_dir.clone(), true);

    let path = generator
        .generate(BiomeTemplate::Development)
        .await
        .unwrap();
    assert!(path.exists());

    std::fs::remove_dir_all(temp_dir).ok();
}
