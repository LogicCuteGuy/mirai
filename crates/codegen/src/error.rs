//! Error types for the mirai codegen crate

use thiserror::Error;

/// Main result type for codegen operations
pub type Result<T> = std::result::Result<T, CodegenError>;

/// Primary error type for code generation
#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Generation error: {0}")]
    GenerationError(String),
    
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("NBT error: {0}")]
    NbtError(String),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Mirai integration error: {0}")]
    MiraiIntegrationError(String),
    
    #[error("ECS compatibility error: {0}")]
    EcsCompatibilityError(String),
}

impl From<std::io::Error> for CodegenError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = CodegenError::ParseError("test error".to_string());
        assert_eq!(error.to_string(), "Parse error: test error");
    }
    
    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let codegen_error = CodegenError::from(io_error);
        
        assert!(matches!(codegen_error, CodegenError::IoError(_)));
        assert!(codegen_error.to_string().contains("file not found"));
    }
    
    #[test]
    fn test_mirai_specific_errors() {
        let mirai_error = CodegenError::MiraiIntegrationError("test mirai error".to_string());
        assert_eq!(mirai_error.to_string(), "Mirai integration error: test mirai error");
        
        let ecs_error = CodegenError::EcsCompatibilityError("test ecs error".to_string());
        assert_eq!(ecs_error.to_string(), "ECS compatibility error: test ecs error");
    }
}