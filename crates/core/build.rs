use std::env;
use std::path::Path;

fn main() {
    // The original build script used `vergen` to emit git build metadata.
    // That crate's API and feature set may differ between environments; to
    // keep local developer builds reliable we emit a simple fallback value
    // for the VERGEN_GIT_DESCRIBE environment variable here. Projects that
    // rely on rich vergen output can re-enable and adapt the vergen usage.
    println!("cargo:rustc-env=VERGEN_GIT_DESCRIBE=unknown");

    // Integrate code generation
    integrate_codegen().unwrap_or_else(|e| {
        println!("cargo:warning=Code generation failed: {}", e);
    });
}

fn integrate_codegen() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir);
    
    // Watch for changes in behavior pack directories
    println!("cargo:rerun-if-changed=../../../bedrock-server/behavior_packs/");
    println!("cargo:rerun-if-changed=../../../bedrock-server/worlds/");
    println!("cargo:rerun-if-changed=../codegen/");
    
    // Generate code from behavior packs if available
    if let Err(e) = generate_mirai_code(out_path) {
        println!("cargo:warning=Mirai code generation failed: {}", e);
        // Create empty fallback files so the build doesn't fail
        create_fallback_files(out_path)?;
    }
    
    Ok(())
}

fn generate_mirai_code(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    // Create the generated code directory
    fs::create_dir_all(out_path)?;
    
    // Try to find behavior pack data
    let behavior_pack_paths = find_behavior_pack_paths()?;
    
    if behavior_pack_paths.is_empty() {
        println!("cargo:warning=No behavior pack data found, generating empty modules");
        create_fallback_files(out_path)?;
        return Ok(());
    }
    
    // Generate code from the first available behavior pack
    for behavior_pack_path in behavior_pack_paths {
        if let Ok(generated_files) = generate_from_behavior_pack(&behavior_pack_path) {
            // Write generated files to output directory
            for generated_file in generated_files {
                let file_path = out_path.join(&generated_file.filename);
                fs::write(&file_path, &generated_file.content)?;
                println!("Generated: {}", file_path.display());
            }
            return Ok(());
        }
    }
    
    // If we get here, generation failed for all behavior packs
    create_fallback_files(out_path)?;
    Ok(())
}

fn find_behavior_pack_paths() -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    use std::fs;
    
    let mut paths = Vec::new();
    
    // Look for bedrock-server directory relative to workspace root
    let workspace_root = env::var("CARGO_MANIFEST_DIR")?;
    let workspace_root = Path::new(&workspace_root)
        .parent() // crates
        .and_then(|p| p.parent()) // mirai
        .and_then(|p| p.parent()) // workspace root
        .unwrap_or(Path::new(&workspace_root));
    
    let bedrock_server_path = workspace_root.join("bedrock-server");
    if bedrock_server_path.exists() {
        let behavior_packs_path = bedrock_server_path.join("behavior_packs");
        if behavior_packs_path.exists() {
            // Add vanilla behavior pack
            let vanilla_path = behavior_packs_path.join("vanilla");
            if vanilla_path.exists() {
                paths.push(vanilla_path);
            }
            
            // Add latest versioned vanilla behavior pack
            if let Ok(entries) = fs::read_dir(&behavior_packs_path) {
                let mut vanilla_versions = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name();
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with("vanilla_") {
                                vanilla_versions.push(entry.path());
                            }
                        }
                    }
                }
                
                // Sort and take the latest version
                vanilla_versions.sort();
                if let Some(latest) = vanilla_versions.last() {
                    paths.push(latest.clone());
                }
            }
        }
    }
    
    Ok(paths)
}

fn generate_from_behavior_pack(behavior_pack_path: &Path) -> Result<Vec<GeneratedFile>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    
    // Generate entities module compatible with mirai
    files.push(GeneratedFile {
        filename: "entities.rs".to_string(),
        content: generate_mirai_entities_module(behavior_pack_path)?,
    });
    
    // Generate items module compatible with mirai
    files.push(GeneratedFile {
        filename: "items.rs".to_string(),
        content: generate_mirai_items_module(behavior_pack_path)?,
    });
    
    // Generate biomes module compatible with mirai
    files.push(GeneratedFile {
        filename: "biomes.rs".to_string(),
        content: generate_mirai_biomes_module(behavior_pack_path)?,
    });
    
    // Generate mod.rs
    files.push(GeneratedFile {
        filename: "mod.rs".to_string(),
        content: r#"/// Generated Minecraft data structures for Mirai

pub mod entities;
pub mod items;
pub mod biomes;

// Re-export for convenience
pub use entities::*;
pub use items::*;
pub use biomes::*;
"#.to_string(),
    });
    
    Ok(files)
}

fn generate_mirai_entities_module(behavior_pack_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    
    let entities_dir = behavior_pack_path.join("entities");
    let mut content = String::from(r#"//! Generated entity definitions from behavior pack for Mirai

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Mirai-compatible entity trait
pub trait MiraiEntity {
    fn identifier(&self) -> &str;
    fn is_spawnable(&self) -> bool;
    fn is_summonable(&self) -> bool;
    fn components(&self) -> &HashMap<String, serde_json::Value>;
}

"#);
    
    if entities_dir.exists() {
        let mut entity_count = 0;
        if let Ok(entries) = fs::read_dir(&entities_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            entity_count += 1;
                            if entity_count > 5 { break; } // Limit for build performance
                            
                            let entity_name = name.strip_suffix(".json").unwrap_or(name);
                            let struct_name = to_pascal_case(entity_name);
                            
                            content.push_str(&format!(r#"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub identifier: String,
    pub is_spawnable: bool,
    pub is_summonable: bool,
    pub is_experimental: bool,
    pub components: HashMap<String, serde_json::Value>,
}}

impl {} {{
    pub const IDENTIFIER: &'static str = "minecraft:{}";
    
    pub fn new() -> Self {{
        Self {{
            identifier: Self::IDENTIFIER.to_string(),
            is_spawnable: true,
            is_summonable: true,
            is_experimental: false,
            components: HashMap::new(),
        }}
    }}
}}

impl MiraiEntity for {} {{
    fn identifier(&self) -> &str {{
        &self.identifier
    }}
    
    fn is_spawnable(&self) -> bool {{
        self.is_spawnable
    }}
    
    fn is_summonable(&self) -> bool {{
        self.is_summonable
    }}
    
    fn components(&self) -> &HashMap<String, serde_json::Value> {{
        &self.components
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#, struct_name, struct_name, entity_name, struct_name, struct_name));
                        }
                    }
                }
            }
        }
    }
    
    Ok(content)
}

fn generate_mirai_items_module(behavior_pack_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    
    let items_dir = behavior_pack_path.join("items");
    let mut content = String::from(r#"//! Generated item definitions from behavior pack for Mirai

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Mirai-compatible item trait
pub trait MiraiItem {
    fn identifier(&self) -> &str;
    fn components(&self) -> &HashMap<String, serde_json::Value>;
}

"#);
    
    if items_dir.exists() {
        let mut item_count = 0;
        if let Ok(entries) = fs::read_dir(&items_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            item_count += 1;
                            if item_count > 5 { break; } // Limit for build performance
                            
                            let item_name = name.strip_suffix(".json").unwrap_or(name);
                            let struct_name = to_pascal_case(item_name);
                            
                            content.push_str(&format!(r#"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub identifier: String,
    pub components: HashMap<String, serde_json::Value>,
}}

impl {} {{
    pub const IDENTIFIER: &'static str = "minecraft:{}";
    
    pub fn new() -> Self {{
        Self {{
            identifier: Self::IDENTIFIER.to_string(),
            components: HashMap::new(),
        }}
    }}
}}

impl MiraiItem for {} {{
    fn identifier(&self) -> &str {{
        &self.identifier
    }}
    
    fn components(&self) -> &HashMap<String, serde_json::Value> {{
        &self.components
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#, struct_name, struct_name, item_name, struct_name, struct_name));
                        }
                    }
                }
            }
        }
    }
    
    Ok(content)
}

fn generate_mirai_biomes_module(behavior_pack_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::fs;
    
    let biomes_dir = behavior_pack_path.join("biomes");
    let mut content = String::from(r#"//! Generated biome definitions from behavior pack for Mirai

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Mirai-compatible biome trait
pub trait MiraiBiome {
    fn identifier(&self) -> &str;
    fn components(&self) -> &HashMap<String, serde_json::Value>;
}

"#);
    
    if biomes_dir.exists() {
        let mut biome_count = 0;
        if let Ok(entries) = fs::read_dir(&biomes_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            biome_count += 1;
                            if biome_count > 5 { break; } // Limit for build performance
                            
                            let biome_name = name.strip_suffix(".json").unwrap_or(name)
                                .strip_suffix(".biome").unwrap_or(name);
                            let struct_name = to_pascal_case(biome_name);
                            
                            content.push_str(&format!(r#"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub identifier: String,
    pub components: HashMap<String, serde_json::Value>,
}}

impl {} {{
    pub const IDENTIFIER: &'static str = "minecraft:{}";
    
    pub fn new() -> Self {{
        Self {{
            identifier: Self::IDENTIFIER.to_string(),
            components: HashMap::new(),
        }}
    }}
}}

impl MiraiBiome for {} {{
    fn identifier(&self) -> &str {{
        &self.identifier
    }}
    
    fn components(&self) -> &HashMap<String, serde_json::Value> {{
        &self.components
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#, struct_name, struct_name, biome_name, struct_name, struct_name));
                        }
                    }
                }
            }
        }
    }
    
    Ok(content)
}

fn create_fallback_files(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    // Create empty modules so the build doesn't fail
    let files = vec![
        ("entities.rs", "//! Entity definitions (empty fallback for Mirai)\n\npub trait MiraiEntity {}\n"),
        ("items.rs", "//! Item definitions (empty fallback for Mirai)\n\npub trait MiraiItem {}\n"),
        ("biomes.rs", "//! Biome definitions (empty fallback for Mirai)\n\npub trait MiraiBiome {}\n"),
        ("mod.rs", "/// Generated Minecraft data structures for Mirai (empty fallback)\n\npub mod entities;\npub mod items;\npub mod biomes;\n"),
    ];
    
    for (filename, content) in files {
        let file_path = out_path.join(filename);
        fs::write(&file_path, content)?;
    }
    
    Ok(())
}

fn to_pascal_case(s: &str) -> String {
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

#[derive(Debug)]
struct GeneratedFile {
    filename: String,
    content: String,
}
