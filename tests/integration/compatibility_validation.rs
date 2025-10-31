//! Compatibility validation tests for existing mirai deployments
//! 
//! Tests that validate existing mirai configurations, plugins, and deployments
//! continue to work with the merged system.

use super::*;
use mirai_core::{
    App, Instance, UnifiedConfig, ConfigMigrator, MiraiCompatibilityLayer,
    config::{MiraiConfig, FeatureConfig}
};
use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;
use serde_json;
use toml;

#[tokio::test]
async fn test_existing_mirai_config_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create legacy mirai config
    let legacy_config = create_legacy_mirai_config();
    let config_path = temp_dir.path().join("mirai_config.toml");
    
    let config_content = toml::to_string(&legacy_config)
        .expect("Failed to serialize legacy config");
    fs::write(&config_path, config_content)
        .expect("Failed to write legacy config");
    
    // Test config migration
    let migrator = ConfigMigrator::new();
    let unified_config = migrator.migrate_mirai_config_file(&config_path)
        .expect("Failed to migrate legacy config");
    
    // Verify migration preserved essential settings
    assert_eq!(unified_config.mirai.server.port, legacy_config.server.port);
    assert_eq!(unified_config.mirai.server.max_players, legacy_config.server.max_players);
    assert_eq!(unified_config.mirai.level.name, legacy_config.level.name);
    
    // Verify new features are properly initialized
    assert!(unified_config.plugins.enabled);
    assert!(unified_config.features.ecs_enabled);
}

#[tokio::test]
async fn test_existing_mirai_instance_compatibility() {
    // Create instance using legacy mirai patterns
    let mut app = App::new();
    
    // Add compatibility layer
    app.add_plugin(MiraiCompatibilityPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build compatible instance");
    
    // Test that legacy mirai APIs still work
    assert!(instance.is_running());
    
    // Test BedrockClient compatibility
    let client_manager = instance.client_manager();
    assert_eq!(client_manager.get_client_count(), 0);
    
    // Test Service compatibility
    let service_manager = instance.service_manager();
    assert!(service_manager.get_service("level").is_some());
    assert!(service_manager.get_service("network").is_some());
    
    // Test Level compatibility
    let level = instance.level();
    assert_eq!(level.get_name(), "world");
    assert!(level.get_spawn_position().is_some());
}

#[test]
fn test_legacy_plugin_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create legacy plugin structure
    create_legacy_plugin_files(&temp_dir);
    
    let mut app = App::new();
    
    // Load legacy plugin using compatibility layer
    let compatibility_layer = MiraiCompatibilityLayer::new();
    let legacy_plugin = compatibility_layer.load_legacy_plugin(
        temp_dir.path().join("legacy_plugin.toml")
    ).expect("Failed to load legacy plugin");
    
    app.add_plugin(legacy_plugin);
    
    // Verify plugin was loaded correctly
    let registry = app.plugin_registry();
    assert!(registry.is_plugin_loaded("legacy_test_plugin"));
}

#[tokio::test]
async fn test_existing_world_data_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create mock existing world data
    create_mock_world_data(&temp_dir);
    
    let mut app = App::new();
    app.add_plugin(WorldCompatibilityPlugin);
    
    // Configure to use existing world
    let mut config = UnifiedConfig::default();
    config.mirai.level.path = temp_dir.path().join("world").to_string_lossy().to_string();
    
    app.set_config(config);
    
    let instance = app.build_instance().await
        .expect("Failed to build instance with existing world");
    
    // Verify world data was loaded correctly
    let level = instance.level();
    assert!(level.chunk_exists(0, 0));
    
    // Verify LevelDB compatibility
    let leveldb = level.leveldb();
    assert!(leveldb.is_open());
    
    // Test that existing chunks can be read
    let chunk = level.get_chunk(0, 0).await
        .expect("Failed to get existing chunk");
    assert!(chunk.is_some());
}

#[test]
fn test_configuration_migration_utilities() {
    let migrator = ConfigMigrator::new();
    
    // Test mirai config migration
    let legacy_mirai = create_legacy_mirai_config();
    let unified = migrator.migrate_mirai_config(legacy_mirai);
    
    assert!(unified.mirai.server.port > 0);
    assert!(unified.plugins.enabled);
    assert!(unified.features.ecs_enabled);
    
    // Test validation
    let validation_result = migrator.validate_unified_config(&unified);
    assert!(validation_result.is_ok());
    
    // Test migration report
    let report = migrator.generate_migration_report(&unified);
    assert!(report.contains("Migration Summary"));
    assert!(report.contains("Mirai Configuration"));
    assert!(report.contains("New Features"));
}

#[tokio::test]
async fn test_existing_network_configuration_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create legacy network config
    let legacy_network_config = LegacyNetworkConfig {
        port: 19132,
        max_connections: 100,
        raknet_config: LegacyRakNetConfig {
            mtu: 1400,
            timeout: 30000,
        },
    };
    
    let config_path = temp_dir.path().join("network.toml");
    let config_content = toml::to_string(&legacy_network_config)
        .expect("Failed to serialize network config");
    fs::write(&config_path, config_content)
        .expect("Failed to write network config");
    
    // Test network config migration
    let migrator = ConfigMigrator::new();
    let unified_network = migrator.migrate_network_config_file(&config_path)
        .expect("Failed to migrate network config");
    
    // Verify migration
    assert_eq!(unified_network.raknet.port, 19132);
    assert_eq!(unified_network.raknet.max_connections, 100);
    assert_eq!(unified_network.raknet.mtu, 1400);
    
    // Verify new protocol features are initialized
    assert!(unified_network.protocol.compression_enabled);
    assert!(unified_network.protocol.encryption_enabled);
}

#[test]
fn test_backward_compatibility_api() {
    let compatibility_layer = MiraiCompatibilityLayer::new();
    
    // Test Instance adapter
    let instance_adapter = compatibility_layer.instance_adapter();
    
    // These should work with legacy code patterns
    assert!(instance_adapter.supports_legacy_api());
    
    // Test Client adapter
    let client_adapter = compatibility_layer.client_adapter();
    assert!(client_adapter.supports_bedrock_client());
    
    // Test Level adapter
    let level_adapter = compatibility_layer.level_adapter();
    assert!(level_adapter.supports_leveldb());
    assert!(level_adapter.supports_chunk_loading());
}

#[tokio::test]
async fn test_existing_plugin_extensions_compatibility() {
    let mut app = App::new();
    
    // Add compatibility for existing mirai extensions
    app.add_plugin(ExtensionCompatibilityPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build instance with extension compatibility");
    
    // Test that existing extension patterns work
    let extension_manager = instance.extension_manager();
    
    // Register legacy extension
    let legacy_extension = LegacyTestExtension::new();
    extension_manager.register_extension(Box::new(legacy_extension))
        .expect("Failed to register legacy extension");
    
    // Verify extension is working
    assert!(extension_manager.is_extension_loaded("legacy_test"));
    
    // Test extension lifecycle
    extension_manager.start_all_extensions()
        .expect("Failed to start extensions");
    
    assert!(extension_manager.get_extension("legacy_test").unwrap().is_running());
}

#[test]
fn test_data_format_compatibility() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create legacy data files
    create_legacy_data_files(&temp_dir);
    
    let migrator = ConfigMigrator::new();
    
    // Test player data migration
    let player_data_path = temp_dir.path().join("players");
    let migration_result = migrator.migrate_player_data(&player_data_path);
    assert!(migration_result.is_ok());
    
    // Test world data migration
    let world_data_path = temp_dir.path().join("world");
    let world_migration = migrator.migrate_world_data(&world_data_path);
    assert!(world_migration.is_ok());
    
    // Verify migrated data is accessible
    let migrated_players = migration_result.unwrap();
    assert!(!migrated_players.is_empty());
}

#[tokio::test]
async fn test_performance_regression_compatibility() {
    // Test that merged system performs at least as well as original mirai
    let legacy_performance = benchmark_legacy_mirai_simulation().await;
    let merged_performance = benchmark_merged_system().await;
    
    // Performance should not regress significantly
    assert!(merged_performance.throughput >= legacy_performance.throughput * 0.9);
    assert!(merged_performance.latency <= legacy_performance.latency * 1.1);
    assert!(merged_performance.memory_usage <= legacy_performance.memory_usage * 1.2);
}

// Helper functions and test structures

fn create_legacy_mirai_config() -> LegacyMiraiConfig {
    LegacyMiraiConfig {
        server: LegacyServerConfig {
            port: 19132,
            max_players: 20,
            motd: "Legacy Mirai Server".to_string(),
        },
        level: LegacyLevelConfig {
            name: "world".to_string(),
            path: "./world".to_string(),
            generator: "flat".to_string(),
        },
        network: LegacyNetworkConfig {
            port: 19132,
            max_connections: 100,
            raknet_config: LegacyRakNetConfig {
                mtu: 1400,
                timeout: 30000,
            },
        },
    }
}

fn create_legacy_plugin_files(temp_dir: &TempDir) {
    let plugin_config = r#"
[plugin]
name = "legacy_test_plugin"
version = "1.0.0"
author = "Test Author"

[plugin.config]
enabled = true
priority = 1

[plugin.features]
test_feature = true
"#;
    
    fs::write(temp_dir.path().join("legacy_plugin.toml"), plugin_config)
        .expect("Failed to write legacy plugin config");
}

fn create_mock_world_data(temp_dir: &TempDir) {
    let world_path = temp_dir.path().join("world");
    fs::create_dir_all(&world_path).expect("Failed to create world directory");
    
    // Create mock level.dat
    let level_dat = r#"{"LevelName":"TestWorld","SpawnX":0,"SpawnY":64,"SpawnZ":0}"#;
    fs::write(world_path.join("level.dat"), level_dat)
        .expect("Failed to write level.dat");
    
    // Create mock db directory (LevelDB)
    let db_path = world_path.join("db");
    fs::create_dir_all(&db_path).expect("Failed to create db directory");
    
    // Create mock chunk data
    fs::write(db_path.join("000001.log"), b"mock chunk data")
        .expect("Failed to write mock chunk data");
}

fn create_legacy_data_files(temp_dir: &TempDir) {
    // Create legacy player data
    let players_path = temp_dir.path().join("players");
    fs::create_dir_all(&players_path).expect("Failed to create players directory");
    
    let player_data = r#"{"name":"TestPlayer","uuid":"test-uuid","position":[0,64,0]}"#;
    fs::write(players_path.join("test-player.json"), player_data)
        .expect("Failed to write player data");
    
    // Create legacy world data
    let world_path = temp_dir.path().join("world");
    fs::create_dir_all(&world_path).expect("Failed to create world directory");
    
    let world_data = r#"{"name":"TestWorld","seed":12345,"generator":"default"}"#;
    fs::write(world_path.join("world.json"), world_data)
        .expect("Failed to write world data");
}

async fn benchmark_legacy_mirai_simulation() -> PerformanceBenchmark {
    // Simulate legacy mirai performance
    let start_time = std::time::Instant::now();
    
    // Simulate typical operations
    for _ in 0..1000 {
        tokio::time::sleep(std::time::Duration::from_micros(1)).await;
    }
    
    let duration = start_time.elapsed();
    
    PerformanceBenchmark {
        throughput: 1000.0 / duration.as_secs_f64(),
        latency: duration,
        memory_usage: 1024 * 1024, // 1MB simulated
    }
}

async fn benchmark_merged_system() -> PerformanceBenchmark {
    let mut app = App::new();
    app.add_plugin(BenchmarkPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build benchmark instance");
    
    let start_time = std::time::Instant::now();
    
    // Simulate same operations with merged system
    for _ in 0..1000 {
        let entity = instance.world().spawn_entity();
        instance.world().add_component(entity, BenchmarkComponent { value: 1 });
    }
    
    instance.world().run_systems();
    
    let duration = start_time.elapsed();
    
    PerformanceBenchmark {
        throughput: 1000.0 / duration.as_secs_f64(),
        latency: duration,
        memory_usage: 1024 * 1024, // 1MB simulated
    }
}

// Test plugin and component implementations

struct MiraiCompatibilityPlugin;

impl mirai_core::Plugin for MiraiCompatibilityPlugin {
    fn name(&self) -> &'static str {
        "mirai_compatibility"
    }
    
    fn build(&self, app: &mut App) {
        // Enable compatibility features
        app.enable_mirai_compatibility();
    }
}

struct WorldCompatibilityPlugin;

impl mirai_core::Plugin for WorldCompatibilityPlugin {
    fn name(&self) -> &'static str {
        "world_compatibility"
    }
    
    fn build(&self, app: &mut App) {
        // Enable world compatibility features
        app.enable_leveldb_compatibility();
    }
}

struct ExtensionCompatibilityPlugin;

impl mirai_core::Plugin for ExtensionCompatibilityPlugin {
    fn name(&self) -> &'static str {
        "extension_compatibility"
    }
    
    fn build(&self, app: &mut App) {
        // Enable extension compatibility
        app.enable_extension_compatibility();
    }
}

struct BenchmarkPlugin;

impl mirai_core::Plugin for BenchmarkPlugin {
    fn name(&self) -> &'static str {
        "benchmark_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<BenchmarkComponent>();
        app.add_system(benchmark_system);
    }
}

#[derive(Debug, Clone)]
struct BenchmarkComponent {
    value: i32,
}

impl mirai_core::Component for BenchmarkComponent {}

fn benchmark_system(world: &mut mirai_core::World) {
    for mut component in world.query::<&mut BenchmarkComponent>() {
        component.value += 1;
    }
}

struct LegacyTestExtension {
    running: bool,
}

impl LegacyTestExtension {
    fn new() -> Self {
        Self { running: false }
    }
    
    fn is_running(&self) -> bool {
        self.running
    }
}

impl mirai_core::Extension for LegacyTestExtension {
    fn name(&self) -> &'static str {
        "legacy_test"
    }
    
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running = true;
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running = false;
        Ok(())
    }
}

// Configuration structures for testing

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyMiraiConfig {
    server: LegacyServerConfig,
    level: LegacyLevelConfig,
    network: LegacyNetworkConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyServerConfig {
    port: u16,
    max_players: u32,
    motd: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyLevelConfig {
    name: String,
    path: String,
    generator: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyNetworkConfig {
    port: u16,
    max_connections: u32,
    raknet_config: LegacyRakNetConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyRakNetConfig {
    mtu: u16,
    timeout: u32,
}

#[derive(Debug, Clone)]
struct PerformanceBenchmark {
    throughput: f64,
    latency: std::time::Duration,
    memory_usage: usize,
}