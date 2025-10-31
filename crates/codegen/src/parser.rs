//! Data parsing infrastructure for various file formats adapted for Mirai

use crate::error::{CodegenError, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Represents parsed data from various sources
#[derive(Debug, Clone)]
pub struct ParsedData {
    pub source_file: PathBuf,
    pub data_type: DataType,
    pub content: DataContent,
    pub metadata: HashMap<String, String>,
}

/// Types of data that can be parsed
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Json,
    Nbt,
    Protocol,
    Registry,
    MiraiEntity,
    MiraiItem,
    MiraiBiome,
}

/// Content of parsed data
#[derive(Debug, Clone)]
pub enum DataContent {
    Json(JsonValue),
    Nbt(Vec<u8>), // Raw NBT data for now
    Protocol(ProtocolData),
    Registry(RegistryData),
    MiraiEntity(MiraiEntityData),
    MiraiItem(MiraiItemData),
    MiraiBiome(MiraiBiomeData),
}

/// Protocol-specific data structure
#[derive(Debug, Clone)]
pub struct ProtocolData {
    pub version: String,
    pub packets: Vec<PacketDefinition>,
}

/// Packet definition for protocol generation
#[derive(Debug, Clone)]
pub struct PacketDefinition {
    pub id: u32,
    pub name: String,
    pub direction: PacketDirection,
    pub fields: Vec<FieldDefinition>,
}

/// Packet direction (client-to-server or server-to-client)
#[derive(Debug, Clone, PartialEq)]
pub enum PacketDirection {
    ClientToServer,
    ServerToClient,
    Bidirectional,
}

/// Field definition for packets and data structures
#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub optional: bool,
    pub description: Option<String>,
}

/// Field types for code generation
#[derive(Debug, Clone)]
pub enum FieldType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    VarInt,
    VarLong,
    Uuid,
    Position,
    Array(Box<FieldType>),
    Optional(Box<FieldType>),
    Custom(String),
}

/// Registry data for game content
#[derive(Debug, Clone)]
pub struct RegistryData {
    pub registry_type: String,
    pub entries: Vec<RegistryEntry>,
}

/// Individual registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub id: u32,
    pub name: String,
    pub properties: HashMap<String, JsonValue>,
}

/// Mirai-specific entity data
#[derive(Debug, Clone)]
pub struct MiraiEntityData {
    pub identifier: String,
    pub components: HashMap<String, JsonValue>,
    pub ecs_components: Vec<String>,
    pub mirai_metadata: HashMap<String, String>,
}

/// Mirai-specific item data
#[derive(Debug, Clone)]
pub struct MiraiItemData {
    pub identifier: String,
    pub components: HashMap<String, JsonValue>,
    pub mirai_metadata: HashMap<String, String>,
}

/// Mirai-specific biome data
#[derive(Debug, Clone)]
pub struct MiraiBiomeData {
    pub identifier: String,
    pub components: HashMap<String, JsonValue>,
    pub mirai_metadata: HashMap<String, String>,
}

/// Main data parser with Mirai integration
pub struct DataParser {
    supported_extensions: Vec<String>,
    mirai_mode: bool,
}

impl DataParser {
    /// Create a new data parser
    pub fn new() -> Self {
        Self {
            supported_extensions: vec![
                "json".to_string(),
                "nbt".to_string(),
                "dat".to_string(),
            ],
            mirai_mode: false,
        }
    }
    
    /// Create a new data parser with Mirai integration enabled
    pub fn new_mirai() -> Self {
        Self {
            supported_extensions: vec![
                "json".to_string(),
                "nbt".to_string(),
                "dat".to_string(),
            ],
            mirai_mode: true,
        }
    }
    
    /// Parse all files in a directory
    pub fn parse_directory<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<ParsedData>> {
        let mut parsed_data = Vec::new();
        
        for entry in WalkDir::new(dir.as_ref()) {
            let entry = entry.map_err(|e| CodegenError::IoError(e.to_string()))?;
            
            if entry.file_type().is_file() {
                if let Some(extension) = entry.path().extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if self.supported_extensions.contains(&ext_str.to_lowercase()) {
                            match self.parse_file(entry.path()) {
                                Ok(data) => parsed_data.push(data),
                                Err(e) => {
                                    tracing::warn!("Failed to parse {}: {}", entry.path().display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(parsed_data)
    }
    
    /// Parse a single file
    pub fn parse_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ParsedData> {
        let path = file_path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| CodegenError::UnsupportedFileType(
                format!("No extension found for {}", path.display())
            ))?;
        
        let content = std::fs::read(path)?;
        
        match extension.to_lowercase().as_str() {
            "json" => self.parse_json(path, &content),
            "nbt" | "dat" => self.parse_nbt(path, &content),
            _ => Err(CodegenError::UnsupportedFileType(extension.to_string())),
        }
    }
    
    /// Parse JSON data with Mirai integration
    fn parse_json(&self, path: &Path, content: &[u8]) -> Result<ParsedData> {
        let json_str = String::from_utf8(content.to_vec())
            .map_err(|e| CodegenError::ParseError(format!("Invalid UTF-8: {}", e)))?;
        
        let json_value: JsonValue = serde_json::from_str(&json_str)?;
        
        // Determine the specific type of JSON data based on content or filename
        let data_type = self.determine_json_type(path, &json_value);
        
        let content = match data_type {
            DataType::Protocol => {
                let protocol_data = self.parse_protocol_json(&json_value)?;
                DataContent::Protocol(protocol_data)
            }
            DataType::Registry => {
                let registry_data = self.parse_registry_json(&json_value)?;
                DataContent::Registry(registry_data)
            }
            DataType::MiraiEntity if self.mirai_mode => {
                let entity_data = self.parse_mirai_entity_json(&json_value)?;
                DataContent::MiraiEntity(entity_data)
            }
            DataType::MiraiItem if self.mirai_mode => {
                let item_data = self.parse_mirai_item_json(&json_value)?;
                DataContent::MiraiItem(item_data)
            }
            DataType::MiraiBiome if self.mirai_mode => {
                let biome_data = self.parse_mirai_biome_json(&json_value)?;
                DataContent::MiraiBiome(biome_data)
            }
            _ => DataContent::Json(json_value),
        };
        
        Ok(ParsedData {
            source_file: path.to_path_buf(),
            data_type,
            content,
            metadata: HashMap::new(),
        })
    }
    
    /// Parse NBT data
    fn parse_nbt(&self, path: &Path, content: &[u8]) -> Result<ParsedData> {
        // For now, just store the raw NBT data
        // In a full implementation, we'd parse the NBT structure
        Ok(ParsedData {
            source_file: path.to_path_buf(),
            data_type: DataType::Nbt,
            content: DataContent::Nbt(content.to_vec()),
            metadata: HashMap::new(),
        })
    }
    
    /// Determine the type of JSON data based on content and filename
    fn determine_json_type(&self, path: &Path, json: &JsonValue) -> DataType {
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        
        // Check for Mirai-specific patterns first
        if self.mirai_mode {
            if filename.contains("entity") || json.get("minecraft:entity").is_some() {
                return DataType::MiraiEntity;
            }
            if filename.contains("item") || json.get("minecraft:item").is_some() {
                return DataType::MiraiItem;
            }
            if filename.contains("biome") || json.get("minecraft:biome").is_some() {
                return DataType::MiraiBiome;
            }
        }
        
        // Fallback to standard types
        if filename.contains("protocol") || filename.contains("packet") {
            DataType::Protocol
        } else if filename.contains("registry") || filename.contains("blocks") || filename.contains("items") {
            DataType::Registry
        } else {
            DataType::Json
        }
    }
    
    /// Parse protocol-specific JSON data
    fn parse_protocol_json(&self, _json: &JsonValue) -> Result<ProtocolData> {
        // Placeholder implementation
        Ok(ProtocolData {
            version: "1.20.1".to_string(),
            packets: Vec::new(),
        })
    }
    
    /// Parse registry-specific JSON data
    fn parse_registry_json(&self, _json: &JsonValue) -> Result<RegistryData> {
        // Placeholder implementation
        Ok(RegistryData {
            registry_type: "unknown".to_string(),
            entries: Vec::new(),
        })
    }
    
    /// Parse Mirai entity JSON data
    fn parse_mirai_entity_json(&self, json: &JsonValue) -> Result<MiraiEntityData> {
        let minecraft_entity = json.get("minecraft:entity")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:entity key".to_string()))?;
        
        let description = minecraft_entity.get("description")
            .ok_or_else(|| CodegenError::ParseError("Missing entity description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing entity identifier".to_string()))?
            .to_string();
        
        let components = if let Some(JsonValue::Object(comps)) = minecraft_entity.get("components") {
            comps.clone()
        } else {
            serde_json::Map::new()
        };
        
        // Extract ECS-compatible components
        let ecs_components = components.keys()
            .filter(|key| key.starts_with("minecraft:"))
            .map(|key| key.clone())
            .collect();
        
        let mut mirai_metadata = HashMap::new();
        mirai_metadata.insert("type".to_string(), "entity".to_string());
        mirai_metadata.insert("source".to_string(), "behavior_pack".to_string());
        
        Ok(MiraiEntityData {
            identifier,
            components: components.into_iter().collect(),
            ecs_components,
            mirai_metadata,
        })
    }
    
    /// Parse Mirai item JSON data
    fn parse_mirai_item_json(&self, json: &JsonValue) -> Result<MiraiItemData> {
        let minecraft_item = json.get("minecraft:item")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:item key".to_string()))?;
        
        let description = minecraft_item.get("description")
            .ok_or_else(|| CodegenError::ParseError("Missing item description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing item identifier".to_string()))?
            .to_string();
        
        let components = if let Some(JsonValue::Object(comps)) = minecraft_item.get("components") {
            comps.clone()
        } else {
            serde_json::Map::new()
        };
        
        let mut mirai_metadata = HashMap::new();
        mirai_metadata.insert("type".to_string(), "item".to_string());
        mirai_metadata.insert("source".to_string(), "behavior_pack".to_string());
        
        Ok(MiraiItemData {
            identifier,
            components: components.into_iter().collect(),
            mirai_metadata,
        })
    }
    
    /// Parse Mirai biome JSON data
    fn parse_mirai_biome_json(&self, json: &JsonValue) -> Result<MiraiBiomeData> {
        let minecraft_biome = json.get("minecraft:biome")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:biome key".to_string()))?;
        
        let description = minecraft_biome.get("description")
            .ok_or_else(|| CodegenError::ParseError("Missing biome description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing biome identifier".to_string()))?
            .to_string();
        
        let components = if let Some(JsonValue::Object(comps)) = minecraft_biome.get("components") {
            comps.clone()
        } else {
            serde_json::Map::new()
        };
        
        let mut mirai_metadata = HashMap::new();
        mirai_metadata.insert("type".to_string(), "biome".to_string());
        mirai_metadata.insert("source".to_string(), "behavior_pack".to_string());
        
        Ok(MiraiBiomeData {
            identifier,
            components: components.into_iter().collect(),
            mirai_metadata,
        })
    }
}

impl Default for DataParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use serde_json::json;
    
    #[test]
    fn test_parse_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("test.json");
        
        let test_json = r#"{"name": "test", "value": 42}"#;
        fs::write(&json_file, test_json).unwrap();
        
        let parser = DataParser::new();
        let result = parser.parse_file(&json_file).unwrap();
        
        assert_eq!(result.data_type, DataType::Json);
        assert_eq!(result.source_file, json_file);
        
        if let DataContent::Json(json) = result.content {
            assert_eq!(json["name"], "test");
            assert_eq!(json["value"], 42);
        } else {
            panic!("Expected JSON content");
        }
    }
    
    #[test]
    fn test_mirai_entity_parsing() {
        let parser = DataParser::new_mirai();
        
        let entity_json = json!({
            "format_version": "1.13.0",
            "minecraft:entity": {
                "description": {
                    "identifier": "minecraft:test_entity"
                },
                "components": {
                    "minecraft:health": {"value": 20},
                    "minecraft:movement": {"value": 0.25}
                }
            }
        });
        
        let result = parser.parse_mirai_entity_json(&entity_json).unwrap();
        
        assert_eq!(result.identifier, "minecraft:test_entity");
        assert_eq!(result.components.len(), 2);
        assert!(result.components.contains_key("minecraft:health"));
        assert!(result.components.contains_key("minecraft:movement"));
        assert_eq!(result.ecs_components.len(), 2);
        assert_eq!(result.mirai_metadata.get("type"), Some(&"entity".to_string()));
    }
    
    #[test]
    fn test_determine_json_type() {
        let parser = DataParser::new_mirai();
        let json = serde_json::json!({"test": "data"});
        
        let entity_path = Path::new("test_entity.json");
        assert_eq!(parser.determine_json_type(entity_path, &json), DataType::MiraiEntity);
        
        let item_path = Path::new("test_item.json");
        assert_eq!(parser.determine_json_type(item_path, &json), DataType::MiraiItem);
        
        let protocol_path = Path::new("protocol.json");
        assert_eq!(parser.determine_json_type(protocol_path, &json), DataType::Protocol);
        
        let generic_path = Path::new("config.json");
        assert_eq!(parser.determine_json_type(generic_path, &json), DataType::Json);
    }
    
    #[test]
    fn test_unsupported_file_type() {
        let temp_dir = TempDir::new().unwrap();
        let unsupported_file = temp_dir.path().join("test.txt");
        
        fs::write(&unsupported_file, "test content").unwrap();
        
        let parser = DataParser::new();
        let result = parser.parse_file(&unsupported_file);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CodegenError::UnsupportedFileType(_)));
    }
}