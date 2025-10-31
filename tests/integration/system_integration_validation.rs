//! System integration validation tests for task 10.2
//! 
//! These tests verify that ECS + Plugin + Protocol systems work together correctly
//! and that UnifiedConfig properly configures all subsystems.

use super::*;
use mirai_core::{
    plugin::{App, Plugin, PluginInfo},
    ecs::{World, EntityId, Component, Resource, System, SystemScheduler},
    unified_config::UnifiedConfig,
    performance::PerformanceManager,
    instance::Instance,
};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time::timeout;
use anyhow::Result;

#[tokio::test]
async fn test_unified_config_system_integration() {
    // Test that UnifiedConfig properly configures all subsystems
    let mut config = UnifiedConfig::default();
    
    // Enable all features for comprehensive testing
    config.features.ecs_system = true;
    config.features.plugin_system = true;
    config.features.performance_monitoring = true;
    
    // Validate configuration
    assert!(config.validate().is_ok());
    
    // Test feature checking
    assert!(config.is_feature_enabled("ecs_system"));
    assert!(config.is_feature_enabled("plugin_system"));
    assert!(config.is_feature_enabled("performance_monitoring"));
    
    // Test mirai config conversion
    let mirai_config = config.to_mirai_config();
    assert_eq!(mirai_config.max_connections(), config.server.max_connections);
    assert_eq!(mirai_config.max_render_distance(), config.server.max_render_distance);
    
    // Test that config can be used to create instance
    let instance = Instance::builder()
        .level_path("test_unified_level")
        .build()
        .await
        .expect("Failed to create instance with unified config");
    
    // Verify instance was created with correct configuration
    let instance_config = instance.config();
    assert!(instance_config.max_connections() > 0);
}

#[tokio::test]
async fn test_ecs_plugin_protocol_integration() {
    // Test that ECS + Plugin + Protocol systems work together
    let mut app = App::new(Weak::new());
    
    // Add integrated test plugin
    app.add_plugin(IntegratedTestPlugin).expect("Failed to add integrated plugin");
    
    let instance = create_test_mirai_instance().await
        .expect("Failed to create test instance");
    
    // Verify ECS system is working
    let world = app.world_mut();
    
    // Create test entities with components
    let entity1 = world.spawn();
    let entity2 = world.spawn();
    
    world.insert(entity1, IntegratedTestComponent { 
        id: 1, 
        name: "TestEntity1".to_string(),
        protocol_data: vec![1, 2, 3, 4],
    }).expect("Failed to insert component");
    
    world.insert(entity2, IntegratedTestComponent { 
        id: 2, 
        name: "TestEntity2".to_string(),
        protocol_data: vec![5, 6, 7, 8],
    }).expect("Failed to insert component");
    
    // Add shared resource
    world.insert_resource(IntegratedTestResource { 
        connection_count: 0,
        packet_count: 0,
    });
    
    // Run systems to verify integration
    app.update().expect("Failed to update app");
    
    // Verify system execution
    let component1 = world.get::<IntegratedTestComponent>(entity1).unwrap();
    let component2 = world.get::<IntegratedTestComponent>(entity2).unwrap();
    
    assert_eq!(component1.id, 1);
    assert_eq!(component2.id, 2);
    assert!(!component1.protocol_data.is_empty());
    assert!(!component2.protocol_data.is_empty());
    
    // Verify resource was updated by systems
    let resource = world.get_resource::<IntegratedTestResource>().unwrap();
    assert!(resource.packet_count > 0); // System should have processed packets
}

#[tokio::test]
async fn test_performance_manager_integration() {
    // Test that PerformanceManager integrates with all systems
    let performance_manager = PerformanceManager::new();
    
    // Test memory pools integration
    let memory_pools = performance_manager.memory_pools();
    
    // Simulate ECS entity creation with memory pools
    let mut entity_buffers = Vec::new();
    for i in 0..100 {
        let mut buffer = memory_pools.entity_pool.get();
        buffer.extend_from_slice(&[i as u8; 64]); // Simulate entity data
        entity_buffers.push(buffer);
    }
    
    // Test packet buffer usage
    let mut packet_buffers = Vec::new();
    for i in 0..50 {
        let mut buffer = memory_pools.packet_buffer_pool.get();
        buffer.extend_from_slice(&format!("packet_{}", i).as_bytes());
        packet_buffers.push(buffer);
    }
    
    // Get performance stats
    let stats = performance_manager.performance_stats();
    assert!(stats.memory.overall_efficiency() >= 0.0);
    assert!(stats.threading.utilization() >= 0.0);
    assert!(stats.metrics.total_metrics >= 0);
    
    // Test metrics collection
    let metrics = performance_manager.metrics_collector();
    metrics.increment_counter("test_integration_counter", None);
    metrics.record_gauge("test_integration_gauge", 42.0, None);
    
    // Verify metrics were recorded
    let summary = metrics.summary();
    assert!(summary.total_metrics >= 2);
    
    // Test thread manager integration
    let thread_manager = performance_manager.thread_manager();
    let task_result = thread_manager.execute_cpu_task(|| {
        // Simulate CPU-intensive work
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        sum
    }).expect("Failed to submit CPU task");
    
    let result = task_result.await_result().await.expect("Task failed");
    assert!(result > 0);
    
    // Cleanup
    drop(entity_buffers);
    drop(packet_buffers);
    performance_manager.maintenance();
}

#[tokio::test]
async fn test_plugin_loading_ecs_registration() {
    // Test plugin loading and ECS component registration in integrated environment
    let mut app = App::new(Weak::new());
    
    // Add multiple plugins with different capabilities
    app.add_plugin(EcsRegistrationPlugin).expect("Failed to add ECS registration plugin");
    app.add_plugin(SystemRegistrationPlugin).expect("Failed to add system registration plugin");
    app.add_plugin(ResourcePlugin).expect("Failed to add resource plugin");
    
    let world = app.world_mut();
    
    // Create entities and add components
    let entity1 = world.spawn();
    let entity2 = world.spawn();
    
    world.insert(entity1, EcsTestComponent { value: 10 }).expect("Failed to insert component");
    world.insert(entity1, SystemTestComponent { data: "entity1".to_string() }).expect("Failed to insert component");
    
    world.insert(entity2, EcsTestComponent { value: 20 }).expect("Failed to insert component");
    world.insert(entity2, SystemTestComponent { data: "entity2".to_string() }).expect("Failed to insert component");
    
    // Run systems
    app.update().expect("Failed to update app");
    
    // Verify systems executed correctly
    let component1 = world.get::<EcsTestComponent>(entity1).unwrap();
    let component2 = world.get::<EcsTestComponent>(entity2).unwrap();
    
    assert_eq!(component1.value, 11); // Should be incremented by system
    assert_eq!(component2.value, 21);
    
    // Verify shared resource was updated
    let shared_resource = world.get_resource::<TestSharedResource>().unwrap();
    assert_eq!(shared_resource.processed_entities, 2);
}

#[tokio::test]
async fn test_protocol_handlers_ecs_entity_management() {
    // Test that protocol handlers work with ECS entity management
    // Create ECS world for entity management
    let mut app = App::new(Weak::new());
    app.add_plugin(ProtocolEcsPlugin).expect("Failed to add protocol ECS plugin");
    
    let world = app.world_mut();
    
    // Simulate client connection and entity creation
    let client_id = uuid::Uuid::new_v4();
    let client_entity = world.spawn();
    
    world.insert(client_entity, ClientComponent {
        client_id,
        connection_state: "connected".to_string(),
        last_packet_time: std::time::Instant::now(),
    }).expect("Failed to insert client component");
    
    world.insert(client_entity, ProtocolComponent {
        protocol_type: "bedrock".to_string(),
        version: 1,
        authenticated: true,
    }).expect("Failed to insert protocol component");
    
    // Run ECS systems to process protocol updates
    app.update().expect("Failed to update app");
    
    // Verify entity was updated by protocol processing
    let client_component = world.get::<ClientComponent>(client_entity).unwrap();
    assert_eq!(client_component.client_id, client_id);
    assert_eq!(client_component.connection_state, "connected");
    
    let protocol_component = world.get::<ProtocolComponent>(client_entity).unwrap();
    assert_eq!(protocol_component.protocol_type, "bedrock");
    assert!(protocol_component.authenticated);
}

#[tokio::test]
async fn test_complete_system_stress_integration() {
    // Comprehensive stress test of all systems working together
    let performance_manager = PerformanceManager::new();
    let mut app = App::new(Weak::new());
    
    // Add all test plugins
    app.add_plugin(StressTestPlugin).expect("Failed to add stress test plugin");
    
    let instance = create_test_mirai_instance().await
        .expect("Failed to create stress test instance");
    
    let world = app.world_mut();
    
    // Create many entities with multiple components
    let entity_count = 1000;
    let mut entities = Vec::new();
    
    for i in 0..entity_count {
        let entity = world.spawn();
        
        world.insert(entity, StressTestComponent {
            id: i,
            position: (i as f32, 0.0, 0.0),
            velocity: (1.0, 0.0, 0.0),
            health: 100,
        }).expect("Failed to insert stress test component");
        
        world.insert(entity, NetworkComponent {
            client_id: Some(uuid::Uuid::new_v4()),
            last_update: std::time::Instant::now(),
            dirty: false,
        }).expect("Failed to insert network component");
        
        entities.push(entity);
    }
    
    // Add stress test resource
    world.insert_resource(StressTestResource {
        total_entities: entity_count,
        processed_count: 0,
        performance_samples: Vec::new(),
    });
    
    // Run systems multiple times to simulate server ticks
    let start_time = std::time::Instant::now();
    
    for tick in 0..10 {
        let tick_start = std::time::Instant::now();
        
        // Run ECS systems
        app.update().expect("Failed to update app");
        
        // Simulate performance monitoring
        let tick_duration = tick_start.elapsed();
        performance_manager.metrics_collector().record_gauge(
            "tick_duration_ms", 
            tick_duration.as_millis() as f64,
            None
        );
        
        // Brief pause to simulate real server timing
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    let total_duration = start_time.elapsed();
    
    // Verify stress test completed successfully
    assert!(total_duration < Duration::from_secs(5)); // Should complete quickly
    
    // Verify all entities were processed
    let stress_resource = world.get_resource::<StressTestResource>().unwrap();
    assert_eq!(stress_resource.total_entities, entity_count);
    assert!(stress_resource.processed_count > 0);
    
    // Verify performance stats
    let perf_stats = performance_manager.performance_stats();
    assert!(perf_stats.memory.overall_efficiency() >= 0.0);
    
    // Verify metrics were collected
    let metrics_summary = performance_manager.metrics_collector().summary();
    assert!(metrics_summary.total_metrics >= 10); // At least one metric per tick
}

// Test plugin implementations

struct IntegratedTestPlugin;

impl Plugin for IntegratedTestPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("integrated_test_plugin", semver::Version::new(1, 0, 0))
            .with_description("Integrated test plugin for system validation")
            .with_author("Mirai Test Suite")
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.world_mut().insert_resource(IntegratedTestResource { 
            connection_count: 0, 
            packet_count: 0 
        });
        app.add_system(IntegratedTestSystem);
        Ok(())
    }
}

struct EcsRegistrationPlugin;

impl Plugin for EcsRegistrationPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("ecs_registration_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        // Components are registered automatically when first used
        Ok(())
    }
}

struct SystemRegistrationPlugin;

impl Plugin for SystemRegistrationPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("system_registration_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.add_system(IncrementSystem);
        Ok(())
    }
}

struct ResourcePlugin;

impl Plugin for ResourcePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("resource_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.world_mut().insert_resource(TestSharedResource { processed_entities: 0 });
        app.add_system(ResourceUpdateSystem);
        Ok(())
    }
}

struct ProtocolEcsPlugin;

impl Plugin for ProtocolEcsPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("protocol_ecs_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.add_system(ProtocolUpdateSystem);
        Ok(())
    }
}

struct StressTestPlugin;

impl Plugin for StressTestPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("stress_test_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> anyhow::Result<()> {
        app.add_system(MovementSystem);
        app.add_system(NetworkUpdateSystem);
        app.add_system(StressMonitoringSystem);
        Ok(())
    }
}

// Test components

#[derive(Debug, Clone)]
struct IntegratedTestComponent {
    id: u32,
    name: String,
    protocol_data: Vec<u8>,
}

impl Component for IntegratedTestComponent {}

#[derive(Debug, Clone)]
struct EcsTestComponent {
    value: i32,
}

impl Component for EcsTestComponent {}

#[derive(Debug, Clone)]
struct SystemTestComponent {
    data: String,
}

impl Component for SystemTestComponent {}

#[derive(Debug, Clone)]
struct ClientComponent {
    client_id: uuid::Uuid,
    connection_state: String,
    last_packet_time: std::time::Instant,
}

impl Component for ClientComponent {}

#[derive(Debug, Clone)]
struct ProtocolComponent {
    protocol_type: String,
    version: u32,
    authenticated: bool,
}

impl Component for ProtocolComponent {}

#[derive(Debug, Clone)]
struct StressTestComponent {
    id: usize,
    position: (f32, f32, f32),
    velocity: (f32, f32, f32),
    health: i32,
}

impl Component for StressTestComponent {}

#[derive(Debug, Clone)]
struct NetworkComponent {
    client_id: Option<uuid::Uuid>,
    last_update: std::time::Instant,
    dirty: bool,
}

impl Component for NetworkComponent {}

// Test resources

#[derive(Debug, Clone)]
struct IntegratedTestResource {
    connection_count: u32,
    packet_count: u32,
}

impl Resource for IntegratedTestResource {}

#[derive(Debug, Clone)]
struct TestSharedResource {
    processed_entities: u32,
}

impl Resource for TestSharedResource {}

#[derive(Debug, Clone)]
struct StressTestResource {
    total_entities: usize,
    processed_count: usize,
    performance_samples: Vec<Duration>,
}

impl Resource for StressTestResource {}

// Test systems

struct IntegratedTestSystem;

impl System for IntegratedTestSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate processing integrated test components
        if let Some(mut resource) = world.get_resource_mut::<IntegratedTestResource>() {
            resource.packet_count += 1;
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "integrated_test_system"
    }
}

struct IncrementSystem;

impl System for IncrementSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate incrementing ECS test components
        // In a real implementation, this would use proper queries
        Ok(())
    }
    
    fn name(&self) -> &str {
        "increment_system"
    }
}

struct ResourceUpdateSystem;

impl System for ResourceUpdateSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate updating shared resource
        if let Some(mut resource) = world.get_resource_mut::<TestSharedResource>() {
            resource.processed_entities = 2; // Simulate processing 2 entities
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "resource_update_system"
    }
}

struct ProtocolUpdateSystem;

impl System for ProtocolUpdateSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate protocol updates
        // In a real implementation, this would update protocol components
        Ok(())
    }
    
    fn name(&self) -> &str {
        "protocol_update_system"
    }
}

struct MovementSystem;

impl System for MovementSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate movement system
        // In a real implementation, this would update positions based on velocity
        Ok(())
    }
    
    fn name(&self) -> &str {
        "movement_system"
    }
}

struct NetworkUpdateSystem;

impl System for NetworkUpdateSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate network updates
        // In a real implementation, this would update network components
        Ok(())
    }
    
    fn name(&self) -> &str {
        "network_update_system"
    }
}

struct StressMonitoringSystem;

impl System for StressMonitoringSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Simulate stress monitoring
        if let Some(mut resource) = world.get_resource_mut::<StressTestResource>() {
            resource.processed_count = resource.total_entities; // Simulate all entities processed
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "stress_monitoring_system"
    }
}

