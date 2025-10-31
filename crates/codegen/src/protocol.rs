//! Protocol packet parser and generator for Mirai
//! 
//! This module provides functionality to parse Java packet definitions from the
//! Protocol repository and generate corresponding Rust packet structures
//! compatible with Mirai's architecture.

use crate::error::{CodegenError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use regex::Regex;

/// Represents a parsed Java packet field
#[derive(Debug, Clone)]
pub struct PacketField {
    pub name: String,
    pub field_type: String,
    pub java_type: String,
    pub is_optional: bool,
    pub is_list: bool,
    pub annotations: Vec<String>,
}

/// Represents a parsed Java packet class
#[derive(Debug, Clone)]
pub struct PacketDefinition {
    pub name: String,
    pub packet_id: Option<String>,
    pub fields: Vec<PacketField>,
    pub imports: Vec<String>,
    pub annotations: Vec<String>,
    pub extends_bedrock_packet: bool,
}

/// Protocol packet parser for Mirai
pub struct ProtocolParser {
    type_mappings: HashMap<String, String>,
}

impl ProtocolParser {
    /// Create a new protocol parser
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();
        
        // Basic type mappings from Java to Rust
        type_mappings.insert("int".to_string(), "i32".to_string());
        type_mappings.insert("long".to_string(), "i64".to_string());
        type_mappings.insert("short".to_string(), "i16".to_string());
        type_mappings.insert("byte".to_string(), "i8".to_string());
        type_mappings.insert("boolean".to_string(), "bool".to_string());
        type_mappings.insert("float".to_string(), "f32".to_string());
        type_mappings.insert("double".to_string(), "f64".to_string());
        type_mappings.insert("String".to_string(), "String".to_string());
        type_mappings.insert("CharSequence".to_string(), "String".to_string());
        
        // Minecraft-specific types
        type_mappings.insert("UUID".to_string(), "Uuid".to_string());
        type_mappings.insert("Vector3f".to_string(), "Vector3f".to_string());
        type_mappings.insert("Vector3i".to_string(), "Vector3i".to_string());
        type_mappings.insert("Vector2f".to_string(), "Vector2f".to_string());
        type_mappings.insert("NbtMap".to_string(), "NbtMap".to_string());
        type_mappings.insert("NbtList".to_string(), "NbtList".to_string());
        
        // Collection types
        type_mappings.insert("List".to_string(), "Vec".to_string());
        type_mappings.insert("ObjectArrayList".to_string(), "Vec".to_string());
        type_mappings.insert("ArrayList".to_string(), "Vec".to_string());
        
        Self { type_mappings }
    }
    
    /// Parse all packet files in the Protocol repository
    pub fn parse_protocol_directory<P: AsRef<Path>>(&self, protocol_dir: P) -> Result<Vec<PacketDefinition>> {
        let packet_dir = protocol_dir.as_ref()
            .join("bedrock-codec")
            .join("src")
            .join("main")
            .join("java")
            .join("org")
            .join("cloudburstmc")
            .join("protocol")
            .join("bedrock")
            .join("packet");
        
        if !packet_dir.exists() {
            return Err(CodegenError::IoError(
                format!("Packet directory not found: {}", packet_dir.display())
            ));
        }
        
        let mut packets = Vec::new();
        
        for entry in fs::read_dir(&packet_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("java") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip base classes and handlers
                    if file_name == "BedrockPacket" || 
                       file_name == "BedrockPacketHandler" || 
                       file_name == "BedrockPacketType" ||
                       file_name == "UnknownPacket" {
                        continue;
                    }
                    
                    match self.parse_packet_file(&path) {
                        Ok(packet) => packets.push(packet),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", file_name, e);
                            // Continue parsing other files
                        }
                    }
                }
            }
        }
        
        Ok(packets)
    }
    
    /// Parse a single Java packet file
    pub fn parse_packet_file<P: AsRef<Path>>(&self, file_path: P) -> Result<PacketDefinition> {
        let content = fs::read_to_string(&file_path)?;
        self.parse_packet_content(&content, file_path.as_ref())
    }
    
    /// Parse packet content from a Java file
    fn parse_packet_content(&self, content: &str, file_path: &Path) -> Result<PacketDefinition> {
        let file_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| CodegenError::ParseError("Invalid file name".to_string()))?;
        
        // Extract class name
        let class_regex = Regex::new(r"public class (\w+)").unwrap();
        let class_name = class_regex.captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| file_name.to_string());
        
        // Check if it implements BedrockPacket
        let extends_bedrock_packet = content.contains("implements BedrockPacket");
        
        // Extract imports
        let import_regex = Regex::new(r"import\s+([^;]+);").unwrap();
        let imports: Vec<String> = import_regex.captures_iter(content)
            .map(|caps| caps[1].to_string())
            .collect();
        
        // Extract class-level annotations
        let annotation_regex = Regex::new(r"@(\w+)(?:\([^)]*\))?").unwrap();
        let annotations: Vec<String> = annotation_regex.captures_iter(content)
            .map(|caps| caps[1].to_string())
            .collect();
        
        // Extract fields
        let fields = self.extract_fields(content)?;
        
        // Try to extract packet ID from BedrockPacketType enum reference
        let packet_id = self.extract_packet_id(content, &class_name);
        
        Ok(PacketDefinition {
            name: class_name,
            packet_id,
            fields,
            imports,
            annotations,
            extends_bedrock_packet,
        })
    }
    
    /// Extract fields from Java class content
    fn extract_fields(&self, content: &str) -> Result<Vec<PacketField>> {
        let mut fields = Vec::new();
        
        // Regex to match field declarations
        let field_regex = Regex::new(
            r"(?m)^\s*(?:@[^\n]*\n\s*)*(?:private|public|protected)?\s*(?:final\s+)?(?:static\s+)?([^=;{]+?)\s+(\w+)(?:\s*=\s*[^;]+)?;"
        ).unwrap();
        
        for caps in field_regex.captures_iter(content) {
            let type_part = caps[1].trim();
            let field_name = caps[2].trim();
            
            // Skip constants and static fields
            if content.contains(&format!("static {}", type_part)) ||
               content.contains(&format!("final {}", type_part)) ||
               field_name.chars().all(|c| c.is_uppercase() || c == '_') {
                continue;
            }
            
            // Skip methods and other non-field declarations
            if type_part.contains("(") || type_part.contains(")") {
                continue;
            }
            
            let field = self.parse_field_type(type_part, field_name)?;
            fields.push(field);
        }
        
        Ok(fields)
    }
    
    /// Parse a field type and convert it to Rust equivalent
    fn parse_field_type(&self, type_str: &str, field_name: &str) -> Result<PacketField> {
        let mut is_optional = false;
        let mut is_list = false;
        let java_type = type_str.to_string();
        let mut rust_type: String;
        
        // Handle generic types like List<T>, Optional<T>
        if let Some(generic_match) = Regex::new(r"(\w+)<(.+)>").unwrap().captures(type_str) {
            let container = generic_match[1].to_string();
            let inner_type = generic_match[2].to_string();
            
            match container.as_str() {
                "List" | "ObjectArrayList" | "ArrayList" => {
                    is_list = true;
                    let inner_rust = self.map_type(&inner_type);
                    rust_type = format!("Vec<{}>", inner_rust);
                }
                "Optional" => {
                    is_optional = true;
                    let inner_rust = self.map_type(&inner_type);
                    rust_type = format!("Option<{}>", inner_rust);
                }
                _ => {
                    rust_type = self.map_type(type_str);
                }
            }
        } else {
            rust_type = self.map_type(type_str);
        }
        
        // Handle array types
        if type_str.contains("[]") {
            is_list = true;
            let base_type = type_str.replace("[]", "");
            let inner_rust = self.map_type(&base_type);
            rust_type = format!("Vec<{}>", inner_rust);
        }
        
        Ok(PacketField {
            name: field_name.to_string(),
            field_type: rust_type,
            java_type,
            is_optional,
            is_list,
            annotations: Vec::new(),
        })
    }
    
    /// Map Java type to Rust type
    fn map_type(&self, java_type: &str) -> String {
        // Remove generic parameters for mapping
        let base_type = if let Some(pos) = java_type.find('<') {
            &java_type[..pos]
        } else {
            java_type
        };
        
        self.type_mappings.get(base_type)
            .cloned()
            .unwrap_or_else(|| {
                // For unknown types, assume they're custom types that will be defined elsewhere
                base_type.to_string()
            })
    }
    
    /// Extract packet ID from getPacketType() method
    fn extract_packet_id(&self, content: &str, class_name: &str) -> Option<String> {
        // Look for getPacketType() method returning BedrockPacketType enum
        let packet_type_regex = Regex::new(r"return BedrockPacketType\.(\w+);").unwrap();
        
        if let Some(caps) = packet_type_regex.captures(content) {
            Some(caps[1].to_string())
        } else {
            // Try to infer from class name by removing "Packet" suffix and converting to UPPER_CASE
            if class_name.ends_with("Packet") {
                let base_name = &class_name[..class_name.len() - 6];
                Some(self.camel_to_upper_snake(base_name))
            } else {
                None
            }
        }
    }
    
    /// Convert CamelCase to UPPER_SNAKE_CASE
    fn camel_to_upper_snake(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch.is_uppercase() && !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_uppercase().next().unwrap());
        }
        
        result
    }
}

impl Default for ProtocolParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol packet generator for Mirai
pub struct ProtocolGenerator {
    parser: ProtocolParser,
}

impl ProtocolGenerator {
    /// Create a new protocol generator
    pub fn new() -> Self {
        Self {
            parser: ProtocolParser::new(),
        }
    }
    
    /// Generate Rust packet definitions from Protocol repository for Mirai
    pub fn generate_mirai_packets<P: AsRef<Path>>(&self, protocol_dir: P, output_dir: P) -> Result<()> {
        let packets = self.parser.parse_protocol_directory(protocol_dir)?;
        let output_path = output_dir.as_ref();
        
        // Generate Mirai-compatible packet enum
        self.generate_mirai_packet_enum(&packets, output_path)?;
        
        // Generate individual packet structs for Mirai
        self.generate_mirai_packet_structs(&packets, output_path)?;
        
        // Generate packet registry for Mirai
        self.generate_mirai_packet_registry(&packets, output_path)?;
        
        Ok(())
    }
    
    /// Generate the main packet enum for Mirai
    fn generate_mirai_packet_enum(&self, packets: &[PacketDefinition], output_dir: &Path) -> Result<()> {
        let mut content = String::new();
        
        content.push_str("//! Auto-generated packet definitions from Protocol repository for Mirai\n\n");
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use uuid::Uuid;\n");
        content.push_str("use std::collections::HashMap;\n\n");
        
        // Generate packet enum
        content.push_str("/// All Bedrock protocol packets for Mirai\n");
        content.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
        content.push_str("pub enum MiraiBedrockPacket {\n");
        
        for packet in packets {
            if packet.extends_bedrock_packet {
                content.push_str(&format!("    {}({}),\n", 
                    packet.name.replace("Packet", ""), 
                    packet.name
                ));
            }
        }
        
        content.push_str("}\n\n");
        
        // Generate packet type enum for IDs
        content.push_str("/// Packet type identifiers for Mirai\n");
        content.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n");
        content.push_str("pub enum MiraiBedrockPacketType {\n");
        
        for packet in packets {
            if let Some(packet_id) = &packet.packet_id {
                content.push_str(&format!("    {},\n", packet_id));
            }
        }
        
        content.push_str("}\n\n");
        
        // Write to file
        let output_path = output_dir.join("mirai_packets.rs");
        fs::write(output_path, content)?;
        
        Ok(())
    }
    
    /// Generate individual packet struct definitions for Mirai
    fn generate_mirai_packet_structs(&self, packets: &[PacketDefinition], output_dir: &Path) -> Result<()> {
        let mut content = String::new();
        
        content.push_str("//! Auto-generated packet struct definitions for Mirai\n\n");
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use uuid::Uuid;\n");
        content.push_str("use std::collections::HashMap;\n\n");
        
        for packet in packets {
            if !packet.extends_bedrock_packet {
                continue;
            }
            
            // Generate packet struct
            content.push_str(&format!("/// {} packet for Mirai\n", packet.name));
            content.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
            content.push_str(&format!("pub struct {} {{\n", packet.name));
            
            for field in &packet.fields {
                content.push_str(&format!("    pub {}: {},\n", 
                    self.snake_case(&field.name), 
                    field.field_type
                ));
            }
            
            content.push_str("}\n\n");
            
            // Generate Default implementation if needed
            if packet.fields.iter().any(|f| f.is_list || f.is_optional) {
                content.push_str(&format!("impl Default for {} {{\n", packet.name));
                content.push_str("    fn default() -> Self {\n");
                content.push_str("        Self {\n");
                
                for field in &packet.fields {
                    let default_value = if field.is_list {
                        "Vec::new()".to_string()
                    } else if field.is_optional {
                        "None".to_string()
                    } else {
                        match field.field_type.as_str() {
                            "bool" => "false".to_string(),
                            "String" => "String::new()".to_string(),
                            "Uuid" => "Uuid::nil()".to_string(),
                            t if t.starts_with("i") || t.starts_with("u") || t.starts_with("f") => "0".to_string(),
                            _ => "Default::default()".to_string(),
                        }
                    };
                    
                    content.push_str(&format!("            {}: {},\n", 
                        self.snake_case(&field.name), 
                        default_value
                    ));
                }
                
                content.push_str("        }\n");
                content.push_str("    }\n");
                content.push_str("}\n\n");
            }
            
            // Generate Mirai-specific packet trait implementation
            if let Some(packet_id) = &packet.packet_id {
                content.push_str(&format!("impl {} {{\n", packet.name));
                content.push_str(&format!("    pub const PACKET_ID: u32 = {};\n", 
                    self.get_packet_id_value(packet_id)));
                content.push_str("    \n");
                content.push_str("    /// Serialize packet for Mirai\n");
                content.push_str("    pub fn serialize_mirai(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {\n");
                content.push_str("        Ok(serde_json::to_vec(self)?)\n");
                content.push_str("    }\n");
                content.push_str("    \n");
                content.push_str("    /// Deserialize packet for Mirai\n");
                content.push_str("    pub fn deserialize_mirai(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {\n");
                content.push_str("        Ok(serde_json::from_slice(data)?)\n");
                content.push_str("    }\n");
                content.push_str("}\n\n");
            }
        }
        
        let output_path = output_dir.join("mirai_packet_structs.rs");
        fs::write(output_path, content)?;
        
        Ok(())
    }
    
    /// Generate packet registry for mapping IDs to types for Mirai
    fn generate_mirai_packet_registry(&self, packets: &[PacketDefinition], output_dir: &Path) -> Result<()> {
        let mut content = String::new();
        
        content.push_str("//! Auto-generated packet registry for Mirai\n\n");
        content.push_str("use super::mirai_packet_structs::*;\n");
        content.push_str("use super::mirai_packets::MiraiBedrockPacketType;\n");
        content.push_str("use std::collections::HashMap;\n\n");
        
        content.push_str("/// Mirai packet registry\n");
        content.push_str("pub struct MiraiPacketRegistry {\n");
        content.push_str("    packet_map: HashMap<u32, String>,\n");
        content.push_str("}\n\n");
        
        content.push_str("impl MiraiPacketRegistry {\n");
        content.push_str("    /// Create new Mirai packet registry\n");
        content.push_str("    pub fn new() -> Self {\n");
        content.push_str("        let mut packet_map = HashMap::new();\n");
        
        for packet in packets {
            if packet.extends_bedrock_packet {
                if let Some(packet_id) = &packet.packet_id {
                    content.push_str(&format!(
                        "        packet_map.insert({}, \"{}\".to_string());\n",
                        self.get_packet_id_value(packet_id),
                        packet.name
                    ));
                }
            }
        }
        
        content.push_str("        \n");
        content.push_str("        Self { packet_map }\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    /// Get packet name by ID\n");
        content.push_str("    pub fn get_packet_name(&self, id: u32) -> Option<&String> {\n");
        content.push_str("        self.packet_map.get(&id)\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
        
        content.push_str("impl Default for MiraiPacketRegistry {\n");
        content.push_str("    fn default() -> Self {\n");
        content.push_str("        Self::new()\n");
        content.push_str("    }\n");
        content.push_str("}\n");
        
        let output_path = output_dir.join("mirai_packet_registry.rs");
        fs::write(output_path, content)?;
        
        Ok(())
    }
    
    /// Convert CamelCase to snake_case
    fn snake_case(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch.is_uppercase() && !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        }
        
        result
    }
    
    /// Get numeric packet ID value (simplified)
    fn get_packet_id_value(&self, packet_id: &str) -> u32 {
        // This is a simplified mapping - in a real implementation,
        // you'd want to parse the actual BedrockPacketType enum
        match packet_id {
            "LOGIN" => 0x01,
            "START_GAME" => 0x0B,
            "DISCONNECT" => 0x05,
            "MOVE_PLAYER" => 0x13,
            "TEXT" => 0x09,
            _ => 0xFF, // Unknown packet
        }
    }
}

impl Default for ProtocolGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_mapping() {
        let parser = ProtocolParser::new();
        
        assert_eq!(parser.map_type("int"), "i32");
        assert_eq!(parser.map_type("String"), "String");
        assert_eq!(parser.map_type("List"), "Vec");
        assert_eq!(parser.map_type("UUID"), "Uuid");
    }
    
    #[test]
    fn test_camel_to_upper_snake() {
        let parser = ProtocolParser::new();
        
        assert_eq!(parser.camel_to_upper_snake("LoginPacket"), "LOGIN_PACKET");
        assert_eq!(parser.camel_to_upper_snake("StartGame"), "START_GAME");
        assert_eq!(parser.camel_to_upper_snake("MovePlayer"), "MOVE_PLAYER");
    }
    
    #[test]
    fn test_snake_case() {
        let generator = ProtocolGenerator::new();
        
        assert_eq!(generator.snake_case("protocolVersion"), "protocol_version");
        assert_eq!(generator.snake_case("playerGameType"), "player_game_type");
        assert_eq!(generator.snake_case("uniqueEntityId"), "unique_entity_id");
    }
}