//! JSON parsing and code generation utilities for Mirai

use crate::error::{CodegenError, Result};
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// JSON schema analyzer for generating Rust types
pub struct JsonSchemaAnalyzer;

impl JsonSchemaAnalyzer {
    /// Analyze JSON structure and generate Rust types
    pub fn analyze_and_generate(json: &JsonValue, type_name: &str) -> Result<String> {
        match json {
            JsonValue::Object(obj) => Self::generate_struct_from_object(obj, type_name),
            JsonValue::Array(arr) => Self::generate_from_array(arr, type_name),
            _ => Err(CodegenError::GenerationError(
                "Can only generate types from JSON objects or arrays".to_string()
            )),
        }
    }
    
    /// Generate a struct from a JSON object
    fn generate_struct_from_object(obj: &Map<String, JsonValue>, struct_name: &str) -> Result<String> {
        let mut code = String::new();
        
        // Add derives and struct declaration
        code.push_str("#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\n");
        code.push_str(&format!("pub struct {} {{\n", struct_name));
        
        // Generate fields
        for (key, value) in obj {
            let field_name = Self::sanitize_field_name(key);
            let field_type = Self::infer_rust_type(value);
            
            // Add serde rename if field name was changed
            if field_name != *key {
                code.push_str(&format!("    #[serde(rename = \"{}\")]\n", key));
            }
            
            code.push_str(&format!("    pub {}: {},\n", field_name, field_type));
        }
        
        code.push_str("}\n");
        
        // Generate implementation block with helper methods
        code.push_str(&format!("\nimpl {} {{\n", struct_name));
        code.push_str("    /// Create a new instance with default values\n");
        code.push_str(&format!("    pub fn new() -> Self {{\n"));
        code.push_str("        Self::default()\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        
        // Generate Default implementation
        code.push_str(&format!("\nimpl Default for {} {{\n", struct_name));
        code.push_str("    fn default() -> Self {\n");
        code.push_str("        Self {\n");
        
        for (key, value) in obj {
            let field_name = Self::sanitize_field_name(key);
            let default_value = Self::generate_default_value(value);
            code.push_str(&format!("            {}: {},\n", field_name, default_value));
        }
        
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        
        Ok(code)
    }
    
    /// Generate code from a JSON array
    fn generate_from_array(arr: &[JsonValue], type_name: &str) -> Result<String> {
        if arr.is_empty() {
            return Ok(format!("pub type {} = Vec<serde_json::Value>;\n", type_name));
        }
        
        // Analyze the first element to determine the array type
        let element_type = Self::infer_rust_type(&arr[0]);
        
        Ok(format!("pub type {} = Vec<{}>;\n", type_name, element_type))
    }
    
    /// Infer Rust type from JSON value
    fn infer_rust_type(value: &JsonValue) -> String {
        match value {
            JsonValue::Null => "Option<serde_json::Value>".to_string(),
            JsonValue::Bool(_) => "bool".to_string(),
            JsonValue::Number(n) => {
                if n.is_i64() {
                    "i64".to_string()
                } else if n.is_u64() {
                    "u64".to_string()
                } else {
                    "f64".to_string()
                }
            }
            JsonValue::String(_) => "String".to_string(),
            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    "Vec<serde_json::Value>".to_string()
                } else {
                    let element_type = Self::infer_rust_type(&arr[0]);
                    format!("Vec<{}>", element_type)
                }
            }
            JsonValue::Object(_) => "serde_json::Map<String, serde_json::Value>".to_string(),
        }
    }
    
    /// Generate default value for a JSON value
    fn generate_default_value(value: &JsonValue) -> String {
        match value {
            JsonValue::Null => "None".to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => {
                if n.is_i64() {
                    n.as_i64().unwrap().to_string()
                } else if n.is_u64() {
                    n.as_u64().unwrap().to_string()
                } else {
                    n.as_f64().unwrap().to_string()
                }
            }
            JsonValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            JsonValue::Array(_) => "Vec::new()".to_string(),
            JsonValue::Object(_) => "serde_json::Map::new()".to_string(),
        }
    }
    
    /// Sanitize field names to be valid Rust identifiers
    fn sanitize_field_name(name: &str) -> String {
        let mut result = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect::<String>();
        
        // Ensure it doesn't start with a number
        if result.chars().next().map_or(false, |c| c.is_numeric()) {
            result = format!("field_{}", result);
        }
        
        // Handle Rust keywords
        match result.as_str() {
            "type" => "r#type".to_string(),
            "match" => "r#match".to_string(),
            "use" => "r#use".to_string(),
            "mod" => "r#mod".to_string(),
            "fn" => "r#fn".to_string(),
            "let" => "r#let".to_string(),
            "mut" => "r#mut".to_string(),
            "const" => "r#const".to_string(),
            "static" => "r#static".to_string(),
            "if" => "r#if".to_string(),
            "else" => "r#else".to_string(),
            "for" => "r#for".to_string(),
            "while" => "r#while".to_string(),
            "loop" => "r#loop".to_string(),
            "break" => "r#break".to_string(),
            "continue" => "r#continue".to_string(),
            "return" => "r#return".to_string(),
            _ => result,
        }
    }
}

/// Behavior pack parser for Minecraft Bedrock Edition (Mirai compatible)
pub struct BehaviorPackParser {
    pub behavior_pack_path: PathBuf,
}

impl BehaviorPackParser {
    /// Create a new behavior pack parser
    pub fn new<P: AsRef<Path>>(behavior_pack_path: P) -> Self {
        Self {
            behavior_pack_path: behavior_pack_path.as_ref().to_path_buf(),
        }
    }
    
    /// Parse all entities from the behavior pack
    pub fn parse_entities(&self) -> Result<Vec<EntityDefinition>> {
        let entities_dir = self.behavior_pack_path.join("entities");
        let mut entities = Vec::new();
        
        if entities_dir.exists() {
            for entry in WalkDir::new(&entities_dir) {
                let entry = entry.map_err(|e| CodegenError::IoError(e.to_string()))?;
                
                if entry.file_type().is_file() {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "json" {
                            let content = std::fs::read_to_string(entry.path())?;
                            let json: JsonValue = serde_json::from_str(&content)?;
                            
                            match MinecraftJsonParser::parse_entity_definition(&json) {
                                Ok(entity) => entities.push(entity),
                                Err(e) => {
                                    tracing::warn!("Failed to parse entity {}: {}", entry.path().display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(entities)
    }
    
    /// Parse all items from the behavior pack
    pub fn parse_items(&self) -> Result<Vec<ItemDefinition>> {
        let items_dir = self.behavior_pack_path.join("items");
        let mut items = Vec::new();
        
        if items_dir.exists() {
            for entry in WalkDir::new(&items_dir) {
                let entry = entry.map_err(|e| CodegenError::IoError(e.to_string()))?;
                
                if entry.file_type().is_file() {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "json" {
                            let content = std::fs::read_to_string(entry.path())?;
                            let json: JsonValue = serde_json::from_str(&content)?;
                            
                            match MinecraftJsonParser::parse_item_definition(&json) {
                                Ok(item) => items.push(item),
                                Err(e) => {
                                    tracing::warn!("Failed to parse item {}: {}", entry.path().display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(items)
    }
    
    /// Parse all biomes from the behavior pack
    pub fn parse_biomes(&self) -> Result<Vec<BiomeDefinition>> {
        let biomes_dir = self.behavior_pack_path.join("biomes");
        let mut biomes = Vec::new();
        
        if biomes_dir.exists() {
            for entry in WalkDir::new(&biomes_dir) {
                let entry = entry.map_err(|e| CodegenError::IoError(e.to_string()))?;
                
                if entry.file_type().is_file() {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "json" {
                            let content = std::fs::read_to_string(entry.path())?;
                            let json: JsonValue = serde_json::from_str(&content)?;
                            
                            match MinecraftJsonParser::parse_biome_definition(&json) {
                                Ok(biome) => biomes.push(biome),
                                Err(e) => {
                                    tracing::warn!("Failed to parse biome {}: {}", entry.path().display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(biomes)
    }
    
    /// Parse all behavior pack data
    pub fn parse_all(&self) -> Result<BehaviorPackData> {
        let entities = self.parse_entities()?;
        let items = self.parse_items()?;
        let biomes = self.parse_biomes()?;
        
        Ok(BehaviorPackData {
            entities,
            items,
            biomes,
        })
    }
}

/// Complete behavior pack data
#[derive(Debug, Clone)]
pub struct BehaviorPackData {
    pub entities: Vec<EntityDefinition>,
    pub items: Vec<ItemDefinition>,
    pub biomes: Vec<BiomeDefinition>,
}

/// JSON configuration parser for Minecraft-specific data
pub struct MinecraftJsonParser;

impl MinecraftJsonParser {
    /// Parse entity definitions from behavior pack JSON files
    pub fn parse_entity_definition(json: &JsonValue) -> Result<EntityDefinition> {
        let format_version = json.get("format_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
        
        let minecraft_entity = json.get("minecraft:entity")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:entity key".to_string()))?;
        
        let description = Self::parse_entity_description(minecraft_entity.get("description"))?;
        let component_groups = Self::parse_component_groups(minecraft_entity.get("component_groups"))?;
        let components = Self::parse_entity_components(minecraft_entity.get("components"))?;
        let events = Self::parse_entity_events(minecraft_entity.get("events"))?;
        
        Ok(EntityDefinition {
            format_version,
            description,
            component_groups,
            components,
            events,
        })
    }
    
    /// Parse item definitions from behavior pack JSON files
    pub fn parse_item_definition(json: &JsonValue) -> Result<ItemDefinition> {
        let format_version = json.get("format_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
        
        let minecraft_item = json.get("minecraft:item")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:item key".to_string()))?;
        
        let description = Self::parse_item_description(minecraft_item.get("description"))?;
        let components = Self::parse_item_components(minecraft_item.get("components"))?;
        
        Ok(ItemDefinition {
            format_version,
            description,
            components,
        })
    }
    
    /// Parse biome definitions from behavior pack JSON files
    pub fn parse_biome_definition(json: &JsonValue) -> Result<BiomeDefinition> {
        let format_version = json.get("format_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();
        
        let minecraft_biome = json.get("minecraft:biome")
            .ok_or_else(|| CodegenError::ParseError("Missing minecraft:biome key".to_string()))?;
        
        let description = Self::parse_biome_description(minecraft_biome.get("description"))?;
        let components = Self::parse_biome_components(minecraft_biome.get("components"))?;
        
        Ok(BiomeDefinition {
            format_version,
            description,
            components,
        })
    }
    
    /// Parse entity description from JSON
    fn parse_entity_description(json: Option<&JsonValue>) -> Result<EntityDescription> {
        let description = json.ok_or_else(|| CodegenError::ParseError("Missing entity description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing entity identifier".to_string()))?
            .to_string();
        
        let is_spawnable = description.get("is_spawnable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let is_summonable = description.get("is_summonable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let is_experimental = description.get("is_experimental")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        Ok(EntityDescription {
            identifier,
            is_spawnable,
            is_summonable,
            is_experimental,
        })
    }
    
    /// Parse component groups from JSON
    fn parse_component_groups(json: Option<&JsonValue>) -> Result<HashMap<String, ComponentGroup>> {
        let mut component_groups = HashMap::new();
        
        if let Some(JsonValue::Object(groups)) = json {
            for (name, group_data) in groups {
                let components = Self::parse_component_data(group_data)?;
                component_groups.insert(name.clone(), ComponentGroup { components });
            }
        }
        
        Ok(component_groups)
    }
    
    /// Parse entity components from JSON
    fn parse_entity_components(json: Option<&JsonValue>) -> Result<HashMap<String, JsonValue>> {
        let mut components = HashMap::new();
        
        if let Some(JsonValue::Object(comps)) = json {
            for (name, comp_data) in comps {
                components.insert(name.clone(), comp_data.clone());
            }
        }
        
        Ok(components)
    }
    
    /// Parse entity events from JSON
    fn parse_entity_events(json: Option<&JsonValue>) -> Result<HashMap<String, EntityEvent>> {
        let mut events = HashMap::new();
        
        if let Some(JsonValue::Object(event_obj)) = json {
            for (name, event_data) in event_obj {
                let event = Self::parse_entity_event(event_data)?;
                events.insert(name.clone(), event);
            }
        }
        
        Ok(events)
    }
    
    /// Parse a single entity event from JSON
    fn parse_entity_event(json: &JsonValue) -> Result<EntityEvent> {
        let add_components = if let Some(add) = json.get("add") {
            Self::parse_component_data(add)?
        } else {
            HashMap::new()
        };
        
        let remove_components = if let Some(JsonValue::Object(remove)) = json.get("remove") {
            remove.keys().cloned().collect()
        } else {
            Vec::new()
        };
        
        let sequence = if let Some(JsonValue::Array(seq)) = json.get("sequence") {
            seq.iter().map(|v| v.clone()).collect()
        } else {
            Vec::new()
        };
        
        Ok(EntityEvent {
            add_components,
            remove_components,
            sequence,
        })
    }
    
    /// Parse component data from JSON
    fn parse_component_data(json: &JsonValue) -> Result<HashMap<String, JsonValue>> {
        let mut components = HashMap::new();
        
        if let JsonValue::Object(obj) = json {
            for (name, data) in obj {
                components.insert(name.clone(), data.clone());
            }
        }
        
        Ok(components)
    }
    
    /// Parse item description from JSON
    fn parse_item_description(json: Option<&JsonValue>) -> Result<ItemDescription> {
        let description = json.ok_or_else(|| CodegenError::ParseError("Missing item description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing item identifier".to_string()))?
            .to_string();
        
        Ok(ItemDescription { identifier })
    }
    
    /// Parse item components from JSON
    fn parse_item_components(json: Option<&JsonValue>) -> Result<HashMap<String, JsonValue>> {
        let mut components = HashMap::new();
        
        if let Some(JsonValue::Object(comps)) = json {
            for (name, comp_data) in comps {
                components.insert(name.clone(), comp_data.clone());
            }
        }
        
        Ok(components)
    }
    
    /// Parse biome description from JSON
    fn parse_biome_description(json: Option<&JsonValue>) -> Result<BiomeDescription> {
        let description = json.ok_or_else(|| CodegenError::ParseError("Missing biome description".to_string()))?;
        
        let identifier = description.get("identifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::ParseError("Missing biome identifier".to_string()))?
            .to_string();
        
        Ok(BiomeDescription { identifier })
    }
    
    /// Parse biome components from JSON
    fn parse_biome_components(json: Option<&JsonValue>) -> Result<HashMap<String, JsonValue>> {
        let mut components = HashMap::new();
        
        if let Some(JsonValue::Object(comps)) = json {
            for (name, comp_data) in comps {
                components.insert(name.clone(), comp_data.clone());
            }
        }
        
        Ok(components)
    }
}

/// Entity definition structure from behavior pack
#[derive(Debug, Clone)]
pub struct EntityDefinition {
    pub format_version: String,
    pub description: EntityDescription,
    pub component_groups: HashMap<String, ComponentGroup>,
    pub components: HashMap<String, JsonValue>,
    pub events: HashMap<String, EntityEvent>,
}

/// Entity description structure
#[derive(Debug, Clone)]
pub struct EntityDescription {
    pub identifier: String,
    pub is_spawnable: bool,
    pub is_summonable: bool,
    pub is_experimental: bool,
}

/// Component group structure
#[derive(Debug, Clone)]
pub struct ComponentGroup {
    pub components: HashMap<String, JsonValue>,
}

/// Entity event structure
#[derive(Debug, Clone)]
pub struct EntityEvent {
    pub add_components: HashMap<String, JsonValue>,
    pub remove_components: Vec<String>,
    pub sequence: Vec<JsonValue>,
}

/// Item definition structure from behavior pack
#[derive(Debug, Clone)]
pub struct ItemDefinition {
    pub format_version: String,
    pub description: ItemDescription,
    pub components: HashMap<String, JsonValue>,
}

/// Item description structure
#[derive(Debug, Clone)]
pub struct ItemDescription {
    pub identifier: String,
}

/// Biome definition structure from behavior pack
#[derive(Debug, Clone)]
pub struct BiomeDefinition {
    pub format_version: String,
    pub description: BiomeDescription,
    pub components: HashMap<String, JsonValue>,
}

/// Biome description structure
#[derive(Debug, Clone)]
pub struct BiomeDescription {
    pub identifier: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_sanitize_field_name() {
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("normal_name"), "normal_name");
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("with-dashes"), "with_dashes");
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("with spaces"), "with_spaces");
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("123numeric"), "field_123numeric");
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("type"), "r#type");
        assert_eq!(JsonSchemaAnalyzer::sanitize_field_name("fn"), "r#fn");
    }
    
    #[test]
    fn test_infer_rust_type() {
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!(null)), "Option<serde_json::Value>");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!(true)), "bool");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!(42)), "i64");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!(3.14)), "f64");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!("hello")), "String");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!([])), "Vec<serde_json::Value>");
        assert_eq!(JsonSchemaAnalyzer::infer_rust_type(&json!([1, 2, 3])), "Vec<i64>");
    }
}