//! Mirai-specific code generation integration
//! 
//! This module provides code generation specifically tailored for the Mirai
//! Minecraft server architecture, including ECS integration and compatibility
//! with existing Mirai APIs.

use crate::error::{CodegenError, Result};
use crate::parser::{DataParser, ParsedData, DataContent, MiraiEntityData, MiraiItemData, MiraiBiomeData};
use crate::generator::{CodeGenerator, GeneratedCode};
use std::path::Path;

/// Mirai-specific code generator
pub struct MiraiCodeGenerator {
    parser: DataParser,
    generator: CodeGenerator,
    mirai_config: MiraiIntegration,
}

/// Configuration for Mirai integration
#[derive(Debug, Clone)]
pub struct MiraiIntegration {
    pub ecs_enabled: bool,
    pub existing_api_compatibility: bool,
    pub plugin_system_integration: bool,
    pub performance_optimizations: bool,
}

impl MiraiCodeGenerator {
    /// Create a new Mirai code generator
    pub fn new() -> Self {
        Self {
            parser: DataParser::new_mirai(),
            generator: CodeGenerator::new(),
            mirai_config: MiraiIntegration::default(),
        }
    }
    
    /// Create a new Mirai code generator with custom configuration
    pub fn with_config(config: MiraiIntegration) -> Self {
        Self {
            parser: DataParser::new_mirai(),
            generator: CodeGenerator::new(),
            mirai_config: config,
        }
    }
    
    /// Generate Mirai-compatible code from directory
    pub fn generate_from_directory<P: AsRef<Path>>(
        &self,
        input_dir: P,
        output_dir: P,
    ) -> Result<Vec<GeneratedCode>> {
        let parsed_data = self.parser.parse_directory(input_dir)?;
        let mut generated_files = Vec::new();
        
        for data in parsed_data {
            let code = self.generate_mirai_code(&data)?;
            let output_path = output_dir.as_ref().join(&code.filename);
            
            std::fs::write(&output_path, &code.content)
                .map_err(|e| CodegenError::IoError(e.to_string()))?;
            
            generated_files.push(code);
        }
        
        Ok(generated_files)
    }
    
    /// Generate Mirai-compatible code from single file
    pub fn generate_from_file<P: AsRef<Path>>(
        &self,
        input_file: P,
        output_dir: P,
    ) -> Result<GeneratedCode> {
        let parsed_data = self.parser.parse_file(input_file)?;
        let code = self.generate_mirai_code(&parsed_data)?;
        
        let output_path = output_dir.as_ref().join(&code.filename);
        std::fs::write(&output_path, &code.content)
            .map_err(|e| CodegenError::IoError(e.to_string()))?;
        
        Ok(code)
    }
    
    /// Generate Mirai-specific code from parsed data
    fn generate_mirai_code(&self, data: &ParsedData) -> Result<GeneratedCode> {
        match &data.content {
            DataContent::MiraiEntity(entity_data) => {
                self.generate_mirai_entity_code(entity_data, data)
            }
            DataContent::MiraiItem(item_data) => {
                self.generate_mirai_item_code(item_data, data)
            }
            DataContent::MiraiBiome(biome_data) => {
                self.generate_mirai_biome_code(biome_data, data)
            }
            _ => {
                // Fallback to standard generation
                self.generator.generate(data)
            }
        }
    }
    
    /// Generate Mirai entity code with ECS integration
    fn generate_mirai_entity_code(
        &self,
        entity_data: &MiraiEntityData,
        parsed_data: &ParsedData,
    ) -> Result<GeneratedCode> {
        let entity_name = self.extract_entity_name(&entity_data.identifier);
        let struct_name = self.to_pascal_case(&entity_name);
        let module_name = self.get_module_name(&parsed_data.source_file);
        
        let mut content = String::new();
        
        // Add imports for Mirai integration
        content.push_str("//! Generated entity definition for Mirai\n\n");
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use std::collections::HashMap;\n");
        
        if self.mirai_config.ecs_enabled {
            content.push_str("use mirai_core::ecs::{Component, Entity, World};\n");
        }
        
        content.push_str("\n");
        
        // Generate the main entity struct compatible with mirai's existing data formats
        content.push_str(&format!("/// {} entity for Mirai (compatible with existing systems)\n", struct_name));
        content.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        content.push_str(&format!("pub struct {} {{\n", struct_name));
        content.push_str("    /// Minecraft identifier for this entity\n");
        content.push_str("    pub identifier: String,\n");
        content.push_str("    /// Whether this entity can be spawned naturally\n");
        content.push_str("    pub is_spawnable: bool,\n");
        content.push_str("    /// Whether this entity can be summoned via commands\n");
        content.push_str("    pub is_summonable: bool,\n");
        content.push_str("    /// Whether this entity is experimental\n");
        content.push_str("    pub is_experimental: bool,\n");
        content.push_str("    /// Component data compatible with mirai's existing format\n");
        content.push_str("    pub components: HashMap<String, serde_json::Value>,\n");
        
        if self.mirai_config.ecs_enabled {
            content.push_str("    /// ECS entity ID when spawned in world\n");
            content.push_str("    #[serde(skip)]\n");
            content.push_str("    pub entity_id: Option<Entity>,\n");
        }
        
        content.push_str("}\n\n");
        
        // Generate ECS component implementations if enabled
        if self.mirai_config.ecs_enabled {
            content.push_str(&format!("impl Component for {} {{}}\n\n", struct_name));
            
            // Generate individual component structs for each minecraft component
            for component_name in &entity_data.ecs_components {
                let component_struct_name = self.component_name_to_struct(component_name);
                content.push_str(&format!(
                    "/// ECS component for {}\n\
                     #[derive(Debug, Clone, Serialize, Deserialize)]\n\
                     pub struct {} {{\n\
                     /// Component data from behavior pack\n\
                     pub data: serde_json::Value,\n\
                     /// Cached parsed values for performance\n\
                     #[serde(skip)]\n\
                     pub cached_values: HashMap<String, f64>,\n\
                     }}\n\n\
                     impl Component for {} {{}}\n\n\
                     impl {} {{\n\
                     /// Create component from behavior pack data\n\
                     pub fn from_behavior_pack_data(data: serde_json::Value) -> Self {{\n\
                     Self {{\n\
                     data,\n\
                     cached_values: HashMap::new(),\n\
                     }}\n\
                     }}\n\
                     \n\
                     /// Get numeric value with caching for performance\n\
                     pub fn get_numeric_value(&mut self, key: &str) -> Option<f64> {{\n\
                     if let Some(cached) = self.cached_values.get(key) {{\n\
                     return Some(*cached);\n\
                     }}\n\
                     \n\
                     if let Some(value) = self.data.get(key) {{\n\
                     if let Some(num) = value.as_f64() {{\n\
                     self.cached_values.insert(key.to_string(), num);\n\
                     return Some(num);\n\
                     }}\n\
                     }}\n\
                     \n\
                     None\n\
                     }}\n\
                     }}\n\n",
                    component_name, component_struct_name, component_struct_name, component_struct_name
                ));
            }
        }
        
        // Generate Mirai trait implementations
        content.push_str(&format!("impl {} {{\n", struct_name));
        content.push_str(&format!("    /// Minecraft identifier constant\n"));
        content.push_str(&format!("    pub const IDENTIFIER: &'static str = \"{}\";\n", entity_data.identifier));
        content.push_str("    \n");
        content.push_str("    /// Create new entity with default values\n");
        content.push_str("    pub fn new() -> Self {\n");
        content.push_str("        Self {\n");
        content.push_str(&format!("            identifier: \"{}\".to_string(),\n", entity_data.identifier));
        content.push_str("            is_spawnable: true,\n");
        content.push_str("            is_summonable: true,\n");
        content.push_str("            is_experimental: false,\n");
        content.push_str("            components: HashMap::new(),\n");
        
        if self.mirai_config.ecs_enabled {
            content.push_str("            entity_id: None,\n");
        }
        
        content.push_str("        }\n");
        content.push_str("    }\n");
        
        // Add component management methods compatible with mirai's existing format
        content.push_str("    \n");
        content.push_str("    /// Add component data (compatible with mirai's existing format)\n");
        content.push_str("    pub fn add_component(&mut self, name: String, data: serde_json::Value) {\n");
        content.push_str("        self.components.insert(name, data);\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Get component data by name\n");
        content.push_str("    pub fn get_component(&self, name: &str) -> Option<&serde_json::Value> {\n");
        content.push_str("        self.components.get(name)\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Check if entity has a specific component\n");
        content.push_str("    pub fn has_component(&self, name: &str) -> bool {\n");
        content.push_str("        self.components.contains_key(name)\n");
        content.push_str("    }\n");
        
        // Add ECS integration methods
        if self.mirai_config.ecs_enabled {
            content.push_str("    \n");
            content.push_str("    /// Spawn this entity in the ECS world\n");
            content.push_str("    pub fn spawn_in_world(&mut self, world: &mut World) -> Result<Entity, Box<dyn std::error::Error>> {\n");
            content.push_str("        let entity = world.spawn();\n");
            content.push_str("        \n");
            content.push_str("        // Add the main entity component\n");
            content.push_str("        world.insert(entity, self.clone())?;\n");
            content.push_str("        \n");
            content.push_str("        // Add individual ECS components for each behavior pack component\n");
            content.push_str("        for (component_name, component_data) in &self.components {\n");
            content.push_str("            match component_name.as_str() {\n");
            
            for component_name in &entity_data.ecs_components {
                let component_struct_name = self.component_name_to_struct(component_name);
                content.push_str(&format!(
                    "                \"{}\" => {{\n\
                     let component = {}::from_behavior_pack_data(component_data.clone());\n\
                     world.insert(entity, component)?;\n\
                     }}\n",
                    component_name, component_struct_name
                ));
            }
            
            content.push_str("                _ => {\n");
            content.push_str("                    // Unknown component, skip for now\n");
            content.push_str("                }\n");
            content.push_str("            }\n");
            content.push_str("        }\n");
            content.push_str("        \n");
            content.push_str("        self.entity_id = Some(entity);\n");
            content.push_str("        Ok(entity)\n");
            content.push_str("    }\n");
            
            content.push_str("    \n");
            content.push_str("    /// Add component to existing ECS entity\n");
            content.push_str("    pub fn add_ecs_component<T: Component>(&self, world: &mut World, component: T) -> Result<(), Box<dyn std::error::Error>> {\n");
            content.push_str("        if let Some(entity_id) = self.entity_id {\n");
            content.push_str("            world.insert(entity_id, component)?;\n");
            content.push_str("            Ok(())\n");
            content.push_str("        } else {\n");
            content.push_str("            Err(\"Entity not spawned in world yet\".into())\n");
            content.push_str("        }\n");
            content.push_str("    }\n");
            
            content.push_str("    \n");
            content.push_str("    /// Remove entity from ECS world\n");
            content.push_str("    pub fn despawn_from_world(&mut self, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {\n");
            content.push_str("        if let Some(entity_id) = self.entity_id {\n");
            content.push_str("            world.despawn(entity_id)?;\n");
            content.push_str("            self.entity_id = None;\n");
            content.push_str("            Ok(())\n");
            content.push_str("        } else {\n");
            content.push_str("            Err(\"Entity not spawned in world\".into())\n");
            content.push_str("        }\n");
            content.push_str("    }\n");
        }
        
        // Add serialization compatibility methods
        content.push_str("    \n");
        content.push_str("    /// Serialize to mirai's existing format\n");
        content.push_str("    pub fn to_mirai_format(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {\n");
        content.push_str("        Ok(serde_json::to_vec(self)?)\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Deserialize from mirai's existing format\n");
        content.push_str("    pub fn from_mirai_format(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {\n");
        content.push_str("        Ok(serde_json::from_slice(data)?)\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Create a mirai-compatible entity instance\n");
        content.push_str("    pub fn create_mirai_entity() -> Self {\n");
        content.push_str("        Self::new()\n");
        content.push_str("    }\n");
        
        content.push_str("}\n\n");
        
        // Generate Default implementation
        content.push_str(&format!("impl Default for {} {{\n", struct_name));
        content.push_str("    fn default() -> Self {\n");
        content.push_str("        Self::new()\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
        
        // Generate trait for mirai compatibility
        content.push_str("/// Trait for entities compatible with mirai's existing systems\n");
        content.push_str("pub trait MiraiCompatibleEntity {\n");
        content.push_str("    /// Get the minecraft identifier\n");
        content.push_str("    fn identifier(&self) -> &str;\n");
        content.push_str("    /// Check if entity can be spawned\n");
        content.push_str("    fn is_spawnable(&self) -> bool;\n");
        content.push_str("    /// Check if entity can be summoned\n");
        content.push_str("    fn is_summonable(&self) -> bool;\n");
        content.push_str("    /// Get all components\n");
        content.push_str("    fn components(&self) -> &HashMap<String, serde_json::Value>;\n");
        content.push_str("}\n\n");
        
        content.push_str(&format!("impl MiraiCompatibleEntity for {} {{\n", struct_name));
        content.push_str("    fn identifier(&self) -> &str {\n");
        content.push_str("        &self.identifier\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn is_spawnable(&self) -> bool {\n");
        content.push_str("        self.is_spawnable\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn is_summonable(&self) -> bool {\n");
        content.push_str("        self.is_summonable\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn components(&self) -> &HashMap<String, serde_json::Value> {\n");
        content.push_str("        &self.components\n");
        content.push_str("    }\n");
        content.push_str("}\n");
        
        Ok(GeneratedCode {
            filename: format!("{}.rs", module_name),
            content,
            module_name,
        })
    }
    
    /// Generate Mirai item code
    fn generate_mirai_item_code(
        &self,
        item_data: &MiraiItemData,
        parsed_data: &ParsedData,
    ) -> Result<GeneratedCode> {
        let item_name = self.extract_entity_name(&item_data.identifier);
        let struct_name = self.to_pascal_case(&item_name);
        let module_name = self.get_module_name(&parsed_data.source_file);
        
        let mut content = String::new();
        
        content.push_str("//! Generated item definition for Mirai\n\n");
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use std::collections::HashMap;\n");
        
        if self.mirai_config.ecs_enabled {
            content.push_str("use crate::ecs::Component;\n");
        }
        
        content.push_str("\n");
        
        content.push_str(&format!("/// {} item for Mirai (compatible with existing item system)\n", struct_name));
        content.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        content.push_str(&format!("pub struct {} {{\n", struct_name));
        content.push_str("    /// Minecraft identifier for this item\n");
        content.push_str("    pub identifier: String,\n");
        content.push_str("    /// Maximum stack size (compatible with mirai's existing format)\n");
        content.push_str("    pub max_stack_size: u32,\n");
        content.push_str("    /// Item durability (-1 for no durability)\n");
        content.push_str("    pub durability: i32,\n");
        content.push_str("    /// Whether this item is stackable\n");
        content.push_str("    pub is_stackable: bool,\n");
        content.push_str("    /// Component data compatible with mirai's existing format\n");
        content.push_str("    pub components: HashMap<String, serde_json::Value>,\n");
        content.push_str("}\n\n");
        
        if self.mirai_config.ecs_enabled {
            content.push_str(&format!("impl Component for {} {{}}\n\n", struct_name));
        }
        
        content.push_str(&format!("impl {} {{\n", struct_name));
        content.push_str(&format!("    /// Minecraft identifier constant\n"));
        content.push_str(&format!("    pub const IDENTIFIER: &'static str = \"{}\";\n", item_data.identifier));
        content.push_str("    \n");
        content.push_str("    /// Create new item with default values\n");
        content.push_str("    pub fn new() -> Self {\n");
        content.push_str("        Self {\n");
        content.push_str(&format!("            identifier: \"{}\".to_string(),\n", item_data.identifier));
        content.push_str("            max_stack_size: 64,\n");
        content.push_str("            durability: -1,\n");
        content.push_str("            is_stackable: true,\n");
        content.push_str("            components: HashMap::new(),\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        
        // Add item-specific methods compatible with mirai
        content.push_str("    \n");
        content.push_str("    /// Check if item is stackable\n");
        content.push_str("    pub fn is_stackable(&self) -> bool {\n");
        content.push_str("        self.is_stackable && self.max_stack_size > 1\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Check if item has durability\n");
        content.push_str("    pub fn has_durability(&self) -> bool {\n");
        content.push_str("        self.durability > 0\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Get maximum stack size\n");
        content.push_str("    pub fn max_stack_size(&self) -> u32 {\n");
        content.push_str("        self.max_stack_size\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Add component data (compatible with mirai's existing format)\n");
        content.push_str("    pub fn add_component(&mut self, name: String, data: serde_json::Value) {\n");
        content.push_str("        self.components.insert(name, data);\n");
        content.push_str("        \n");
        content.push_str("        // Update derived properties based on components\n");
        content.push_str("        self.update_properties_from_components();\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Update item properties from component data\n");
        content.push_str("    fn update_properties_from_components(&mut self) {\n");
        content.push_str("        // Update max stack size from component\n");
        content.push_str("        if let Some(stack_component) = self.components.get(\"minecraft:max_stack_size\") {\n");
        content.push_str("            if let Some(size) = stack_component.as_u64() {\n");
        content.push_str("                self.max_stack_size = size as u32;\n");
        content.push_str("            }\n");
        content.push_str("        }\n");
        content.push_str("        \n");
        content.push_str("        // Update durability from component\n");
        content.push_str("        if let Some(durability_component) = self.components.get(\"minecraft:durability\") {\n");
        content.push_str("            if let Some(dur) = durability_component.get(\"max_durability\") {\n");
        content.push_str("                if let Some(dur_val) = dur.as_i64() {\n");
        content.push_str("                    self.durability = dur_val as i32;\n");
        content.push_str("                }\n");
        content.push_str("            }\n");
        content.push_str("        }\n");
        content.push_str("        \n");
        content.push_str("        // Update stackable property\n");
        content.push_str("        self.is_stackable = self.max_stack_size > 1 && self.durability <= 0;\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Serialize to mirai's existing format\n");
        content.push_str("    pub fn to_mirai_format(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {\n");
        content.push_str("        Ok(serde_json::to_vec(self)?)\n");
        content.push_str("    }\n");
        
        content.push_str("    \n");
        content.push_str("    /// Deserialize from mirai's existing format\n");
        content.push_str("    pub fn from_mirai_format(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {\n");
        content.push_str("        Ok(serde_json::from_slice(data)?)\n");
        content.push_str("    }\n");
        
        content.push_str("}\n\n");
        
        content.push_str(&format!("impl Default for {} {{\n", struct_name));
        content.push_str("    fn default() -> Self {\n");
        content.push_str("        Self::new()\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
        
        // Generate trait for mirai compatibility
        content.push_str("/// Trait for items compatible with mirai's existing systems\n");
        content.push_str("pub trait MiraiCompatibleItem {\n");
        content.push_str("    /// Get the minecraft identifier\n");
        content.push_str("    fn identifier(&self) -> &str;\n");
        content.push_str("    /// Get maximum stack size\n");
        content.push_str("    fn max_stack_size(&self) -> u32;\n");
        content.push_str("    /// Check if item has durability\n");
        content.push_str("    fn has_durability(&self) -> bool;\n");
        content.push_str("    /// Get all components\n");
        content.push_str("    fn components(&self) -> &HashMap<String, serde_json::Value>;\n");
        content.push_str("}\n\n");
        
        content.push_str(&format!("impl MiraiCompatibleItem for {} {{\n", struct_name));
        content.push_str("    fn identifier(&self) -> &str {\n");
        content.push_str("        &self.identifier\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn max_stack_size(&self) -> u32 {\n");
        content.push_str("        self.max_stack_size\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn has_durability(&self) -> bool {\n");
        content.push_str("        self.has_durability()\n");
        content.push_str("    }\n");
        content.push_str("    \n");
        content.push_str("    fn components(&self) -> &HashMap<String, serde_json::Value> {\n");
        content.push_str("        &self.components\n");
        content.push_str("    }\n");
        content.push_str("}\n");
        
        Ok(GeneratedCode {
            filename: format!("{}.rs", module_name),
            content,
            module_name,
        })
    }
    
    /// Generate Mirai biome code
    fn generate_mirai_biome_code(
        &self,
        biome_data: &MiraiBiomeData,
        parsed_data: &ParsedData,
    ) -> Result<GeneratedCode> {
        let biome_name = self.extract_entity_name(&biome_data.identifier);
        let struct_name = self.to_pascal_case(&biome_name);
        let module_name = self.get_module_name(&parsed_data.source_file);
        
        let mut content = String::new();
        
        content.push_str("//! Generated biome definition for Mirai\n\n");
        content.push_str("use serde::{Deserialize, Serialize};\n");
        content.push_str("use std::collections::HashMap;\n\n");
        
        content.push_str(&format!("/// {} biome for Mirai\n", struct_name));
        content.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        content.push_str(&format!("pub struct {} {{\n", struct_name));
        content.push_str("    pub identifier: String,\n");
        content.push_str("    pub components: HashMap<String, serde_json::Value>,\n");
        content.push_str("}\n\n");
        
        content.push_str(&format!("impl {} {{\n", struct_name));
        content.push_str(&format!("    pub const IDENTIFIER: &'static str = \"{}\";\n", biome_data.identifier));
        content.push_str("    \n");
        content.push_str("    pub fn new() -> Self {\n");
        content.push_str("        Self {\n");
        content.push_str(&format!("            identifier: \"{}\".to_string(),\n", biome_data.identifier));
        content.push_str("            components: HashMap::new(),\n");
        content.push_str("        }\n");
        content.push_str("    }\n");
        content.push_str("}\n\n");
        
        content.push_str(&format!("impl Default for {} {{\n", struct_name));
        content.push_str("    fn default() -> Self {\n");
        content.push_str("        Self::new()\n");
        content.push_str("    }\n");
        content.push_str("}\n");
        
        Ok(GeneratedCode {
            filename: format!("{}.rs", module_name),
            content,
            module_name,
        })
    }
    
    /// Extract entity name from identifier (remove minecraft: prefix)
    fn extract_entity_name(&self, identifier: &str) -> String {
        identifier.strip_prefix("minecraft:")
            .unwrap_or(identifier)
            .to_string()
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
    
    /// Get module name from file path
    fn get_module_name(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("generated")
            .to_lowercase()
            .replace('-', "_")
    }
    
    /// Convert component name to struct name
    fn component_name_to_struct(&self, component_name: &str) -> String {
        let clean_name = component_name
            .strip_prefix("minecraft:")
            .unwrap_or(component_name)
            .replace(":", "_");
        
        let pascal_case = self.to_pascal_case(&clean_name);
        if pascal_case.ends_with("Component") {
            pascal_case
        } else {
            format!("{}Component", pascal_case)
        }
    }
}

impl Default for MiraiIntegration {
    fn default() -> Self {
        Self {
            ecs_enabled: true,
            existing_api_compatibility: true,
            plugin_system_integration: true,
            performance_optimizations: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParsedData, DataType, DataContent};
    use std::collections::HashMap;
    use std::path::PathBuf;
    
    #[test]
    fn test_mirai_code_generator_creation() {
        let generator = MiraiCodeGenerator::new();
        assert!(generator.mirai_config.ecs_enabled);
        assert!(generator.mirai_config.existing_api_compatibility);
    }
    
    #[test]
    fn test_extract_entity_name() {
        let generator = MiraiCodeGenerator::new();
        
        assert_eq!(generator.extract_entity_name("minecraft:chicken"), "chicken");
        assert_eq!(generator.extract_entity_name("custom:entity"), "custom:entity");
        assert_eq!(generator.extract_entity_name("simple"), "simple");
    }
    
    #[test]
    fn test_to_pascal_case() {
        let generator = MiraiCodeGenerator::new();
        
        assert_eq!(generator.to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(generator.to_pascal_case("test"), "Test");
        assert_eq!(generator.to_pascal_case("multi_word_test"), "MultiWordTest");
    }
    
    #[test]
    fn test_component_name_to_struct() {
        let generator = MiraiCodeGenerator::new();
        
        assert_eq!(generator.component_name_to_struct("minecraft:health"), "HealthComponent");
        assert_eq!(generator.component_name_to_struct("minecraft:movement"), "MovementComponent");
        assert_eq!(generator.component_name_to_struct("custom:component"), "CustomComponent");
    }
    
    #[test]
    fn test_mirai_entity_code_generation() {
        let generator = MiraiCodeGenerator::new();
        
        let mut components = HashMap::new();
        components.insert("minecraft:health".to_string(), serde_json::json!({"value": 20}));
        
        let entity_data = MiraiEntityData {
            identifier: "minecraft:test_entity".to_string(),
            components,
            ecs_components: vec!["minecraft:health".to_string()],
            mirai_metadata: HashMap::new(),
        };
        
        let parsed_data = ParsedData {
            source_file: PathBuf::from("test_entity.json"),
            data_type: DataType::MiraiEntity,
            content: DataContent::MiraiEntity(entity_data),
            metadata: HashMap::new(),
        };
        
        let result = generator.generate_mirai_code(&parsed_data);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        assert!(code.content.contains("TestEntity"));
        assert!(code.content.contains("mirai_core::ecs"));
        assert!(code.content.contains("spawn_in_world"));
        assert!(code.content.contains("create_mirai_entity"));
    }
}