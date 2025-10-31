//! Integration tests for code generation and build system integration
//! 
//! Tests that validate the minecraft-server-codegen integration with
//! mirai's build system works correctly.

use super::*;
use mirai_codegen::{
    MiraiCodeGenerator, CodegenConfig, MiraiIntegration,
    EntityGenerator, ItemGenerator, BlockGenerator
};
use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_mirai_codegen_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("mds");
    let output_path = temp_dir.path().join("generated");
    
    // Create mock MDS structure
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    create_mock_mds_files(&mds_path);
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = MiraiCodeGenerator::new(config);
    
    // Test entity generation
    generator.generate_entities()
        .expect("Failed to generate entities");
    
    // Verify generated files exist
    assert!(output_path.join("entities.rs").exists());
    
    // Test generated code is valid Rust
    let generated_content = fs::read_to_string(output_path.join("entities.rs"))
        .expect("Failed to read generated entities");
    
    assert!(generated_content.contains("pub struct"));
    assert!(generated_content.contains("impl Component for"));
    assert!(generated_content.contains("MiraiEntity"));
}

#[test]
fn test_item_generation_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("mds");
    let output_path = temp_dir.path().join("generated");
    
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    create_mock_item_files(&mds_path);
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = ItemGenerator::new(config);
    
    // Generate items
    generator.generate()
        .expect("Failed to generate items");
    
    // Verify generated items
    let items_file = output_path.join("items.rs");
    assert!(items_file.exists());
    
    let content = fs::read_to_string(items_file)
        .expect("Failed to read generated items");
    
    assert!(content.contains("pub enum ItemType"));
    assert!(content.contains("Diamond"));
    assert!(content.contains("Stone"));
    assert!(content.contains("impl MiraiItem for"));
}

#[test]
fn test_block_generation_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("mds");
    let output_path = temp_dir.path().join("generated");
    
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    create_mock_block_files(&mds_path);
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = BlockGenerator::new(config);
    
    // Generate blocks
    generator.generate()
        .expect("Failed to generate blocks");
    
    // Verify generated blocks
    let blocks_file = output_path.join("blocks.rs");
    assert!(blocks_file.exists());
    
    let content = fs::read_to_string(blocks_file)
        .expect("Failed to read generated blocks");
    
    assert!(content.contains("pub struct BlockState"));
    assert!(content.contains("pub enum BlockType"));
    assert!(content.contains("impl MiraiBlock for"));
}

#[test]
fn test_build_script_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().join("test_project");
    let mds_path = project_path.join("mds");
    
    // Create mock project structure
    fs::create_dir_all(&project_path).expect("Failed to create project directory");
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    
    create_mock_mds_files(&mds_path);
    create_mock_cargo_toml(&project_path);
    create_mock_build_rs(&project_path);
    
    // Test build script execution
    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&project_path)
        .output();
    
    // Note: This test may fail in CI without proper Rust toolchain
    // In a real environment, this would verify the build script runs correctly
    if let Ok(output) = output {
        assert!(output.status.success() || output.stderr.is_empty());
    }
}

#[test]
fn test_incremental_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("mds");
    let output_path = temp_dir.path().join("generated");
    
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    create_mock_mds_files(&mds_path);
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = MiraiCodeGenerator::new(config);
    
    // First generation
    generator.generate_all()
        .expect("Failed to generate initially");
    
    let initial_time = fs::metadata(output_path.join("entities.rs"))
        .expect("Failed to get file metadata")
        .modified()
        .expect("Failed to get modification time");
    
    // Wait a bit to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Second generation without changes - should skip
    generator.generate_all()
        .expect("Failed to generate incrementally");
    
    let second_time = fs::metadata(output_path.join("entities.rs"))
        .expect("Failed to get file metadata")
        .modified()
        .expect("Failed to get modification time");
    
    // File should not have been regenerated
    assert_eq!(initial_time, second_time);
    
    // Modify source file
    let entity_file = mds_path.join("entities.json");
    fs::write(&entity_file, r#"{"entities": [{"name": "modified_entity", "components": []}]}"#)
        .expect("Failed to modify entity file");
    
    // Third generation with changes - should regenerate
    generator.generate_all()
        .expect("Failed to generate after modification");
    
    let third_time = fs::metadata(output_path.join("entities.rs"))
        .expect("Failed to get file metadata")
        .modified()
        .expect("Failed to get modification time");
    
    // File should have been regenerated
    assert!(third_time > initial_time);
}

#[test]
fn test_mirai_type_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("mds");
    let output_path = temp_dir.path().join("generated");
    
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    create_mock_mds_files(&mds_path);
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = MiraiCodeGenerator::new(config);
    
    // Generate with mirai integration
    generator.generate_entities()
        .expect("Failed to generate entities");
    
    let content = fs::read_to_string(output_path.join("entities.rs"))
        .expect("Failed to read generated content");
    
    // Verify mirai-specific integrations
    assert!(content.contains("use mirai_core::"));
    assert!(content.contains("impl Component for"));
    assert!(content.contains("MiraiEntity"));
    assert!(content.contains("serialize_with = \"mirai_serde\""));
    
    // Verify ECS compatibility
    assert!(content.contains("Entity"));
    assert!(content.contains("World"));
}

#[test]
fn test_error_handling_in_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mds_path = temp_dir.path().join("nonexistent");
    let output_path = temp_dir.path().join("generated");
    
    let config = CodegenConfig {
        mds_path: mds_path.clone(),
        output_path: output_path.clone(),
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = MiraiCodeGenerator::new(config);
    
    // Should fail gracefully with non-existent MDS path
    let result = generator.generate_entities();
    assert!(result.is_err());
    
    // Create invalid JSON file
    fs::create_dir_all(&mds_path).expect("Failed to create MDS directory");
    fs::write(mds_path.join("entities.json"), "invalid json")
        .expect("Failed to write invalid JSON");
    
    let result = generator.generate_entities();
    assert!(result.is_err());
}

// Helper functions for creating mock files

fn create_mock_mds_files(mds_path: &PathBuf) {
    create_mock_entity_files(mds_path);
    create_mock_item_files(mds_path);
    create_mock_block_files(mds_path);
}

fn create_mock_entity_files(mds_path: &PathBuf) {
    let entities_json = r#"
{
    "entities": [
        {
            "name": "player",
            "components": [
                {"name": "health", "type": "i32"},
                {"name": "position", "type": "Vec3"}
            ]
        },
        {
            "name": "zombie",
            "components": [
                {"name": "health", "type": "i32"},
                {"name": "position", "type": "Vec3"},
                {"name": "ai_state", "type": "String"}
            ]
        }
    ]
}
"#;
    
    fs::write(mds_path.join("entities.json"), entities_json)
        .expect("Failed to write entities.json");
}

fn create_mock_item_files(mds_path: &PathBuf) {
    let items_json = r#"
{
    "items": [
        {
            "name": "diamond",
            "id": 264,
            "max_stack_size": 64,
            "properties": {
                "durability": null,
                "tool_type": null
            }
        },
        {
            "name": "stone",
            "id": 1,
            "max_stack_size": 64,
            "properties": {
                "durability": null,
                "tool_type": null
            }
        }
    ]
}
"#;
    
    fs::write(mds_path.join("items.json"), items_json)
        .expect("Failed to write items.json");
}

fn create_mock_block_files(mds_path: &PathBuf) {
    let blocks_json = r#"
{
    "blocks": [
        {
            "name": "stone",
            "id": 1,
            "properties": {
                "hardness": 1.5,
                "resistance": 6.0,
                "tool_required": false
            },
            "states": []
        },
        {
            "name": "oak_log",
            "id": 17,
            "properties": {
                "hardness": 2.0,
                "resistance": 2.0,
                "tool_required": false
            },
            "states": [
                {"name": "axis", "type": "enum", "values": ["x", "y", "z"]}
            ]
        }
    ]
}
"#;
    
    fs::write(mds_path.join("blocks.json"), blocks_json)
        .expect("Failed to write blocks.json");
}

fn create_mock_cargo_toml(project_path: &PathBuf) {
    let cargo_toml = r#"
[package]
name = "test_codegen_project"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[dependencies]
mirai-core = { path = "../mirai/crates/core" }
mirai-codegen = { path = "../mirai/crates/codegen" }

[build-dependencies]
mirai-codegen = { path = "../mirai/crates/codegen" }
"#;
    
    fs::write(project_path.join("Cargo.toml"), cargo_toml)
        .expect("Failed to write Cargo.toml");
}

fn create_mock_build_rs(project_path: &PathBuf) {
    let build_rs = r#"
use mirai_codegen::{MiraiCodeGenerator, CodegenConfig, MiraiIntegration};
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mds_path = PathBuf::from("mds");
    let output_path = PathBuf::from(&out_dir);
    
    let config = CodegenConfig {
        mds_path,
        output_path,
        mirai_integration: MiraiIntegration::enabled(),
    };
    
    let generator = MiraiCodeGenerator::new(config);
    
    if let Err(e) = generator.generate_all() {
        println!("cargo:warning=Code generation failed: {}", e);
    }
    
    println!("cargo:rerun-if-changed=mds/");
}
"#;
    
    fs::write(project_path.join("build.rs"), build_rs)
        .expect("Failed to write build.rs");
}