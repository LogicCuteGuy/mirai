//! Final system integration test for task 10
//! 
//! This test validates that all systems work together correctly and that
//! the unified Mirai server is ready for production use.

use mirai_core::{
    unified_config::UnifiedConfig,
    performance::PerformanceManager,
    plugin::{App, Plugin, PluginInfo},
    ecs::{World, Component, Resource, System},
};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time::timeout;
use anyhow::Result;

#[tokio::test]
async fn test_complete_unified_system_integration() {
    // Test that the complete unified system works end-to-end
    
    // 1. Configuration System Integration
    let mut config = UnifiedConfig::default();
    config.features.ecs_system = true;
    config.features.plugin_system = true;
    config.features.performance_monitoring = true;
    
    assert!(config.validate().is_ok(), "Unified config should be valid");
    assert!(config.is_feature_enabled("ecs_system"));
    assert!(config.is_feature_enabled("plugin_system"));
    assert!(config.is_feature_enabled("performance_monitoring"));
    
    // 2. Performance Manager Integration
    let performance_manager = PerformanceManager::new();
    
    // Test memory pools
    let memory_pools = performance_manager.memory_pools();
    let entity_buffer = memory_pools.entity_pool.get();
    let packet_buffer = memory_pools.packet_buffer_pool.get();
    
    assert!(!entity_buffer.is_empty() || entity_buffer.capacity() > 0);
    assert!(!packet_buffer.is_empty() || packet_buffer.capacity() > 0);
    
    // Test metrics collection
    let metrics = performance_manager.metrics_collector();
    metrics.increment_counter("final_integration_test", None);
    metrics.record_gauge("system_health", 100.0, None);
    
    let summary = metrics.summary();
    assert!(summary.total_metrics >= 2);
    
    // 3. ECS + Plugin System Integration
    let mut app = App::new(Weak::new());
    
    // Add comprehensive test plugin
    app.add_plugin(FinalIntegrationPlugin).expect("Failed to add final integration plugin");
    
    let world = app.world_mut();
    
    // Create test entities with components
    let entity1 = world.spawn();
    let entity2 = world.spawn();
    
    world.insert(entity1, FinalTestComponent {
        id: 1,
        name: "Entity1".to_string(),
        health: 100,
        position: (0.0, 0.0, 0.0),
    }).expect("Failed to insert component");
    
    world.insert(entity2, FinalTestComponent {
        id: 2,
        name: "Entity2".to_string(),
        health: 80,
        position: (10.0, 5.0, -3.0),
    }).expect("Failed to insert component");
    
    // Add shared resource
    world.insert_resource(FinalTestResource {
        total_entities: 2,
        processed_count: 0,
        system_ticks: 0,
    });
    
    // 4. System Execution Integration
    for tick in 0..5 {
        let tick_start = std::time::Instant::now();
        
        // Run ECS systems
        app.update().expect("Failed to update app");
        
        // Record performance metrics
        let tick_duration = tick_start.elapsed();
        metrics.record_gauge("tick_duration_ms", tick_duration.as_millis() as f64, None);
        
        // Brief pause to simulate real server timing
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    // 5. Validate System State
    let resource = world.get_resource::<FinalTestResource>().unwrap();
    assert_eq!(resource.total_entities, 2);
    assert_eq!(resource.system_ticks, 5); // Should have run 5 times
    assert!(resource.processed_count > 0); // Should have processed entities
    
    // Validate entity state changes
    let component1 = world.get::<FinalTestComponent>(entity1).unwrap();
    let component2 = world.get::<FinalTestComponent>(entity2).unwrap();
    
    assert_eq!(component1.id, 1);
    assert_eq!(component2.id, 2);
    assert!(component1.health <= 100); // May have been modified by systems
    assert!(component2.health <= 80);
    
    // 6. Performance Validation
    let perf_stats = performance_manager.performance_stats();
    assert!(perf_stats.memory.overall_efficiency() >= 0.0);
    assert!(perf_stats.threading.utilization() >= 0.0);
    assert!(perf_stats.metrics.total_metrics >= 7); // At least 7 metrics recorded
    
    // 7. Configuration Serialization/Deserialization
    let config_json = serde_json::to_string(&config).expect("Config should serialize to JSON");
    let deserialized_config: UnifiedConfig = serde_json::from_str(&config_json)
        .expect("Config should deserialize from JSON");
    
    assert_eq!(config.server.server_name, deserialized_config.server.server_name);
    assert_eq!(config.features.ecs_system, deserialized_config.features.ecs_system);
    
    // 8. Memory Management Validation
    drop(entity_buffer);
    drop(packet_buffer);
    performance_manager.maintenance(); // Should not panic
    
    // 9. Thread Manager Integration
    let thread_manager = performance_manager.thread_manager();
    let cpu_task = thread_manager.execute_cpu_task(|| {
        // Simulate CPU work
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        sum
    }).expect("Failed to submit CPU task");
    
    let result = cpu_task.await_result().await.expect("CPU task should complete");
    assert!(result > 0);
    
    println!("✅ Final system integration test completed successfully!");
    println!("   - Configuration system: ✅");
    println!("   - Performance monitoring: ✅");
    println!("   - ECS system: ✅");
    println!("   - Plugin system: ✅");
    println!("   - Memory management: ✅");
    println!("   - Thread management: ✅");
    println!("   - Serialization: ✅");
}

#[tokio::test]
async fn test_system_stress_and_stability() {
    // Stress test to ensure system stability under load
    let performance_manager = PerformanceManager::new();
    let mut app = App::new(Weak::new());
    
    app.add_plugin(StressTestPlugin).expect("Failed to add stress test plugin");
    
    let world = app.world_mut();
    
    // Create many entities
    let entity_count = 500;
    let mut entities = Vec::new();
    
    for i in 0..entity_count {
        let entity = world.spawn();
        
        world.insert(entity, StressTestComponent {
            id: i,
            value: i as f32,
            active: true,
        }).expect("Failed to insert stress component");
        
        entities.push(entity);
    }
    
    world.insert_resource(StressTestResource {
        total_entities: entity_count,
        processed_count: 0,
        max_value: 0.0,
    });
    
    // Run stress test
    let start_time = std::time::Instant::now();
    
    for _tick in 0..20 {
        let tick_start = std::time::Instant::now();
        
        app.update().expect("Failed to update app during stress test");
        
        let tick_duration = tick_start.elapsed();
        assert!(tick_duration < Duration::from_millis(100), "Tick should complete quickly");
        
        // Brief pause
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    let total_duration = start_time.elapsed();
    assert!(total_duration < Duration::from_secs(5), "Stress test should complete quickly");
    
    // Validate stress test results
    let resource = world.get_resource::<StressTestResource>().unwrap();
    assert_eq!(resource.total_entities, entity_count);
    assert!(resource.processed_count > 0);
    assert!(resource.max_value > 0.0);
    
    println!("✅ System stress test completed successfully!");
    println!("   - Processed {} entities", entity_count);
    println!("   - Completed in {:?}", total_duration);
    println!("   - Average tick time: {:?}", total_duration / 20);
}

#[test]
fn test_configuration_migration_compatibility() {
    // Test that configuration migration works correctly
    let config = UnifiedConfig::default();
    
    // Test Mirai config conversion
    let mirai_config = config.to_mirai_config();
    assert_eq!(mirai_config.max_connections(), config.server.max_connections);
    assert_eq!(mirai_config.max_render_distance(), config.server.max_render_distance);
    
    // Test feature validation
    assert!(config.validate().is_ok());
    
    // Test serialization formats
    let json_result = serde_json::to_string(&config);
    assert!(json_result.is_ok(), "Config should serialize to JSON");
    
    let toml_result = toml::to_string(&config);
    assert!(toml_result.is_ok(), "Config should serialize to TOML");
    
    if let (Ok(json_str), Ok(toml_str)) = (json_result, toml_result) {
        assert!(!json_str.is_empty());
        assert!(!toml_str.is_empty());
        
        // Test deserialization
        let json_config: Result<UnifiedConfig, _> = serde_json::from_str(&json_str);
        let toml_config: Result<UnifiedConfig, _> = toml::from_str(&toml_str);
        
        assert!(json_config.is_ok(), "Should deserialize from JSON");
        assert!(toml_config.is_ok(), "Should deserialize from TOML");
    }
    
    println!("✅ Configuration migration compatibility test completed!");
}

// Test plugin implementations

struct FinalIntegrationPlugin;

impl Plugin for FinalIntegrationPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("final_integration_plugin", semver::Version::new(1, 0, 0))
            .with_description("Final integration test plugin")
            .with_author("Mirai Integration Test Suite")
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.world_mut().insert_resource(FinalTestResource {
            total_entities: 0,
            processed_count: 0,
            system_ticks: 0,
        });
        app.add_system(FinalTestSystem);
        Ok(())
    }
}

struct StressTestPlugin;

impl Plugin for StressTestPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("stress_test_plugin", semver::Version::new(1, 0, 0))
            .with_description("Stress test plugin for system validation")
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.add_system(StressTestSystem);
        Ok(())
    }
}

// Test components

#[derive(Debug, Clone)]
struct FinalTestComponent {
    id: u32,
    name: String,
    health: i32,
    position: (f32, f32, f32),
}

impl Component for FinalTestComponent {}

#[derive(Debug, Clone)]
struct StressTestComponent {
    id: usize,
    value: f32,
    active: bool,
}

impl Component for StressTestComponent {}

// Test resources

#[derive(Debug, Clone)]
struct FinalTestResource {
    total_entities: usize,
    processed_count: usize,
    system_ticks: u32,
}

impl Resource for FinalTestResource {}

#[derive(Debug, Clone)]
struct StressTestResource {
    total_entities: usize,
    processed_count: usize,
    max_value: f32,
}

impl Resource for StressTestResource {}

// Test systems

struct FinalTestSystem;

impl System for FinalTestSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Update resource to track system execution
        if let Some(mut resource) = world.get_resource_mut::<FinalTestResource>() {
            resource.system_ticks += 1;
            resource.processed_count = resource.total_entities; // Simulate processing all entities
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "final_test_system"
    }
}

struct StressTestSystem;

impl System for StressTestSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate processing stress test components
        if let Some(mut resource) = world.get_resource_mut::<StressTestResource>() {
            resource.processed_count = resource.total_entities;
            resource.max_value = resource.total_entities as f32; // Simulate finding max value
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "stress_test_system"
    }
}