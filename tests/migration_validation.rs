//! Migration testing and validation
//! 
//! This module contains comprehensive tests for validating migration functionality
//! from standard Mirai configurations to the unified server system.

use mirai_core::{
    config_migration::{ConfigMigrator, LegacyServerConfig, LegacyMiraiConfig, MigrationError},
    unified_config::{UnifiedConfig, Difficulty, GameMode},
    config::Config as MiraiConfig,
};
use mirai_level::{
    leveldb_compatibility::{LevelDbCompatibilityManager, ChunkFormatVersion, MigrationReport},
    world::{EnhancedChunk, ChunkPos, ChunkState},
    provider::Provider,
};
use mirai_proto::unified_auth::{
    UnifiedAuthService, UnifiedAuthConfig, UnifiedAuthData, ProtocolType,
    JavaAuthConfig, BedrockAuthConfig,
};
use proto::types::Dimension;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[cfg(test)]
mod config_migration_tests {
    use super::*;

    #[test]
    fn test_config_migrator_creation() {
        let migrator = ConfigMigrator::new();
        assert!(migrator.validation_enabled);
        assert!(migrator.backup_enabled);
        
        let custom_migrator = ConfigMigrator::with_settings(false, false);
        assert!(!custom_migrator.validation_enabled);
        assert!(!custom_migrator.backup_enabled);
    }

    #[test]
    fn test_legacy_server_config_migration() {
        let legacy_config = create_test_legacy_server_config();
        let migrator = ConfigMigrator::new();
        
        let result = migrator.convert_server_config_to_unified(legacy_config.clone());
        assert!(result.is_ok());
        
        let unified = result.unwrap();
        assert_eq!(unified.server.server_name, legacy_config.server.server_name);
        assert_eq!(unified.server.max_players, legacy_config.server.max_players);
        assert_eq!(unified.network.port, legacy_config.network.port);
        assert_eq!(unified.world.world_name, legacy_config.world.world_name);
        assert!(unified.features.ecs_system);
        assert!(unified.features.plugin_system);
    }

    #[test]
    fn test_mirai_config_migration() {
        let mirai_config = MiraiConfig::new();
        let migrator = ConfigMigrator::new();
        
        let result = migrator.migrate_mirai_config(&mirai_config);
        assert!(result.is_ok());
        
        let unified = result.unwrap();
        assert_eq!(unified.server.server_name, "Mirai server");
        assert_eq!(unified.network.port, 19132);
        assert!(unified.features.ecs_system);
        assert!(unified.features.plugin_system);
        assert!(unified.mirai.enable_legacy_api);
    }

    #[test]
    fn test_config_merging() {
        let mirai_config = create_test_mirai_unified_config();
        let server_config = create_test_server_unified_config();
        let migrator = ConfigMigrator::new();
        
        let result = migrator.merge_configurations(mirai_config.clone(), server_config.clone());
        assert!(result.is_ok());
        
        let merged = result.unwrap();
        
        // Should prefer mirai for network settings
        assert_eq!(merged.network.port, mirai_config.network.port);
        assert_eq!(merged.server.max_connections, mirai_config.server.max_connections);
        
        // Should prefer server for gameplay settings
        assert_eq!(merged.server.max_players, server_config.server.max_players);
        assert_eq!(merged.server.difficulty, server_config.server.difficulty);
        assert_eq!(merged.features.vanilla_mobs, server_config.features.vanilla_mobs);
        
        // Should always enable unified features
        assert!(merged.features.ecs_system);
        assert!(merged.features.plugin_system);
    }

    #[test]
    fn test_migration_validation() {
        let migrator = ConfigMigrator::new();
        let mut config = UnifiedConfig::default();
        
        // Valid config should pass
        assert!(migrator.validate_migration(&config).is_ok());
        
        // Invalid config: max_players > max_connections
        config.server.max_players = 1000;
        config.server.max_connections = 10;
        let result = migrator.validate_migration(&config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationError::ValidationFailed(_)));
        
        // Fix the config
        config.server.max_connections = 1000;
        assert!(migrator.validate_migration(&config).is_ok());
        
        // Invalid config: zero port
        config.network.port = 0;
        let result = migrator.validate_migration(&config);
        assert!(result.is_err());
        
        // Invalid config: empty level path
        config.network.port = 19132;
        config.world.level_path = String::new();
        let result = migrator.validate_migration(&config);
        assert!(result.is_err());
        
        // Invalid config: feature dependencies
        config.world.level_path = "world".to_string();
        config.features.vanilla_mobs = true;
        config.features.world_generation = false;
        let result = migrator.validate_migration(&config);
        assert!(result.is_err());
        
        // Invalid config: plugin system without ECS
        config.features.world_generation = true;
        config.features.plugin_system = true;
        config.features.ecs_system = false;
        let result = migrator.validate_migration(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_report_generation() {
        let mirai_config = MiraiConfig::new();
        let legacy_server = create_test_legacy_server_config();
        let unified_config = UnifiedConfig::default();
        let migrator = ConfigMigrator::new();
        
        let report = migrator.create_migration_report(
            &mirai_config,
            Some(&legacy_server),
            &unified_config
        );
        
        assert!(!report.changes.is_empty());
        assert!(!report.new_features.is_empty());
        assert!(!report.timestamp.is_empty());
        
        // Check for expected new features
        let feature_names: Vec<&str> = report.new_features.iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(feature_names.contains(&"ecs_system"));
        assert!(feature_names.contains(&"plugin_system"));
    }

    #[test]
    fn test_config_file_migration() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("server_config.toml");
        
        // Create test config file
        let legacy_config = create_test_legacy_server_config();
        let config_content = toml::to_string(&legacy_config)
            .expect("Failed to serialize config");
        std::fs::write(&config_path, config_content)
            .expect("Failed to write config file");
        
        let migrator = ConfigMigrator::new();
        let result = migrator.migrate_minecraft_server_config(&config_path);
        assert!(result.is_ok());
        
        let unified = result.unwrap();
        assert_eq!(unified.server.server_name, legacy_config.server.server_name);
        
        // Check that backup was created
        let backup_path = config_path.with_extension("toml.backup");
        assert!(backup_path.exists());
    }

    #[test]
    fn test_difficulty_conversion() {
        let migrator = ConfigMigrator::new();
        
        let test_cases = vec![
            ("peaceful", Difficulty::Peaceful),
            ("easy", Difficulty::Easy),
            ("normal", Difficulty::Normal),
            ("hard", Difficulty::Hard),
            ("invalid", Difficulty::Normal), // Should default to Normal
        ];
        
        for (input, expected) in test_cases {
            let mut legacy_config = create_test_legacy_server_config();
            legacy_config.server.difficulty = input.to_string();
            
            let unified = migrator.convert_server_config_to_unified(legacy_config)
                .expect("Failed to convert config");
            assert_eq!(unified.server.difficulty, expected);
        }
    }

    #[test]
    fn test_gamemode_conversion() {
        let migrator = ConfigMigrator::new();
        
        let test_cases = vec![
            ("survival", GameMode::Survival),
            ("creative", GameMode::Creative),
            ("adventure", GameMode::Adventure),
            ("spectator", GameMode::Spectator),
            ("invalid", GameMode::Survival), // Should default to Survival
        ];
        
        for (input, expected) in test_cases {
            let mut legacy_config = create_test_legacy_server_config();
            legacy_config.server.gamemode = input.to_string();
            
            let unified = migrator.convert_server_config_to_unified(legacy_config)
                .expect("Failed to convert config");
            assert_eq!(unified.server.gamemode, expected);
        }
    }

    fn create_test_legacy_server_config() -> LegacyServerConfig {
        use mirai_core::config_migration::{
            LegacyServerSettings, LegacyNetworkConfig, LegacyWorldSettings, LegacyFeatureFlags
        };
        
        LegacyServerConfig {
            server: LegacyServerSettings {
                server_name: "Test Server".to_string(),
                motd: "Test MOTD".to_string(),
                max_players: 50,
                view_distance: 12,
                simulation_distance: 8,
                difficulty: "hard".to_string(),
                gamemode: "creative".to_string(),
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
        }
    }

    fn create_test_mirai_unified_config() -> UnifiedConfig {
        use mirai_core::unified_config::{
            UnifiedServerSettings, UnifiedNetworkConfig, UnifiedWorldSettings,
            UnifiedFeatureFlags, MiraiCompatConfig, CompressionConfig, ThrottlingConfig
        };
        
        UnifiedConfig {
            server: UnifiedServerSettings {
                server_name: "Mirai Server".to_string(),
                max_connections: 100,
                max_render_distance: 16,
                ..Default::default()
            },
            network: UnifiedNetworkConfig {
                port: 19132,
                compression: CompressionConfig {
                    algorithm: "flate".to_string(),
                    threshold: 256,
                },
                throttling: ThrottlingConfig {
                    enabled: true,
                    scalar: 0.5,
                    threshold: 1000,
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_test_server_unified_config() -> UnifiedConfig {
        use mirai_core::unified_config::{
            UnifiedServerSettings, UnifiedFeatureFlags
        };
        
        UnifiedConfig {
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
        }
    }
}

#[cfg(test)]
mod leveldb_compatibility_tests {
    use super::*;

    fn create_test_provider() -> (Arc<Provider>, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("db");
        std::fs::create_dir_all(&db_path).expect("Failed to create db directory");
        
        // Create a minimal level.dat file for testing
        let level_dat_path = temp_dir.path().join("level.dat");
        let minimal_level_data = vec![
            8, 0, 0, 0,  // version
            100, 0, 0, 0, // size
        ];
        let mut level_data = minimal_level_data;
        level_data.extend_from_slice(&[0; 96]); // Padding to match size
        std::fs::write(&level_dat_path, level_data).expect("Failed to write level.dat");
        
        let provider = Provider::open(temp_dir.path()).expect("Failed to open provider");
        (Arc::new(provider), temp_dir)
    }

    #[test]
    fn test_leveldb_compatibility_manager_creation() {
        let (provider, _temp_dir) = create_test_provider();
        let manager = LevelDbCompatibilityManager::new(provider.clone());
        
        assert_eq!(Arc::ptr_eq(&manager.provider(), &provider), true);
    }

    #[test]
    fn test_chunk_format_version_detection() {
        let (provider, _temp_dir) = create_test_provider();
        let format_version = ChunkFormatVersion::detect_from_provider(&provider);
        
        // Should detect as Mirai format since no enhanced metadata exists
        assert_eq!(format_version, ChunkFormatVersion::Mirai);
    }

    #[test]
    fn test_enhanced_chunk_loading_fallback() {
        let (provider, _temp_dir) = create_test_provider();
        let manager = LevelDbCompatibilityManager::new(provider);
        
        let pos = ChunkPos::new(0, 0);
        let dimension = Dimension::Overworld;
        
        // Should return None for non-existent chunk
        let result = manager.load_enhanced_chunk(pos, dimension);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_enhanced_chunk_save_and_load() {
        let (provider, _temp_dir) = create_test_provider();
        let manager = LevelDbCompatibilityManager::new(provider);
        
        let pos = ChunkPos::new(1, 1);
        let dimension = Dimension::Overworld;
        let mut chunk = EnhancedChunk::new(pos, dimension);
        
        // Modify chunk data
        chunk.set_biome(0, 0, 42);
        chunk.set_height(0, 0, 128);
        chunk.mark_dirty();
        chunk.state = ChunkState::Loaded;
        
        // Save chunk
        let save_result = manager.save_enhanced_chunk(&chunk);
        assert!(save_result.is_ok());
        
        // Load chunk back
        let load_result = manager.load_enhanced_chunk(pos, dimension);
        assert!(load_result.is_ok());
        
        if let Some(loaded_chunk) = load_result.unwrap() {
            assert_eq!(loaded_chunk.pos, pos);
            assert_eq!(loaded_chunk.dimension, dimension);
            assert_eq!(loaded_chunk.get_biome(0, 0), 42);
            assert_eq!(loaded_chunk.get_height(0, 0), 128);
        }
    }

    #[test]
    fn test_world_metadata_operations() {
        let (provider, _temp_dir) = create_test_provider();
        let mut manager = LevelDbCompatibilityManager::new(provider);
        
        // Load metadata (should create default)
        let metadata_result = manager.load_world_metadata();
        assert!(metadata_result.is_ok());
        assert!(metadata_result.unwrap().is_some());
        
        // Create test world and save metadata
        use mirai_level::world::{EnhancedGameWorld, WorldMetadata, Position, WorldBorder, GameRules, GenerationSettings};
        
        let world = EnhancedGameWorld {
            id: Uuid::new_v4(),
            name: "TestWorld".to_string(),
            dimension: Dimension::Overworld,
            spawn_point: Position::new(0.0, 64.0, 0.0),
            world_border: WorldBorder::default(),
            game_rules: GameRules::default(),
            generation_settings: GenerationSettings::default(),
            metadata: WorldMetadata {
                created_at: std::time::SystemTime::now(),
                last_played: std::time::SystemTime::now(),
                version: "1.0.0".to_string(),
            },
        };
        
        let save_result = manager.save_world_metadata(&world);
        assert!(save_result.is_ok());
        
        // Load metadata again
        let loaded_metadata = manager.load_world_metadata();
        assert!(loaded_metadata.is_ok());
        assert!(loaded_metadata.unwrap().is_some());
    }

    #[test]
    fn test_migration_report() {
        let (provider, _temp_dir) = create_test_provider();
        let manager = LevelDbCompatibilityManager::new(provider);
        
        // Run migration (should handle empty database gracefully)
        let migration_result = manager.migrate_to_enhanced_format();
        assert!(migration_result.is_ok());
        
        let report = migration_result.unwrap();
        assert_eq!(report.total_chunks, 0);
        assert_eq!(report.migrated_chunks, 0);
        assert_eq!(report.failed_chunks, 0);
    }

    #[test]
    fn test_data_integrity_validation() {
        let (provider, _temp_dir) = create_test_provider();
        let manager = LevelDbCompatibilityManager::new(provider);
        
        let pos = ChunkPos::new(0, 0);
        let dimension = Dimension::Overworld;
        
        // Validate non-existent chunk
        let integrity_result = manager.validate_data_integrity(pos, dimension);
        assert!(integrity_result.is_ok());
        
        let report = integrity_result.unwrap();
        assert!(!report.both_formats_exist);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn test_custom_key_operations() {
        use mirai_level::leveldb_compatibility::{create_custom_key, is_custom_enhanced_key, get_custom_key_type};
        use util::Vector;
        
        let coordinates = Vector::from([10, 20]);
        let dimension = Dimension::Overworld;
        
        let metadata_key = create_custom_key(coordinates, dimension, "enhanced_metadata");
        assert!(is_custom_enhanced_key(&metadata_key));
        assert_eq!(get_custom_key_type(&metadata_key), Some("enhanced_metadata"));
        
        let world_key = create_custom_key(coordinates, dimension, "world_metadata");
        assert!(is_custom_enhanced_key(&world_key));
        assert_eq!(get_custom_key_type(&world_key), Some("world_metadata"));
        
        let generic_key = create_custom_key(coordinates, dimension, "unknown");
        assert!(is_custom_enhanced_key(&generic_key));
        assert_eq!(get_custom_key_type(&generic_key), Some("generic_custom"));
    }
}

#[cfg(test)]
mod plugin_compatibility_tests {
    use super::*;
    use mirai_core::{
        plugin::{Plugin, PluginRegistry, PluginMetadata, PluginError},
        App, Instance,
    };

    struct TestLegacyPlugin {
        name: String,
        enabled: bool,
    }

    impl TestLegacyPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                enabled: false,
            }
        }
    }

    impl Plugin for TestLegacyPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn on_enable(&mut self, _instance: &Instance) -> Result<(), PluginError> {
            self.enabled = true;
            Ok(())
        }

        fn on_disable(&mut self) -> Result<(), PluginError> {
            self.enabled = false;
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.plugin_count(), 0);
        assert!(registry.list_plugins().is_empty());
    }

    #[test]
    fn test_legacy_plugin_registration() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestLegacyPlugin::new("test_plugin"));
        
        let result = registry.register_plugin(plugin);
        assert!(result.is_ok());
        assert_eq!(registry.plugin_count(), 1);
        assert!(registry.is_plugin_registered("test_plugin"));
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestLegacyPlugin::new("lifecycle_test"));
        
        registry.register_plugin(plugin).expect("Failed to register plugin");
        
        // Create mock instance for testing
        let app = App::new();
        let instance = app.build_instance_sync().expect("Failed to build instance");
        
        // Enable plugin
        let enable_result = registry.enable_plugin("lifecycle_test", &instance);
        assert!(enable_result.is_ok());
        
        // Check plugin is enabled
        assert!(registry.is_plugin_enabled("lifecycle_test"));
        
        // Disable plugin
        let disable_result = registry.disable_plugin("lifecycle_test");
        assert!(disable_result.is_ok());
        
        // Check plugin is disabled
        assert!(!registry.is_plugin_enabled("lifecycle_test"));
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            description: Some("Test plugin description".to_string()),
            dependencies: vec!["core".to_string()],
            optional_dependencies: vec!["optional_feature".to_string()],
        };
        
        assert_eq!(metadata.name, "test_plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author.as_ref().unwrap(), "Test Author");
        assert!(!metadata.dependencies.is_empty());
        assert!(!metadata.optional_dependencies.is_empty());
    }

    #[test]
    fn test_plugin_dependency_resolution() {
        let mut registry = PluginRegistry::new();
        
        // Register core plugin
        let core_plugin = Box::new(TestLegacyPlugin::new("core"));
        registry.register_plugin(core_plugin).expect("Failed to register core plugin");
        
        // Register dependent plugin
        let dependent_plugin = Box::new(TestLegacyPlugin::new("dependent"));
        registry.register_plugin(dependent_plugin).expect("Failed to register dependent plugin");
        
        // Set up dependency
        registry.add_dependency("dependent", "core").expect("Failed to add dependency");
        
        // Verify dependency exists
        assert!(registry.has_dependency("dependent", "core"));
        
        // Test dependency resolution order
        let load_order = registry.resolve_load_order().expect("Failed to resolve load order");
        let core_index = load_order.iter().position(|&name| name == "core").unwrap();
        let dependent_index = load_order.iter().position(|&name| name == "dependent").unwrap();
        
        // Core should be loaded before dependent
        assert!(core_index < dependent_index);
    }

    #[test]
    fn test_plugin_error_handling() {
        struct FailingPlugin;
        
        impl Plugin for FailingPlugin {
            fn name(&self) -> &str {
                "failing_plugin"
            }
            
            fn version(&self) -> &str {
                "1.0.0"
            }
            
            fn on_enable(&mut self, _instance: &Instance) -> Result<(), PluginError> {
                Err(PluginError::InitializationFailed("Test failure".to_string()))
            }
            
            fn on_disable(&mut self) -> Result<(), PluginError> {
                Ok(())
            }
        }
        
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(FailingPlugin);
        
        registry.register_plugin(plugin).expect("Failed to register plugin");
        
        let app = App::new();
        let instance = app.build_instance_sync().expect("Failed to build instance");
        
        // Enable should fail
        let enable_result = registry.enable_plugin("failing_plugin", &instance);
        assert!(enable_result.is_err());
        assert!(!registry.is_plugin_enabled("failing_plugin"));
    }

    #[test]
    fn test_plugin_hot_reload() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestLegacyPlugin::new("hot_reload_test"));
        
        registry.register_plugin(plugin).expect("Failed to register plugin");
        
        let app = App::new();
        let instance = app.build_instance_sync().expect("Failed to build instance");
        
        // Enable plugin
        registry.enable_plugin("hot_reload_test", &instance).expect("Failed to enable plugin");
        assert!(registry.is_plugin_enabled("hot_reload_test"));
        
        // Hot reload (disable and re-enable)
        let reload_result = registry.reload_plugin("hot_reload_test", &instance);
        assert!(reload_result.is_ok());
        assert!(registry.is_plugin_enabled("hot_reload_test"));
    }
}

#[cfg(test)]
mod auth_migration_tests {
    use super::*;

    #[test]
    fn test_unified_auth_service_creation() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        let stats = auth_service.get_auth_stats();
        assert_eq!(stats.total_authentications, 0);
        assert_eq!(stats.java_authentications, 0);
        assert_eq!(stats.bedrock_authentications, 0);
    }

    #[test]
    fn test_java_offline_authentication_migration() {
        let mut config = UnifiedAuthConfig::default();
        config.java_config.offline_mode = true;
        
        let auth_service = UnifiedAuthService::new(config);
        
        // Test migration from legacy Java authentication
        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some("LegacyPlayer".to_string());
        
        let result = auth_service.authenticate_player(ProtocolType::Java, &auth_data);
        assert!(result.is_ok());
        
        let profile = result.unwrap();
        assert_eq!(profile.username, "LegacyPlayer");
        assert_eq!(profile.protocol_type, ProtocolType::Java);
        assert!(profile.xuid.is_none());
        
        // Verify UUID generation is consistent
        let auth_data2 = UnifiedAuthData {
            username: Some("LegacyPlayer".to_string()),
            ..Default::default()
        };
        let result2 = auth_service.authenticate_player(ProtocolType::Java, &auth_data2);
        assert!(result2.is_ok());
        
        let profile2 = result2.unwrap();
        assert_eq!(profile.uuid, profile2.uuid); // Same username should generate same UUID
    }

    #[test]
    fn test_bedrock_authentication_migration() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        // Test migration from legacy Bedrock authentication
        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some("BedrockLegacyPlayer".to_string());
        auth_data.xuid = Some("1234567890123456".to_string());
        auth_data.identity_chain = Some(vec![
            "mock_identity_token".to_string(),
            "mock_xbox_token".to_string(),
        ]);
        
        let result = auth_service.authenticate_player(ProtocolType::Bedrock, &auth_data);
        assert!(result.is_ok());
        
        let profile = result.unwrap();
        assert_eq!(profile.username, "BedrockLegacyPlayer");
        assert_eq!(profile.protocol_type, ProtocolType::Bedrock);
        assert_eq!(profile.xuid, Some(1234567890123456));
    }

    #[test]
    fn test_encryption_migration() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        let connection_id = Uuid::new_v4();
        let shared_secret = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        
        // Test Java encryption migration
        let java_result = auth_service.enable_encryption(
            connection_id, 
            ProtocolType::Java, 
            shared_secret.clone()
        );
        assert!(java_result.is_ok());
        
        // Test encryption/decryption works
        let test_data = b"Migration test data";
        let encrypted = auth_service.encrypt_data(connection_id, test_data);
        assert!(encrypted.is_ok());
        
        let decrypted = auth_service.decrypt_data(connection_id, &encrypted.unwrap());
        assert!(decrypted.is_ok());
        assert_eq!(decrypted.unwrap().as_ref(), test_data);
        
        // Test Bedrock encryption migration
        let bedrock_connection_id = Uuid::new_v4();
        let bedrock_result = auth_service.enable_encryption(
            bedrock_connection_id,
            ProtocolType::Bedrock,
            shared_secret
        );
        assert!(bedrock_result.is_ok());
    }

    #[test]
    fn test_auth_config_migration() {
        // Test default configuration migration
        let config = UnifiedAuthConfig::default();
        
        // Java config should have sensible defaults
        assert!(!config.java_config.offline_mode);
        assert!(config.java_config.enable_encryption);
        assert_eq!(config.java_config.mojang_api_url, "https://api.mojang.com");
        
        // Bedrock config should have sensible defaults
        assert!(config.bedrock_config.require_xbox_live);
        assert!(config.bedrock_config.enable_encryption);
        assert_eq!(config.bedrock_config.xbox_live_api_url, "https://user.auth.xboxlive.com");
        
        // Unified config should enable caching
        assert!(config.enable_caching);
        assert_eq!(config.cache_timeout, 300);
    }

    #[test]
    fn test_auth_statistics_migration() {
        let mut config = UnifiedAuthConfig::default();
        config.java_config.offline_mode = true;
        
        let auth_service = UnifiedAuthService::new(config);
        
        // Simulate legacy authentication data
        let legacy_java_users = vec!["Player1", "Player2", "Player3"];
        let legacy_bedrock_users = vec!["BedrockPlayer1", "BedrockPlayer2"];
        
        // Migrate Java users
        for username in legacy_java_users {
            let mut auth_data = UnifiedAuthData::default();
            auth_data.username = Some(username.to_string());
            
            let result = auth_service.authenticate_player(ProtocolType::Java, &auth_data);
            assert!(result.is_ok());
        }
        
        // Migrate Bedrock users
        for (i, username) in legacy_bedrock_users.iter().enumerate() {
            let mut auth_data = UnifiedAuthData::default();
            auth_data.username = Some(username.to_string());
            auth_data.xuid = Some(format!("123456789{}", i));
            
            let result = auth_service.authenticate_player(ProtocolType::Bedrock, &auth_data);
            assert!(result.is_ok());
        }
        
        // Verify statistics
        let stats = auth_service.get_auth_stats();
        assert_eq!(stats.total_authentications, 5);
        assert_eq!(stats.java_authentications, 3);
        assert_eq!(stats.bedrock_authentications, 2);
        assert_eq!(stats.active_sessions, 5);
    }

    #[test]
    fn test_verify_token_migration() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        // Test that verify token generation works for migration
        let token1 = auth_service.generate_verify_token();
        let token2 = auth_service.generate_verify_token();
        
        assert_eq!(token1.len(), 4);
        assert_eq!(token2.len(), 4);
        assert_ne!(token1, token2); // Should be different
    }

    #[test]
    fn test_bedrock_encryption_token_migration() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        // Test Bedrock encryption token generation for migration
        let client_public_key = "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAE8ELkixyLcwlZryUQcu1TvPOmI2B7vX83ndnWRUaXm74wFfa5f/lwQNTfrLVHa2PmenpGI6JhIMUJaWZrjmMj90NoKNFSNBuKdm8rYiXsfaz3K36x/1U26HpG0ZxK/V1V";
        
        let result = auth_service.generate_bedrock_encryption_token(client_public_key);
        assert!(result.is_ok());
        
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_skin_validation_migration() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);
        
        // Test skin validation for migrated Bedrock players
        let valid_64x64_skin = vec![0u8; 64 * 64 * 4];
        let valid_128x128_skin = vec![0u8; 128 * 128 * 4];
        let invalid_small_skin = vec![0u8; 32 * 32 * 4];
        let invalid_large_skin = vec![0u8; 256 * 256 * 4];
        
        assert!(auth_service.validate_skin(&valid_64x64_skin).is_ok());
        assert!(auth_service.validate_skin(&valid_128x128_skin).is_ok());
        assert!(auth_service.validate_skin(&invalid_small_skin).is_err());
        assert!(auth_service.validate_skin(&invalid_large_skin).is_err());
    }
}

#[cfg(test)]
mod integration_migration_tests {
    use super::*;

    #[test]
    fn test_full_migration_workflow() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Step 1: Create legacy configurations
        let legacy_server_config = create_test_legacy_server_config();
        let server_config_path = temp_dir.path().join("server.toml");
        let server_config_content = toml::to_string(&legacy_server_config)
            .expect("Failed to serialize server config");
        std::fs::write(&server_config_path, server_config_content)
            .expect("Failed to write server config");
        
        // Step 2: Migrate configurations
        let migrator = ConfigMigrator::new();
        let unified_server_config = migrator.migrate_minecraft_server_config(&server_config_path)
            .expect("Failed to migrate server config");
        
        let mirai_config = MiraiConfig::new();
        let unified_mirai_config = migrator.migrate_mirai_config(&mirai_config)
            .expect("Failed to migrate mirai config");
        
        // Step 3: Merge configurations
        let merged_config = migrator.merge_configurations(
            unified_mirai_config,
            unified_server_config
        ).expect("Failed to merge configurations");
        
        // Step 4: Validate merged configuration
        let validation_result = migrator.validate_migration(&merged_config);
        assert!(validation_result.is_ok());
        
        // Step 5: Generate migration report
        let report = migrator.create_migration_report(
            &mirai_config,
            Some(&legacy_server_config),
            &merged_config
        );
        
        // Verify report contains expected information
        assert!(!report.changes.is_empty());
        assert!(!report.new_features.is_empty());
        assert!(report.warnings.is_empty()); // Should be no warnings for valid migration
        
        // Step 6: Save migration report
        let report_path = temp_dir.path().join("migration_report.json");
        let save_result = report.save_to_file(&report_path);
        assert!(save_result.is_ok());
        assert!(report_path.exists());
    }

    #[test]
    fn test_migration_with_world_data() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Create mock world data
        create_mock_world_data(&temp_dir);
        
        // Create provider for the world
        let (provider, _temp_world_dir) = create_test_provider_with_world(&temp_dir);
        let manager = LevelDbCompatibilityManager::new(provider);
        
        // Test world metadata loading
        let mut manager = manager;
        let metadata_result = manager.load_world_metadata();
        assert!(metadata_result.is_ok());
        assert!(metadata_result.unwrap().is_some());
        
        // Test migration
        let migration_result = manager.migrate_to_enhanced_format();
        assert!(migration_result.is_ok());
        
        let report = migration_result.unwrap();
        // Should handle empty world gracefully
        assert_eq!(report.failed_chunks, 0);
    }

    #[test]
    fn test_migration_error_handling() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Test with invalid config file
        let invalid_config_path = temp_dir.path().join("invalid.toml");
        std::fs::write(&invalid_config_path, "invalid toml content [[[")
            .expect("Failed to write invalid config");
        
        let migrator = ConfigMigrator::new();
        let result = migrator.migrate_minecraft_server_config(&invalid_config_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationError::ParseFailed(_)));
        
        // Test with non-existent file
        let missing_config_path = temp_dir.path().join("missing.toml");
        let result = migrator.migrate_minecraft_server_config(&missing_config_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationError::FileNotFound(_)));
    }

    fn create_test_legacy_server_config() -> LegacyServerConfig {
        use mirai_core::config_migration::{
            LegacyServerSettings, LegacyNetworkConfig, LegacyWorldSettings, LegacyFeatureFlags
        };
        
        LegacyServerConfig {
            server: LegacyServerSettings {
                server_name: "Integration Test Server".to_string(),
                motd: "Integration Test MOTD".to_string(),
                max_players: 100,
                view_distance: 16,
                simulation_distance: 10,
                difficulty: "normal".to_string(),
                gamemode: "survival".to_string(),
                hardcore: false,
                pvp: true,
                online_mode: true,
                whitelist: false,
                enforce_whitelist: false,
            },
            network: LegacyNetworkConfig {
                port: 25565,
                max_clients: 200,
                timeout_seconds: 60,
                compression_threshold: 512,
                encryption_enabled: true,
            },
            world: LegacyWorldSettings {
                world_name: "integration_world".to_string(),
                seed: Some(987654321),
                generate_structures: true,
                generator_settings: "{}".to_string(),
                level_type: "default".to_string(),
                spawn_protection: 32,
                max_world_size: 29999984,
            },
            features: LegacyFeatureFlags {
                vanilla_mobs: true,
                redstone: true,
                world_generation: true,
                creative_mode: false,
                command_system: true,
                performance_monitoring: true,
            },
        }
    }

    fn create_mock_world_data(temp_dir: &TempDir) {
        let world_path = temp_dir.path().join("world");
        std::fs::create_dir_all(&world_path).expect("Failed to create world directory");
        
        // Create mock level.dat
        let level_dat_content = vec![
            8, 0, 0, 0,  // version
            200, 0, 0, 0, // size
        ];
        let mut level_data = level_dat_content;
        level_data.extend_from_slice(&[0; 196]); // Padding
        std::fs::write(world_path.join("level.dat"), level_data)
            .expect("Failed to write level.dat");
        
        // Create mock db directory
        let db_path = world_path.join("db");
        std::fs::create_dir_all(&db_path).expect("Failed to create db directory");
    }

    fn create_test_provider_with_world(temp_dir: &TempDir) -> (Arc<Provider>, TempDir) {
        let world_temp_dir = TempDir::new().expect("Failed to create world temp directory");
        create_mock_world_data(&world_temp_dir);
        
        let provider = Provider::open(world_temp_dir.path().join("world"))
            .expect("Failed to open provider");
        (Arc::new(provider), world_temp_dir)
    }
}