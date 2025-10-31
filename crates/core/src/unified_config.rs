//! Unified configuration system merging minecraft-server and mirai configurations

use crate::config::{Config as MiraiConfig, Compression, LevelConfig, MotdCallback};
use proto::bedrock::{CompressionAlgorithm, ThrottleSettings};
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddrV4, SocketAddrV6},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use util::CowString;

/// Unified configuration that merges minecraft-server and mirai configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedConfig {
    pub server: UnifiedServerSettings,
    pub network: UnifiedNetworkConfig,
    pub world: UnifiedWorldSettings,
    pub features: UnifiedFeatureFlags,
    pub mirai: MiraiCompatConfig,
}

/// Server settings combining both projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedServerSettings {
    pub server_name: String,
    pub motd: String,
    pub max_players: usize,
    pub max_connections: usize,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub max_render_distance: usize,
    pub difficulty: Difficulty,
    pub gamemode: GameMode,
    pub hardcore: bool,
    pub pvp: bool,
    pub online_mode: bool,
    pub whitelist: bool,
    pub enforce_whitelist: bool,
}

/// Network configuration merging both systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNetworkConfig {
    pub port: u16,
    pub ipv4_addr: Option<String>,
    pub ipv6_addr: Option<String>,
    pub max_clients: usize,
    pub timeout_seconds: u64,
    pub compression: CompressionConfig,
    pub throttling: ThrottlingConfig,
    pub encryption_enabled: bool,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub algorithm: String,
    pub threshold: u16,
}

/// Throttling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottlingConfig {
    pub enabled: bool,
    pub scalar: f32,
    pub threshold: u32,
}

/// World settings combining both projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedWorldSettings {
    pub world_name: String,
    pub level_path: String,
    pub seed: Option<i64>,
    pub generate_structures: bool,
    pub generator_settings: String,
    pub level_type: String,
    pub spawn_protection: i32,
    pub max_world_size: i32,
}

/// Unified feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFeatureFlags {
    pub vanilla_mobs: bool,
    pub redstone: bool,
    pub world_generation: bool,
    pub creative_mode: bool,
    pub command_system: bool,
    pub performance_monitoring: bool,
    pub ecs_system: bool,
    pub plugin_system: bool,
}

/// Mirai-specific compatibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiraiCompatConfig {
    pub enable_legacy_api: bool,
    pub preserve_existing_behavior: bool,
    pub migration_mode: bool,
}

/// Game difficulty levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

/// Game modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self {
            server: UnifiedServerSettings::default(),
            network: UnifiedNetworkConfig::default(),
            world: UnifiedWorldSettings::default(),
            features: UnifiedFeatureFlags::default(),
            mirai: MiraiCompatConfig::default(),
        }
    }
}

impl Default for UnifiedServerSettings {
    fn default() -> Self {
        Self {
            server_name: "Mirai Minecraft Server".to_string(),
            motd: "Powered by Mirai - A unified Minecraft server".to_string(),
            max_players: 10,
            max_connections: 10,
            view_distance: 10,
            simulation_distance: 10,
            max_render_distance: 12,
            difficulty: Difficulty::Normal,
            gamemode: GameMode::Survival,
            hardcore: false,
            pvp: true,
            online_mode: true,
            whitelist: false,
            enforce_whitelist: false,
        }
    }
}

impl Default for UnifiedNetworkConfig {
    fn default() -> Self {
        Self {
            port: 19132,
            ipv4_addr: Some("0.0.0.0".to_string()),
            ipv6_addr: None,
            max_clients: 100,
            timeout_seconds: 30,
            compression: CompressionConfig::default(),
            throttling: ThrottlingConfig::default(),
            encryption_enabled: true,
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: "flate".to_string(),
            threshold: 1,
        }
    }
}

impl Default for ThrottlingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scalar: 0.0,
            threshold: 0,
        }
    }
}

impl Default for UnifiedWorldSettings {
    fn default() -> Self {
        Self {
            world_name: "world".to_string(),
            level_path: "resources/level".to_string(),
            seed: None,
            generate_structures: true,
            generator_settings: "{}".to_string(),
            level_type: "default".to_string(),
            spawn_protection: 16,
            max_world_size: 29999984,
        }
    }
}

impl Default for UnifiedFeatureFlags {
    fn default() -> Self {
        Self {
            vanilla_mobs: true,
            redstone: true,
            world_generation: true,
            creative_mode: true,
            command_system: true,
            performance_monitoring: false,
            ecs_system: true,
            plugin_system: true,
        }
    }
}

impl Default for MiraiCompatConfig {
    fn default() -> Self {
        Self {
            enable_legacy_api: true,
            preserve_existing_behavior: true,
            migration_mode: false,
        }
    }
}

impl UnifiedConfig {
    /// Load configuration from file with format detection
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileNotFound(e.to_string()))?;
        
        let format = ConfigFormat::from_extension(path);
        let config: UnifiedConfig = match format {
            ConfigFormat::Json => serde_json::from_str(&content)
                .map_err(|e| ConfigError::ParseFailed(format!("JSON parse error: {}", e)))?,
            ConfigFormat::Toml => toml::from_str(&content)
                .map_err(|e| ConfigError::ParseFailed(format!("TOML parse error: {}", e)))?,
        };
        
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let format = ConfigFormat::from_extension(path);
        
        let content = match format {
            ConfigFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| ConfigError::ParseFailed(format!("JSON serialize error: {}", e)))?,
            ConfigFormat::Toml => toml::to_string_pretty(self)
                .map_err(|e| ConfigError::ParseFailed(format!("TOML serialize error: {}", e)))?,
        };
        
        std::fs::write(path, content)
            .map_err(|e| ConfigError::FileNotFound(e.to_string()))?;
        
        Ok(())
    }
    
    /// Validate the unified configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Server validation
        if self.server.server_name.is_empty() {
            return Err(ConfigError::Invalid("Server name cannot be empty".to_string()));
        }
        
        if self.server.server_name.len() > 64 {
            return Err(ConfigError::Invalid("Server name cannot exceed 64 characters".to_string()));
        }
        
        if self.server.max_players == 0 {
            return Err(ConfigError::Invalid("Max players cannot be 0".to_string()));
        }
        
        if self.server.max_connections == 0 {
            return Err(ConfigError::Invalid("Max connections cannot be 0".to_string()));
        }
        
        if self.server.max_players > self.server.max_connections {
            return Err(ConfigError::Invalid("Max players cannot exceed max connections".to_string()));
        }
        
        // Network validation
        if self.network.port == 0 {
            return Err(ConfigError::Invalid("Port cannot be 0".to_string()));
        }
        
        if self.network.max_clients == 0 {
            return Err(ConfigError::Invalid("Max clients cannot be 0".to_string()));
        }
        
        if self.network.compression.threshold == 0 {
            return Err(ConfigError::Invalid("Compression threshold cannot be 0".to_string()));
        }
        
        // World validation
        if self.world.world_name.is_empty() {
            return Err(ConfigError::Invalid("World name cannot be empty".to_string()));
        }
        
        if self.world.level_path.is_empty() {
            return Err(ConfigError::Invalid("Level path cannot be empty".to_string()));
        }
        
        if self.world.spawn_protection < 0 {
            return Err(ConfigError::Invalid("Spawn protection cannot be negative".to_string()));
        }
        
        // Cross-validation
        if self.server.hardcore && self.server.gamemode != GameMode::Survival {
            return Err(ConfigError::Invalid("Hardcore mode requires survival gamemode".to_string()));
        }
        
        Ok(())
    }
    
    /// Convert to legacy mirai config for backward compatibility
    pub fn to_mirai_config(&self) -> MiraiConfig {
        let mut config = MiraiConfig::new();
        
        // Update the internal fields through the existing setters
        config.set_max_connections(self.server.max_connections);
        config.set_max_render_distance(self.server.max_render_distance);
        
        config
    }
    
    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "vanilla_mobs" => self.features.vanilla_mobs,
            "redstone" => self.features.redstone,
            "world_generation" => self.features.world_generation,
            "creative_mode" => self.features.creative_mode,
            "command_system" => self.features.command_system,
            "performance_monitoring" => self.features.performance_monitoring,
            "ecs_system" => self.features.ecs_system,
            "plugin_system" => self.features.plugin_system,
            _ => false,
        }
    }
}

/// Configuration file format detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

impl ConfigFormat {
    pub fn from_extension(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => Self::Toml,
            Some("json") => Self::Json,
            _ => Self::Toml, // Default to TOML
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    
    #[error("Failed to parse configuration: {0}")]
    ParseFailed(String),
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_unified_config_default() {
        let config = UnifiedConfig::default();
        assert_eq!(config.server.server_name, "Mirai Minecraft Server");
        assert_eq!(config.network.port, 19132);
        assert_eq!(config.world.world_name, "world");
        assert!(config.features.ecs_system);
        assert!(config.features.plugin_system);
    }
    
    #[test]
    fn test_unified_config_validation() {
        let mut config = UnifiedConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid server name
        config.server.server_name = String::new();
        assert!(config.validate().is_err());
        
        // Reset and test invalid max players
        config.server.server_name = "Test Server".to_string();
        config.server.max_players = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_feature_checking() {
        let config = UnifiedConfig::default();
        
        assert!(config.is_feature_enabled("ecs_system"));
        assert!(config.is_feature_enabled("plugin_system"));
        assert!(!config.is_feature_enabled("unknown_feature"));
    }
    
    #[test]
    fn test_mirai_config_conversion() {
        let unified_config = UnifiedConfig::default();
        let mirai_config = unified_config.to_mirai_config();
        
        assert_eq!(mirai_config.max_connections(), unified_config.server.max_connections);
        assert_eq!(mirai_config.max_render_distance(), unified_config.server.max_render_distance);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = UnifiedConfig::default();
        
        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.is_empty());
        
        let deserialized: UnifiedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.server.server_name, deserialized.server.server_name);
        
        // Test TOML serialization
        let toml = toml::to_string(&config).unwrap();
        assert!(!toml.is_empty());
        
        let toml_deserialized: UnifiedConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.server.server_name, toml_deserialized.server.server_name);
    }
    
    #[test]
    fn test_file_operations() {
        let config = UnifiedConfig::default();
        
        // Test TOML file operations
        let toml_temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let toml_temp_path = toml_temp_file.path().to_path_buf();
        
        config.save_to_file(&toml_temp_path).unwrap();
        let loaded_config = UnifiedConfig::load_from_file(&toml_temp_path).unwrap();
        assert_eq!(config.server.server_name, loaded_config.server.server_name);
        
        // Test JSON file operations
        let mut json_temp_file = NamedTempFile::with_suffix(".json").unwrap();
        let json_temp_path = json_temp_file.path().to_path_buf();
        
        let json = serde_json::to_string_pretty(&config).unwrap();
        json_temp_file.write_all(json.as_bytes()).unwrap();
        json_temp_file.flush().unwrap();
        
        let loaded_json_config = UnifiedConfig::load_from_file(&json_temp_path).unwrap();
        assert_eq!(config.server.server_name, loaded_json_config.server.server_name);
    }
}