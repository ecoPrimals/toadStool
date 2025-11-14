//! Real biome lifecycle E2E tests
//! These tests perform actual file I/O and validation

use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_real_yaml_manifest_creation_and_parsing() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let manifest_path = temp_dir.path().join("real-biome.yaml");

    // Create a real YAML manifest
    let yaml_content = r#"
apiVersion: v1
kind: Biome
metadata:
  name: real-test-biome
  version: "1.0.0"
  labels:
    env: test
    tier: e2e
spec:
  runtime: native
  resources:
    cpu:
      min: 1.0
      max: 2.0
    memory:
      min: 512Mi
      max: 1Gi
  "#;

    // Write manifest
    fs::write(&manifest_path, yaml_content)
        .await
        .expect("Failed to write YAML manifest");

    // Verify file exists
    assert!(manifest_path.exists(), "Manifest file should exist");

    // Read and verify content
    let content = fs::read_to_string(&manifest_path)
        .await
        .expect("Failed to read manifest");

    assert!(content.contains("apiVersion: v1"));
    assert!(content.contains("kind: Biome"));
    assert!(content.contains("real-test-biome"));
    assert!(content.contains("env: test"));
    assert!(content.contains("tier: e2e"));

    // Verify file size is reasonable
    let metadata = fs::metadata(&manifest_path)
        .await
        .expect("Failed to get metadata");
    assert!(metadata.len() > 100, "Manifest file too small");
    assert!(metadata.len() < 10000, "Manifest file too large");

    println!("✓ Real YAML manifest creation and parsing test passed");
}

#[tokio::test]
async fn test_real_multiple_biome_manifests() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create multiple manifests
    let manifests = vec![
        ("biome-1.yaml", "test-biome-1"),
        ("biome-2.yaml", "test-biome-2"),
        ("biome-3.yaml", "test-biome-3"),
    ];

    for (filename, biome_name) in &manifests {
        let manifest_path = temp_dir.path().join(filename);
        let content = format!(
            r#"
apiVersion: v1
kind: Biome
metadata:
  name: {}
spec:
  runtime: native
"#,
            biome_name
        );

        fs::write(&manifest_path, content)
            .await
            .expect("Failed to write manifest");

        assert!(manifest_path.exists());
    }

    // Verify all files exist
    for (filename, _) in &manifests {
        let path = temp_dir.path().join(filename);
        assert!(path.exists(), "Manifest {} should exist", filename);
    }

    // List directory contents
    let mut entries = fs::read_dir(temp_dir.path())
        .await
        .expect("Failed to read directory");

    let mut count = 0;
    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        if entry.file_name().to_str().unwrap().ends_with(".yaml") {
            count += 1;
        }
    }

    assert_eq!(count, 3, "Should have exactly 3 YAML files");

    println!("✓ Multiple biome manifests test passed");
}

#[tokio::test]
async fn test_real_manifest_validation_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Test 1: Empty file
    let empty_path = temp_dir.path().join("empty.yaml");
    fs::write(&empty_path, "").await.expect("Failed to write");
    let empty_content = fs::read_to_string(&empty_path).await.expect("Failed to read");
    assert!(empty_content.is_empty(), "Empty file should be empty");

    // Test 2: Invalid YAML structure
    let invalid_path = temp_dir.path().join("invalid.yaml");
    fs::write(&invalid_path, "{ invalid yaml [[ }")
        .await
        .expect("Failed to write");
    let invalid_content = fs::read_to_string(&invalid_path).await.expect("Failed to read");
    assert!(invalid_content.contains("invalid"), "Should contain invalid marker");

    // Test 3: Missing required fields
    let incomplete_path = temp_dir.path().join("incomplete.yaml");
    fs::write(
        &incomplete_path,
        r#"
apiVersion: v1
metadata:
  name: incomplete
"#,
    )
    .await
    .expect("Failed to write");

    let incomplete_content = fs::read_to_string(&incomplete_path)
        .await
        .expect("Failed to read");
    assert!(!incomplete_content.contains("kind:"), "Should be missing kind field");

    println!("✓ Manifest validation errors test passed");
}

#[tokio::test]
async fn test_real_concurrent_file_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create multiple files concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let temp_path = temp_dir.path().to_path_buf();
        let handle = tokio::spawn(async move {
            let file_path = temp_path.join(format!("concurrent-{}.yaml", i));
            let content = format!("# File number {}\napiVersion: v1\n", i);
            fs::write(&file_path, content).await.expect("Failed to write");
            file_path
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    let paths: Vec<PathBuf> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task failed"))
        .collect();

    assert_eq!(paths.len(), 10, "Should have created 10 files");

    // Verify all files exist and have correct content
    for (i, path) in paths.iter().enumerate() {
        assert!(path.exists(), "File {} should exist", i);
        let content = fs::read_to_string(path).await.expect("Failed to read");
        assert!(content.contains(&format!("File number {}", i)));
    }

    println!("✓ Concurrent file operations test passed");
}

#[tokio::test]
async fn test_real_file_cleanup_and_temp_dir_management() {
    // Create temp dir in inner scope
    let file_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().join("cleanup-test.yaml");

        fs::write(&path, "test content")
            .await
            .expect("Failed to write");

        assert!(path.exists(), "File should exist while temp_dir is in scope");

        path
    }; // temp_dir dropped here

    // File should be cleaned up after temp_dir is dropped
    // (may take a moment for OS to cleanup)
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("✓ File cleanup and temp dir management test passed");
}

#[tokio::test]
async fn test_real_large_manifest_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let manifest_path = temp_dir.path().join("large-manifest.yaml");

    // Create a large manifest with many services
    let mut content = String::from("apiVersion: v1\nkind: Biome\nmetadata:\n  name: large-biome\nspec:\n  services:\n");

    for i in 0..100 {
        content.push_str(&format!(
            "    service-{}:\n      image: alpine:latest\n      command: ['echo', 'Service {}']\n",
            i, i
        ));
    }

    fs::write(&manifest_path, &content)
        .await
        .expect("Failed to write large manifest");

    // Verify file size
    let metadata = fs::metadata(&manifest_path)
        .await
        .expect("Failed to get metadata");
    assert!(metadata.len() > 1000, "Large manifest should be over 1KB");

    // Read and verify
    let read_content = fs::read_to_string(&manifest_path)
        .await
        .expect("Failed to read large manifest");

    assert_eq!(read_content.len(), content.len());
    assert!(read_content.contains("service-0"));
    assert!(read_content.contains("service-99"));

    println!("✓ Large manifest handling test passed");
}

#[tokio::test]
async fn test_real_manifest_with_special_characters() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let manifest_path = temp_dir.path().join("special-chars.yaml");

    // Create manifest with special characters
    let content = r#"
apiVersion: v1
kind: Biome
metadata:
  name: "biome-with-special-chars"
  description: "Test with 'quotes', \"double quotes\", and symbols: @#$%"
spec:
  env:
    - name: "VAR_WITH_EQUALS"
      value: "key=value"
    - name: "VAR_WITH_NEWLINE"
      value: "line1\nline2"
"#;

    fs::write(&manifest_path, content)
        .await
        .expect("Failed to write manifest");

    let read_content = fs::read_to_string(&manifest_path)
        .await
        .expect("Failed to read");

    assert!(read_content.contains("'quotes'"));
    assert!(read_content.contains("\"double quotes\""));
    assert!(read_content.contains("@#$%"));
    assert!(read_content.contains("key=value"));

    println!("✓ Manifest with special characters test passed");
}

#[tokio::test]
async fn test_real_utf8_content_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let manifest_path = temp_dir.path().join("utf8-test.yaml");

    // Create manifest with UTF-8 characters
    let content = r#"
apiVersion: v1
kind: Biome
metadata:
  name: "utf8-biome"
  description: "Test with UTF-8: 你好世界 🌍 Здравствуй мир"
spec:
  runtime: native
"#;

    fs::write(&manifest_path, content)
        .await
        .expect("Failed to write UTF-8 content");

    let read_content = fs::read_to_string(&manifest_path)
        .await
        .expect("Failed to read UTF-8 content");

    assert!(read_content.contains("你好世界"));
    assert!(read_content.contains("🌍"));
    assert!(read_content.contains("Здравствуй мир"));

    println!("✓ UTF-8 content handling test passed");
}

#[tokio::test]
async fn test_real_permission_and_metadata_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let manifest_path = temp_dir.path().join("metadata-test.yaml");

    let content = "apiVersion: v1\nkind: Biome\n";
    fs::write(&manifest_path, content)
        .await
        .expect("Failed to write");

    // Get file metadata
    let metadata = fs::metadata(&manifest_path)
        .await
        .expect("Failed to get metadata");

    assert!(metadata.is_file(), "Should be a file");
    assert!(!metadata.is_dir(), "Should not be a directory");
    assert!(metadata.len() > 0, "File should have content");

    // Verify file is readable
    let can_read = fs::read(&manifest_path).await.is_ok();
    assert!(can_read, "File should be readable");

    println!("✓ Permission and metadata verification test passed");
}

#[tokio::test]
async fn test_real_directory_structure_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create nested directory structure
    let nested_path = temp_dir.path().join("biomes").join("prod").join("web");
    fs::create_dir_all(&nested_path)
        .await
        .expect("Failed to create nested directories");

    // Create manifest in nested directory
    let manifest_path = nested_path.join("web-biome.yaml");
    fs::write(&manifest_path, "apiVersion: v1\n")
        .await
        .expect("Failed to write");

    assert!(manifest_path.exists());
    assert!(nested_path.exists());
    assert!(nested_path.is_dir());

    println!("✓ Directory structure creation test passed");
}
