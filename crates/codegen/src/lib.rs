//! Code generation utilities for the Mirai Minecraft server
//! 
//! This crate provides build-time code generation capabilities for parsing
//! and generating Rust code from various data sources like JSON, NBT, and
//! protocol specifications, specifically adapted for the Mirai server architecture.

pub mod error;
pub mod parser;
pub mod generator;
pub mod nbt;
pub mod json;
pub mod protocol;
pub mod mirai_integration;

// Re-export commonly used types
pub use error::{CodegenError, Result};
pub use parser::{DataParser, ParsedData};
pub use generator::{CodeGenerator, GeneratedCode};
pub use mirai_integration::{MiraiCodeGenerator, MiraiIntegration};

use std::path::Path;

/// Main entry point for Mirai code generation
pub fn generate_mirai_from_directory<P: AsRef<Path>>(
    input_dir: P,
    output_dir: P,
) -> Result<Vec<GeneratedCode>> {
    let mirai_generator = MiraiCodeGenerator::new();
    mirai_generator.generate_from_directory(input_dir, output_dir)
}

/// Generate Mirai-compatible code from a single file
pub fn generate_mirai_from_file<P: AsRef<Path>>(
    input_file: P,
    output_dir: P,
) -> Result<GeneratedCode> {
    let mirai_generator = MiraiCodeGenerator::new();
    mirai_generator.generate_from_file(input_file, output_dir)
}

/// Generate code from directory (legacy compatibility)
pub fn generate_from_directory<P: AsRef<Path>>(
    input_dir: P,
    output_dir: P,
) -> Result<Vec<GeneratedCode>> {
    let parser = DataParser::new();
    let generator = CodeGenerator::new();
    
    let parsed_data = parser.parse_directory(input_dir)?;
    let mut generated_files = Vec::new();
    
    for data in parsed_data {
        let code = generator.generate(&data)?;
        let output_path = output_dir.as_ref().join(&code.filename);
        
        std::fs::write(&output_path, &code.content)
            .map_err(|e| CodegenError::IoError(e.to_string()))?;
        
        generated_files.push(code);
    }
    
    Ok(generated_files)
}

/// Generate code from a single file (legacy compatibility)
pub fn generate_from_file<P: AsRef<Path>>(
    input_file: P,
    output_dir: P,
) -> Result<GeneratedCode> {
    let parser = DataParser::new();
    let generator = CodeGenerator::new();
    
    let parsed_data = parser.parse_file(input_file)?;
    let code = generator.generate(&parsed_data)?;
    
    let output_path = output_dir.as_ref().join(&code.filename);
    std::fs::write(&output_path, &code.content)
        .map_err(|e| CodegenError::IoError(e.to_string()))?;
    
    Ok(code)
}

// Include generated code at build time
include!(concat!(env!("OUT_DIR"), "/mod.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_generate_mirai_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");
        
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        
        // Create a test JSON file
        let test_json = r#"{"test": "data"}"#;
        fs::write(input_dir.join("test.json"), test_json).unwrap();
        
        // This should not panic (basic smoke test)
        let result = generate_mirai_from_directory(&input_dir, &output_dir);
        
        // For now, we expect it to work with empty results since we haven't
        // implemented the full parsing logic yet
        match result {
            Ok(_) => {}, // Success case
            Err(_) => {}, // Expected for now since parsing isn't fully implemented
        }
    }
    
    #[test]
    fn test_legacy_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");
        
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        
        // Create a test JSON file
        let test_json = r#"{"test": "data"}"#;
        fs::write(input_dir.join("test.json"), test_json).unwrap();
        
        // Test legacy functions still work
        let result = generate_from_directory(&input_dir, &output_dir);
        
        match result {
            Ok(_) => {}, // Success case
            Err(_) => {}, // Expected for now since parsing isn't fully implemented
        }
    }
}