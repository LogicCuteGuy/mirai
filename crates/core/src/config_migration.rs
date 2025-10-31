//! Configuration migration utilities for merging mirai and minecraft-server configurations

use crate::{
    config::{Config as MiraiConfig, LevelConfig},
    unified_config::{
        UnifiedConfig, UnifiedServerSettings, UnifiedNetworkConfig, UnifiedWorldSettings,
        UnifiedFeatureFlags, MiraiCompatConfig, CompressionConfig, ThrottlingConfig,
        Difficulty, GameMode, ConfigError,
    },
};
use proto::bedrock::CompressionAlgorithm;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Migration errors
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    
    #[error("Failed to parse configuration: {0}")]
    ParseFailed(String),
    
    #[error("Migration validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Unsupported configuration format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Configuration conflict: {0}")]
    ConfigConflict(String),
}

/// Legacy minecraft-server configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyServerConfig {
    pub server: LegacyServerSettings,
    pub network: LegacyNetworkConfig,
    pub world: LegacyWorldSettings,
    pub features: LegacyFeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyServerSettings {
    pub server_name: String,
    pub motd: String,
    pub max_players: usize,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub difficulty: String,
    pub gamemode: String,
    pub hardcore: bool,
    pub pvp: bool,
    pub online_mode: bool,
    pub whitelist: bool,
    pub enforce_whitelist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyNetworkConfig {
    pub port: u16,
    pub max_clients: usize,
    pub timeout_seconds: u64,
    pub compression_threshold: u16,
    pub encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyWorldSettings {
    pub world_name: String,
    pub seed: Option<i64>,
    pub generate_structures: bool,
    pub generator_settings: String,
    pub level_type: String,
    pub spawn_protection: i32,
    pub max_world_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyFeatureFlags {
    pub vanilla_mobs: bool,
    pub redstone: bool,
    pub world_generation: bool,
    pub creative_mode: bool,
    pub command_system: bool,
    pub performance_monitoring: bool,
}

/// Legacy mirai configuration representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMiraiConfig {
    pub server_name: String,
    pub motd: String,
    pub port: u16,
    pub max_connections: usize,
    pub max_render_distance: usize,
    pub level_path: String,
    pub compression_algorithm: String,
    pub compression_threshold: u16,
    pub throttling_enabled: bool,
    pub throttling_scalar: f32,
    pub throttling_threshold: u32,
}

/// Configuration migration utilities
pub struct ConfigMigrator {
    validation_enabled: bool,
    backup_enabled: bool,
}

impl ConfigMigrator {
    /// Create a new configuration migrator
    pub fn new() -> Self {
        Self {
            validation_enabled: true,
            backup_enabled: true,
        }
    }
    
    /// Create migrator with custom settings
    pub fn with_settings(validation_enabled: bool, backup_enabled: bool) -> Self {
        Self {
            validation_enabled,
            backup_enabled,
        }
    }
    
    /// Migrate legacy minecraft-server configuration to unified format
    pub fn migrate_minecraft_server_config<P: AsRef<Path>>(
        &self,
        config_path: P,
    ) -> Result<UnifiedConfig, MigrationError> {
        let config_path = config_path.as_ref();
        
        // Backup original if enabled
        if self.backup_enabled {
            self.create_backup(config_path)?;
        }
        
        // Load legacy config
        let legacy_config = self.load_legacy_server_config(config_path)?;
        
        // Convert to unified format
        let unified_config = self.convert_server_config_to_unified(legacy_config)?;
        
        // Validate if enabled
        if self.validation_enabled {
            unified_config.validate()
                .map_err(|e| MigrationError::ValidationFailed(e.to_string()))?;
        }
        
        Ok(unified_config)
    }
    
    /// Migrate legacy mirai configuration to unified format
    pub fn migrate_mirai_config(&self, mirai_config: &MiraiConfig) -> Result<UnifiedConfig, MigrationError> {
        let legacy_mirai = self.extract_mirai_config(mirai_config);
        let unified_config = self.convert_mirai_config_to_unified(legacy_mirai)?;
        
        // Validate if enabled
        if self.validation_enabled {
            unified_config.validate()
                .map_err(|e| MigrationError::ValidationFailed(e.to_string()))?;
        }
        
        Ok(unified_config)
    }
    
    /// Merge two configurations with conflict resolution
    pub fn merge_configurations(
        &self,
        mirai_config: UnifiedConfig,
        server_config: UnifiedConfig,
    ) -> Result<UnifiedConfig, MigrationError> {
        let mut merged = UnifiedConfig::default();
        
        // Server settings - prefer mirai for network, server for gameplay
        merged.server = UnifiedServerSettings {
            server_name: if !mirai_config.server.server_name.is_empty() {
                mirai_config.server.server_name
            } else {
                server_config.server.server_name
            },
            motd: if !server_config.server.motd.is_empty() {
                server_config.server.motd
            } else {
                mirai_config.server.motd
            },
            max_players: server_config.server.max_players,
            max_connections: mirai_config.server.max_connections,
            view_distance: server_config.server.view_distance,
            simulation_distance: server_config.server.simulation_distance,
            max_render_distance: mirai_config.server.max_render_distance,
            difficulty: server_config.server.difficulty,
            gamemode: server_config.server.gamemode,
            hardcore: server_config.server.hardcore,
            pvp: server_config.server.pvp,
            online_mode: server_config.server.online_mode,
            whitelist: server_config.server.whitelist,
            enforce_whitelist: server_config.server.enforce_whitelist,
        };
        
        // Network settings - prefer mirai's proven network stack
        merged.network = UnifiedNetworkConfig {
            port: mirai_config.network.port,
            ipv4_addr: mirai_config.network.ipv4_addr,
            ipv6_addr: mirai_config.network.ipv6_addr,
            max_clients: std::cmp::max(mirai_config.network.max_clients, server_config.network.max_clients),
            timeout_seconds: server_config.network.timeout_seconds,
            compression: mirai_config.network.compression,
            throttling: mirai_config.network.throttling,
            encryption_enabled: server_config.network.encryption_enabled,
        };
        
        // World settings - prefer server config for world generation
        merged.world = UnifiedWorldSettings {
            world_name: server_config.world.world_name,
            level_path: mirai_config.world.level_path,
            seed: server_config.world.seed,
            generate_structures: server_config.world.generate_structures,
            generator_settings: server_config.world.generator_settings,
            level_type: server_config.world.level_type,
            spawn_protection: server_config.world.spawn_protection,
            max_world_size: server_config.world.max_world_size,
        };
        
        // Features - merge both sets
        merged.features = UnifiedFeatureFlags {
            vanilla_mobs: server_config.features.vanilla_mobs,
            redstone: server_config.features.redstone,
            world_generation: server_config.features.world_generation,
            creative_mode: server_config.features.creative_mode,
            command_system: server_config.features.command_system,
            performance_monitoring: server_config.features.performance_monitoring,
            ecs_system: true, // Always enable in unified system
            plugin_system: true, // Always enable in unified system
        };
        
        // Mirai compatibility settings
        merged.mirai = MiraiCompatConfig {
            enable_legacy_api: true,
            preserve_existing_behavior: true,
            migration_mode: true,
        };
        
        // Validate merged config
        if self.validation_enabled {
            merged.validate()
                .map_err(|e| MigrationError::ValidationFailed(e.to_string()))?;
        }
        
        Ok(merged)
    }
    
    /// Create a migration report
    pub fn create_migration_report(
        &self,
        original_mirai: &MiraiConfig,
        original_server: Option<&LegacyServerConfig>,
        unified: &UnifiedConfig,
    ) -> MigrationReport {
        let mut report = MigrationReport::new();
        
        // Compare mirai settings
        report.add_change(
            "server.max_connections".to_string(),
            original_mirai.max_connections().to_string(),
            unified.server.max_connections.to_string(),
        );
        
        report.add_change(
            "server.max_render_distance".to_string(),
            original_mirai.max_render_distance().to_string(),
            unified.server.max_render_distance.to_string(),
        );
        
        report.add_change(
            "network.port".to_string(),
            original_mirai.ipv4_addr().port().to_string(),
            unified.network.port.to_string(),
        );
        
        // Compare server settings if available
        if let Some(server_config) = original_server {
            report.add_change(
                "server.server_name".to_string(),
                server_config.server.server_name.clone(),
                unified.server.server_name.clone(),
            );
            
            report.add_change(
                "features.vanilla_mobs".to_string(),
                server_config.features.vanilla_mobs.to_string(),
                unified.features.vanilla_mobs.to_string(),
            );
        }
        
        // Add new features
        report.add_new_feature("ecs_system".to_string(), "Entity Component System architecture".to_string());
        report.add_new_feature("plugin_system".to_string(), "Plugin architecture and management".to_string());
        
        report
    }
    
    /// Validate a unified configuration for migration compatibility
    pub fn validate_migration(&self, config: &UnifiedConfig) -> Result<(), MigrationError> {
        // Basic validation
        config.validate()
            .map_err(|e| MigrationError::ValidationFailed(e.to_string()))?;
        
        // Migration-specific validation
        if config.server.max_players > config.server.max_connections {
            return Err(MigrationError::ValidationFailed(
                "Max players cannot exceed max connections after migration".to_string()
            ));
        }
        
        if config.network.port == 0 {
            return Err(MigrationError::ValidationFailed(
                "Network port must be specified after migration".to_string()
            ));
        }
        
        if config.world.level_path.is_empty() {
            return Err(MigrationError::ValidationFailed(
                "Level path must be specified after migration".to_string()
            ));
        }
        
        // Check feature dependencies
        if config.features.vanilla_mobs && !config.features.world_generation {
            return Err(MigrationError::ValidationFailed(
                "Vanilla mobs require world generation to be enabled".to_string()
            ));
        }
        
        if config.features.plugin_system && !config.features.ecs_system {
            return Err(MigrationError::ValidationFailed(
                "Plugin system requires ECS system to be enabled".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn load_legacy_server_config<P: AsRef<Path>>(&self, path: P) -> Result<LegacyServerConfig, MigrationError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| MigrationError::FileNotFound(e.to_string()))?;
        
        // Try TOML first, then JSON
        if let Ok(config) = toml::from_str::<LegacyServerConfig>(&content) {
            Ok(config)
        } else if let Ok(config) = serde_json::from_str::<LegacyServerConfig>(&content) {
            Ok(config)
        } else {
            Err(MigrationError::ParseFailed("Could not parse as TOML or JSON".to_string()))
        }
    }
    
    fn extract_mirai_config(&self, mirai_config: &MiraiConfig) -> LegacyMiraiConfig {
        LegacyMiraiConfig {
            server_name: mirai_config.name().to_string(),
            motd: "Powered by Mirai".to_string(), // Default MOTD
            port: mirai_config.ipv4_addr().port(),
            max_connections: mirai_config.max_connections(),
            max_render_distance: mirai_config.max_render_distance(),
            level_path: mirai_config.level().path.clone(),
            compression_algorithm: match mirai_config.compression().algorithm {
                CompressionAlgorithm::Flate => "flate".to_string(),
                CompressionAlgorithm::Snappy => "snappy".to_string(),
            },
            compression_threshold: mirai_config.compression().threshold,
            throttling_enabled: mirai_config.throttling().enabled,
            throttling_scalar: mirai_config.throttling().scalar,
            throttling_threshold: mirai_config.throttling().threshold as u32,
        }
    }
    
    fn convert_server_config_to_unified(&self, legacy: LegacyServerConfig) -> Result<UnifiedConfig, MigrationError> {
        let difficulty = match legacy.server.difficulty.to_lowercase().as_str() {
            "peaceful" => Difficulty::Peaceful,
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            "hard" => Difficulty::Hard,
            _ => Difficulty::Normal,
        };
        
        let gamemode = match legacy.server.gamemode.to_lowercase().as_str() {
            "survival" => GameMode::Survival,
            "creative" => GameMode::Creative,
            "adventure" => GameMode::Adventure,
            "spectator" => GameMode::Spectator,
            _ => GameMode::Survival,
        };
        
        Ok(UnifiedConfig {
            server: UnifiedServerSettings {
                server_name: legacy.server.server_name,
                motd: legacy.server.motd,
                max_players: legacy.server.max_players,
                max_connections: legacy.network.max_clients,
                view_distance: legacy.server.view_distance,
                simulation_distance: legacy.server.simulation_distance,
                max_render_distance: 12, // Default value
                difficulty,
                gamemode,
                hardcore: legacy.server.hardcore,
                pvp: legacy.server.pvp,
                online_mode: legacy.server.online_mode,
                whitelist: legacy.server.whitelist,
                enforce_whitelist: legacy.server.enforce_whitelist,
            },
            network: UnifiedNetworkConfig {
                port: legacy.network.port,
                ipv4_addr: Some("0.0.0.0".to_string()),
                ipv6_addr: None,
                max_clients: legacy.network.max_clients,
                timeout_seconds: legacy.network.timeout_seconds,
                compression: CompressionConfig {
                    algorithm: "flate".to_string(),
                    threshold: legacy.network.compression_threshold,
                },
                throttling: ThrottlingConfig {
                    enabled: false,
                    scalar: 0.0,
                    threshold: 0,
                },
                encryption_enabled: legacy.network.encryption_enabled,
            },
            world: UnifiedWorldSettings {
                world_name: legacy.world.world_name,
                level_path: "resources/level".to_string(), // Default mirai path
                seed: legacy.world.seed,
                generate_structures: legacy.world.generate_structures,
                generator_settings: legacy.world.generator_settings,
                level_type: legacy.world.level_type,
                spawn_protection: legacy.world.spawn_protection,
                max_world_size: legacy.world.max_world_size,
            },
            features: UnifiedFeatureFlags {
                vanilla_mobs: legacy.features.vanilla_mobs,
                redstone: legacy.features.redstone,
                world_generation: legacy.features.world_generation,
                creative_mode: legacy.features.creative_mode,
                command_system: legacy.features.command_system,
                performance_monitoring: legacy.features.performance_monitoring,
                ecs_system: true,
                plugin_system: true,
            },
            mirai: MiraiCompatConfig {
                enable_legacy_api: true,
                preserve_existing_behavior: true,
                migration_mode: true,
            },
        })
    }
    
    fn convert_mirai_config_to_unified(&self, legacy: LegacyMiraiConfig) -> Result<UnifiedConfig, MigrationError> {
        Ok(UnifiedConfig {
            server: UnifiedServerSettings {
                server_name: legacy.server_name,
                motd: legacy.motd,
                max_players: legacy.max_connections, // Use max_connections as max_players to avoid validation error
                max_connections: legacy.max_connections,
                view_distance: 10, // Default value
                simulation_distance: 10, // Default value
                max_render_distance: legacy.max_render_distance,
                difficulty: Difficulty::Normal,
                gamemode: GameMode::Survival,
                hardcore: false,
                pvp: true,
                online_mode: true,
                whitelist: false,
                enforce_whitelist: false,
            },
            network: UnifiedNetworkConfig {
                port: legacy.port,
                ipv4_addr: Some("0.0.0.0".to_string()),
                ipv6_addr: None,
                max_clients: legacy.max_connections,
                timeout_seconds: 30,
                compression: CompressionConfig {
                    algorithm: legacy.compression_algorithm,
                    threshold: legacy.compression_threshold,
                },
                throttling: ThrottlingConfig {
                    enabled: legacy.throttling_enabled,
                    scalar: legacy.throttling_scalar,
                    threshold: legacy.throttling_threshold,
                },
                encryption_enabled: true,
            },
            world: UnifiedWorldSettings {
                world_name: "world".to_string(),
                level_path: legacy.level_path,
                seed: None,
                generate_structures: true,
                generator_settings: "{}".to_string(),
                level_type: "default".to_string(),
                spawn_protection: 16,
                max_world_size: 29999984,
            },
            features: UnifiedFeatureFlags {
                vanilla_mobs: false, // Conservative default
                redstone: false, // Conservative default
                world_generation: false, // Conservative default
                creative_mode: false, // Conservative default
                command_system: false, // Conservative default
                performance_monitoring: false, // Conservative default
                ecs_system: true,
                plugin_system: true,
            },
            mirai: MiraiCompatConfig {
                enable_legacy_api: true,
                preserve_existing_behavior: true,
                migration_mode: true,
            },
        })
    }
    
    fn create_backup<P: AsRef<Path>>(&self, original_path: P) -> Result<(), MigrationError> {
        let original_path = original_path.as_ref();
        let backup_path = original_path.with_extension(
            format!("{}.backup", original_path.extension().unwrap_or_default().to_string_lossy())
        );
        
        std::fs::copy(original_path, backup_path)
            .map_err(|e| MigrationError::FileNotFound(format!("Failed to create backup: {}", e)))?;
        
        Ok(())
    }
}

impl Default for ConfigMigrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Migration report for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub changes: Vec<ConfigChange>,
    pub new_features: Vec<NewFeature>,
    pub warnings: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFeature {
    pub name: String,
    pub description: String,
}

impl MigrationReport {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            new_features: Vec::new(),
            warnings: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn add_change(&mut self, key: String, old_value: String, new_value: String) {
        self.changes.push(ConfigChange { key, old_value, new_value });
    }
    
    pub fn add_new_feature(&mut self, name: String, description: String) {
        self.new_features.push(NewFeature { name, description });
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), MigrationError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| MigrationError::ParseFailed(e.to_string()))?;
        
        std::fs::write(path, content)
            .map_err(|e| MigrationError::FileNotFound(e.to_string()))?;
        
        Ok(())
    }
}

impl Default for MigrationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_legacy_server_config_migration() {
        let legacy_config = LegacyServerConfig {
            server: LegacyServerSettings {
                server_name: "Test Server".to_string(),
                motd: "Test MOTD".to_string(),
                max_players: 50,
                view_distance: 12,
                simulation_distance: 8,
                difficulty: "Hard".to_string(),
                gamemode: "Creative".to_string(),
                hardcore: false,
                pvp: true,
                online_mode: true,
                whitelist: false,
                enforce_whitelist: false,
            },
            network: LegacyNetworkConfig {
                port: 25565,
                max_clients: 100,
                timeout_seconds: 30,
                compression_threshold: 256,
                encryption_enabled: true,
            },
            world: LegacyWorldSettings {
                world_name: "test_world".to_string(),
                seed: Some(12345),
                generate_structures: true,
                generator_settings: "{}".to_string(),
                level_type: "default".to_string(),
                spawn_protection: 16,
                max_world_size: 29999984,
            },
            features: LegacyFeatureFlags {
                vanilla_mobs: true,
                redstone: true,
                world_generation: true,
                creative_mode: true,
                command_system: true,
                performance_monitoring: false,
            },
        };
        
        let migrator = ConfigMigrator::new();
        let unified = migrator.convert_server_config_to_unified(legacy_config).unwrap();
        
        assert_eq!(unified.server.server_name, "Test Server");
        assert_eq!(unified.server.difficulty, Difficulty::Hard);
        assert_eq!(unified.server.gamemode, GameMode::Creative);
        assert_eq!(unified.network.port, 25565);
        assert!(unified.features.ecs_system);
        assert!(unified.features.plugin_system);
    }
    
    #[test]
    fn test_mirai_config_migration() {
        let mirai_config = MiraiConfig::new();
        let migrator = ConfigMigrator::new();
        let unified = migrator.migrate_mirai_config(&mirai_config).unwrap();
        
        assert_eq!(unified.server.server_name, "Mirai server");
        assert_eq!(unified.network.port, 19132);
        assert!(unified.features.ecs_system);
        assert!(unified.features.plugin_system);
        assert!(unified.mirai.enable_legacy_api);
    }
    
    #[test]
    fn test_config_merging() {
        let mirai_config = UnifiedConfig {
            server: UnifiedServerSettings {
                server_name: "Mirai Server".to_string(),
                max_connections: 100,
                max_render_distance: 16,
                ..Default::default()
            },
            network: UnifiedNetworkConfig {
                port: 19132,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let server_config = UnifiedConfig {
            server: UnifiedServerSettings {
                server_name: "Minecraft Server".to_string(),
                max_players: 100,
                difficulty: Difficulty::Hard,
                ..Default::default()
            },
            features: UnifiedFeatureFlags {
                vanilla_mobs: true,
                redstone: true,
                world_generation: true,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let migrator = ConfigMigrator::new();
        let merged = migrator.merge_configurations(mirai_config, server_config).unwrap();
        
        // Should prefer mirai for network settings
        assert_eq!(merged.network.port, 19132);
        assert_eq!(merged.server.max_connections, 100);
        
        // Should prefer server for gameplay settings
        assert_eq!(merged.server.max_players, 100);
        assert_eq!(merged.server.difficulty, Difficulty::Hard);
        assert!(merged.features.vanilla_mobs);
        
        // Should always enable unified features
        assert!(merged.features.ecs_system);
        assert!(merged.features.plugin_system);
    }
    
    #[test]
    fn test_migration_validation() {
        let mut config = UnifiedConfig::default();
        let migrator = ConfigMigrator::new();
        
        // Valid config should pass
        assert!(migrator.validate_migration(&config).is_ok());
        
        // Invalid config should fail
        config.server.max_players = 1000;
        config.server.max_connections = 10;
        assert!(migrator.validate_migration(&config).is_err());
    }
    
    #[test]
    fn test_migration_report() {
        let mirai_config = MiraiConfig::new();
        let unified_config = UnifiedConfig::default();
        let migrator = ConfigMigrator::new();
        
        let report = migrator.create_migration_report(&mirai_config, None, &unified_config);
        
        assert!(!report.changes.is_empty());
        assert!(!report.new_features.is_empty());
        assert!(!report.timestamp.is_empty());
    }
}