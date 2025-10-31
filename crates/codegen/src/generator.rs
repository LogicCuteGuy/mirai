//! Code generation from parsed data for Mirai

use crate::error::Result;
use crate::parser::{ParsedData, DataContent, ProtocolData, RegistryData, FieldType};
use crate::json::{EntityDefinition, ItemDefinition, BiomeDefinition, BehaviorPackData};
// use crate::nbt::NbtTag; // Unused for now
use quote::{format_ident, quote};

/// Generated code output
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub filename: String,
    pub content: String,
    pub module_name: String,
}

/// Main code generator
pub struct CodeGenerator {
    templates: Templates,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            templates: Templates::new(),
        }
    }
    
    /// Generate code from parsed data
    pub fn generate(&self, data: &ParsedData) -> Result<GeneratedCode> {
        match &data.content {
            DataContent::Json(json) => self.generate_from_json(data, json),
            DataContent::Nbt(nbt_data) => self.generate_from_nbt(data, nbt_data),
            DataContent::Protocol(protocol) => self.generate_from_protocol(data, protocol),
            DataContent::Registry(registry) => self.generate_from_registry(data, registry),
            // Mirai-specific content types are handled by MiraiCodeGenerator
            _ => {
                let module_name = self.get_module_name(&data.source_file);
                let filename = format!("{}.rs", module_name);
                let content = "//! Generated code (unsupported data type)\n".to_string();
                
                Ok(GeneratedCode {
                    filename,
                    content,
                    module_name,
                })
            }
        }
    }
    
    /// Generate code from behavior pack data
    pub fn generate_from_behavior_pack(&self, behavior_pack: &BehaviorPackData) -> Result<Vec<GeneratedCode>> {
        let mut generated_files = Vec::new();
        
        // Generate entities module
        if !behavior_pack.entities.is_empty() {
            let entities_code = self.generate_entities_code(&behavior_pack.entities)?;
            generated_files.push(GeneratedCode {
                filename: "entities.rs".to_string(),
                content: entities_code,
                module_name: "entities".to_string(),
            });
        }
        
        // Generate items module
        if !behavior_pack.items.is_empty() {
            let items_code = self.generate_items_code(&behavior_pack.items)?;
            generated_files.push(GeneratedCode {
                filename: "items.rs".to_string(),
                content: items_code,
                module_name: "items".to_string(),
            });
        }
        
        // Generate biomes module
        if !behavior_pack.biomes.is_empty() {
            let biomes_code = self.generate_biomes_code(&behavior_pack.biomes)?;
            generated_files.push(GeneratedCode {
                filename: "biomes.rs".to_string(),
                content: biomes_code,
                module_name: "biomes".to_string(),
            });
        }
        
        // Generate main module file
        let mod_code = self.generate_mod_file(&generated_files)?;
        generated_files.push(GeneratedCode {
            filename: "mod.rs".to_string(),
            content: mod_code,
            module_name: "generated".to_string(),
        });
        
        Ok(generated_files)
    }
    
    /// Generate code from JSON data
    fn generate_from_json(&self, data: &ParsedData, _json: &serde_json::Value) -> Result<GeneratedCode> {
        let module_name = self.get_module_name(&data.source_file);
        let filename = format!("{}.rs", module_name);
        
        let content = self.templates.json_template(&module_name);
        
        Ok(GeneratedCode {
            filename,
            content,
            module_name,
        })
    }
    
    /// Generate code from NBT data
    fn generate_from_nbt(&self, data: &ParsedData, _nbt_data: &[u8]) -> Result<GeneratedCode> {
        let module_name = self.get_module_name(&data.source_file);
        let filename = format!("{}.rs", module_name);
        
        let content = self.templates.nbt_template(&module_name);
        
        Ok(GeneratedCode {
            filename,
            content,
            module_name,
        })
    }
    
    /// Generate code from protocol data
    fn generate_from_protocol(&self, data: &ParsedData, protocol: &ProtocolData) -> Result<GeneratedCode> {
        let module_name = self.get_module_name(&data.source_file);
        let filename = format!("{}.rs", module_name);
        
        let content = self.generate_protocol_code(protocol)?;
        
        Ok(GeneratedCode {
            filename,
            content,
            module_name,
        })
    }
    
    /// Generate code from registry data
    fn generate_from_registry(&self, data: &ParsedData, registry: &RegistryData) -> Result<GeneratedCode> {
        let module_name = self.get_module_name(&data.source_file);
        let filename = format!("{}.rs", module_name);
        
        let content = self.generate_registry_code(registry)?;
        
        Ok(GeneratedCode {
            filename,
            content,
            module_name,
        })
    }
    
    /// Generate protocol-specific code
    fn generate_protocol_code(&self, protocol: &ProtocolData) -> Result<String> {
        let version_ident = format_ident!("PROTOCOL_VERSION");
        let version = &protocol.version;
        
        let packet_structs = protocol.packets.iter().map(|packet| {
            let packet_name = format_ident!("{}", self.to_pascal_case(&packet.name));
            let packet_id = packet.id;
            
            let fields = packet.fields.iter().map(|field| {
                let field_name = format_ident!("{}", field.name);
                let field_type = self.field_type_to_rust(&field.field_type);
                
                if field.optional {
                    quote! { pub #field_name: Option<#field_type> }
                } else {
                    quote! { pub #field_name: #field_type }
                }
            });
            
            quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #packet_name {
                    #(#fields,)*
                }
                
                impl #packet_name {
                    pub const PACKET_ID: u32 = #packet_id;
                }
            }
        });
        
        let generated = quote! {
            //! Generated protocol code for Mirai
            
            pub const #version_ident: &str = #version;
            
            #(#packet_structs)*
        };
        
        Ok(generated.to_string())
    }
    
    /// Generate registry-specific code
    fn generate_registry_code(&self, registry: &RegistryData) -> Result<String> {
        let registry_name = format_ident!("{}", self.to_pascal_case(&registry.registry_type));
        let _registry_type = &registry.registry_type;
        
        let entries = registry.entries.iter().map(|entry| {
            let entry_name = format_ident!("{}", self.to_screaming_snake_case(&entry.name));
            let entry_id = entry.id;
            let entry_string = &entry.name;
            
            quote! {
                pub const #entry_name: RegistryEntry = RegistryEntry {
                    id: #entry_id,
                    name: #entry_string,
                };
            }
        });
        
        let generated = quote! {
            //! Generated registry code for Mirai
            
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct RegistryEntry {
                pub id: u32,
                pub name: &'static str,
            }
            
            pub struct #registry_name;
            
            impl #registry_name {
                #(#entries)*
            }
        };
        
        Ok(generated.to_string())
    }
    
    /// Convert field type to Rust type
    fn field_type_to_rust(&self, field_type: &FieldType) -> proc_macro2::TokenStream {
        match field_type {
            FieldType::Bool => quote! { bool },
            FieldType::I8 => quote! { i8 },
            FieldType::I16 => quote! { i16 },
            FieldType::I32 => quote! { i32 },
            FieldType::I64 => quote! { i64 },
            FieldType::U8 => quote! { u8 },
            FieldType::U16 => quote! { u16 },
            FieldType::U32 => quote! { u32 },
            FieldType::U64 => quote! { u64 },
            FieldType::F32 => quote! { f32 },
            FieldType::F64 => quote! { f64 },
            FieldType::String => quote! { String },
            FieldType::VarInt => quote! { i32 }, // VarInt is encoded as i32
            FieldType::VarLong => quote! { i64 }, // VarLong is encoded as i64
            FieldType::Uuid => quote! { uuid::Uuid },
            FieldType::Position => quote! { Position },
            FieldType::Array(inner) => {
                let inner_type = self.field_type_to_rust(inner);
                quote! { Vec<#inner_type> }
            }
            FieldType::Optional(inner) => {
                let inner_type = self.field_type_to_rust(inner);
                quote! { Option<#inner_type> }
            }
            FieldType::Custom(type_name) => {
                let ident = format_ident!("{}", type_name);
                quote! { #ident }
            }
        }
    }
    
    /// Get module name from file path
    fn get_module_name(&self, path: &std::path::Path) -> String {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("generated")
            .to_lowercase()
            .replace('-', "_")
    }
    
    /// Convert string to PascalCase
    fn to_pascal_case(&self, s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect()
    }
    
    /// Convert string to SCREAMING_SNAKE_CASE
    fn to_screaming_snake_case(&self, s: &str) -> String {
        s.to_uppercase().replace(' ', "_").replace('-', "_")
    }
    
    /// Generate entities code from entity definitions
    fn generate_entities_code(&self, entities: &[EntityDefinition]) -> Result<String> {
        let entity_structs = entities.iter().map(|entity| {
            let entity_name = self.extract_entity_name(&entity.description.identifier);
            let struct_name = format_ident!("{}", self.to_pascal_case(&entity_name));
            
            let identifier = &entity.description.identifier;
            let is_spawnable = entity.description.is_spawnable;
            let is_summonable = entity.description.is_summonable;
            let is_experimental = entity.description.is_experimental;
            
            // Generate component fields
            let component_fields = entity.components.iter().map(|(name, _value)| {
                let field_name = format_ident!("{}", self.sanitize_field_name(name));
                quote! {
                    pub #field_name: Option<serde_json::Value>,
                }
            });
            
            quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #struct_name {
                    pub identifier: String,
                    pub is_spawnable: bool,
                    pub is_summonable: bool,
                    pub is_experimental: bool,
                    #(#component_fields)*
                }
                
                impl #struct_name {
                    pub const IDENTIFIER: &'static str = #identifier;
                    
                    pub fn new() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                            is_spawnable: #is_spawnable,
                            is_summonable: #is_summonable,
                            is_experimental: #is_experimental,
                            ..Default::default()
                        }
                    }
                }
                
                impl Default for #struct_name {
                    fn default() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                            is_spawnable: #is_spawnable,
                            is_summonable: #is_summonable,
                            is_experimental: #is_experimental,
                        }
                    }
                }
            }
        });
        
        let generated = quote! {
            //! Generated entity definitions from behavior pack for Mirai
            
            use serde::{Deserialize, Serialize};
            
            #(#entity_structs)*
        };
        
        Ok(generated.to_string())
    }
    
    /// Generate items code from item definitions
    fn generate_items_code(&self, items: &[ItemDefinition]) -> Result<String> {
        let item_structs = items.iter().map(|item| {
            let item_name = self.extract_entity_name(&item.description.identifier);
            let struct_name = format_ident!("{}", self.to_pascal_case(&item_name));
            
            let identifier = &item.description.identifier;
            
            // Generate component fields
            let component_fields = item.components.iter().map(|(name, _value)| {
                let field_name = format_ident!("{}", self.sanitize_field_name(name));
                quote! {
                    pub #field_name: Option<serde_json::Value>,
                }
            });
            
            quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #struct_name {
                    pub identifier: String,
                    #(#component_fields)*
                }
                
                impl #struct_name {
                    pub const IDENTIFIER: &'static str = #identifier;
                    
                    pub fn new() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                            ..Default::default()
                        }
                    }
                }
                
                impl Default for #struct_name {
                    fn default() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                        }
                    }
                }
            }
        });
        
        let generated = quote! {
            //! Generated item definitions from behavior pack for Mirai
            
            use serde::{Deserialize, Serialize};
            
            #(#item_structs)*
        };
        
        Ok(generated.to_string())
    }
    
    /// Generate biomes code from biome definitions
    fn generate_biomes_code(&self, biomes: &[BiomeDefinition]) -> Result<String> {
        let biome_structs = biomes.iter().map(|biome| {
            let biome_name = self.extract_entity_name(&biome.description.identifier);
            let struct_name = format_ident!("{}", self.to_pascal_case(&biome_name));
            
            let identifier = &biome.description.identifier;
            
            // Generate component fields
            let component_fields = biome.components.iter().map(|(name, _value)| {
                let field_name = format_ident!("{}", self.sanitize_field_name(name));
                quote! {
                    pub #field_name: Option<serde_json::Value>,
                }
            });
            
            quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #struct_name {
                    pub identifier: String,
                    #(#component_fields)*
                }
                
                impl #struct_name {
                    pub const IDENTIFIER: &'static str = #identifier;
                    
                    pub fn new() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                            ..Default::default()
                        }
                    }
                }
                
                impl Default for #struct_name {
                    fn default() -> Self {
                        Self {
                            identifier: #identifier.to_string(),
                        }
                    }
                }
            }
        });
        
        let generated = quote! {
            //! Generated biome definitions from behavior pack for Mirai
            
            use serde::{Deserialize, Serialize};
            
            #(#biome_structs)*
        };
        
        Ok(generated.to_string())
    }
    
    /// Generate module file
    fn generate_mod_file(&self, generated_files: &[GeneratedCode]) -> Result<String> {
        let modules = generated_files.iter()
            .filter(|file| file.filename != "mod.rs")
            .map(|file| {
                let module_name = format_ident!("{}", &file.module_name);
                quote! { pub mod #module_name; }
            });
        
        let generated = quote! {
            //! Generated Minecraft data structures for Mirai
            
            #(#modules)*
        };
        
        Ok(generated.to_string())
    }
    
    /// Extract entity name from identifier (remove minecraft: prefix)
    fn extract_entity_name(&self, identifier: &str) -> String {
        identifier.strip_prefix("minecraft:")
            .unwrap_or(identifier)
            .to_string()
    }
    
    /// Sanitize field names for Rust
    fn sanitize_field_name(&self, name: &str) -> String {
        let mut result = name
            .strip_prefix("minecraft:")
            .unwrap_or(name)
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
            _ => result,
        }
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Code templates for different data types
struct Templates;

impl Templates {
    fn new() -> Self {
        Self
    }
    
    fn json_template(&self, module_name: &str) -> String {
        format!(
            r#"//! Generated code from JSON data for {} (Mirai)

use serde::{{Deserialize, Serialize}};

// Generated structures will be added here
"#,
            module_name
        )
    }
    
    fn nbt_template(&self, module_name: &str) -> String {
        format!(
            r#"//! Generated code from NBT data for {} (Mirai)

// Generated NBT structures will be added here
"#,
            module_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{PacketDefinition, PacketDirection, FieldDefinition, RegistryEntry};
    use std::collections::HashMap;
    
    #[test]
    fn test_pascal_case_conversion() {
        let generator = CodeGenerator::new();
        
        assert_eq!(generator.to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(generator.to_pascal_case("test"), "Test");
        assert_eq!(generator.to_pascal_case("multi_word_test"), "MultiWordTest");
    }
    
    #[test]
    fn test_screaming_snake_case_conversion() {
        let generator = CodeGenerator::new();
        
        assert_eq!(generator.to_screaming_snake_case("hello world"), "HELLO_WORLD");
        assert_eq!(generator.to_screaming_snake_case("test-item"), "TEST_ITEM");
        assert_eq!(generator.to_screaming_snake_case("simple"), "SIMPLE");
    }
    
    #[test]
    fn test_field_type_conversion() {
        let generator = CodeGenerator::new();
        
        let bool_type = generator.field_type_to_rust(&FieldType::Bool);
        assert_eq!(bool_type.to_string(), "bool");
        
        let string_type = generator.field_type_to_rust(&FieldType::String);
        assert_eq!(string_type.to_string(), "String");
        
        let array_type = generator.field_type_to_rust(&FieldType::Array(Box::new(FieldType::I32)));
        assert_eq!(array_type.to_string(), "Vec < i32 >");
    }
    
    #[test]
    fn test_extract_entity_name() {
        let generator = CodeGenerator::new();
        
        assert_eq!(generator.extract_entity_name("minecraft:chicken"), "chicken");
        assert_eq!(generator.extract_entity_name("custom:entity"), "custom:entity");
        assert_eq!(generator.extract_entity_name("simple"), "simple");
    }
    
    #[test]
    fn test_sanitize_field_name() {
        let generator = CodeGenerator::new();
        
        assert_eq!(generator.sanitize_field_name("minecraft:health"), "health");
        assert_eq!(generator.sanitize_field_name("normal_field"), "normal_field");
        assert_eq!(generator.sanitize_field_name("with-dashes"), "with_dashes");
        assert_eq!(generator.sanitize_field_name("type"), "r#type");
        assert_eq!(generator.sanitize_field_name("123numeric"), "field_123numeric");
    }
}