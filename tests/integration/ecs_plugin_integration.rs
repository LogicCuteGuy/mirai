//! Integration tests for ECS and Plugin system integration
//! 
//! Tests that validate the merged ECS framework and plugin system
//! work correctly with the existing mirai architecture.

use super::*;
use mirai_core::{
    App, World, Entity, Component, Plugin,
    ecs::{EntityManager, ComponentManager, SystemScheduler},
    plugin::{PluginRegistry, PluginLifecycle}
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_ecs_plugin_system_integration() {
    let mut app = App::new();
    
    // Add test plugin with ECS components
    app.add_plugin(EcsTestPlugin);
    
    // Build and verify the app
    let instance = app.build_instance().await
        .expect("Failed to build instance with ECS plugin");
    
    // Verify ECS components are registered
    let world = instance.world();
    assert!(world.has_component::<TestEcsComponent>());
    
    // Create test entities
    let entity1 = world.spawn_entity();
    let entity2 = world.spawn_entity();
    
    world.add_component(entity1, TestEcsComponent { health: 100, name: "Player1".to_string() });
    world.add_component(entity2, TestEcsComponent { health: 50, name: "Player2".to_string() });
    
    // Run systems
    world.run_systems();
    
    // Verify system execution
    let component1 = world.get_component::<TestEcsComponent>(entity1).unwrap();
    let component2 = world.get_component::<TestEcsComponent>(entity2).unwrap();
    
    assert_eq!(component1.health, 101); // Health system should increment
    assert_eq!(component2.health, 51);
}

#[tokio::test]
async fn test_plugin_dependency_resolution() {
    let mut app = App::new();
    
    // Add plugins with dependencies
    app.add_plugin(DependentPlugin);
    app.add_plugin(BasePlugin); // Should be loaded first due to dependency
    
    let instance = app.build_instance().await
        .expect("Failed to build instance with dependent plugins");
    
    // Verify plugins were loaded in correct order
    let registry = instance.plugin_registry();
    assert!(registry.is_plugin_loaded("base_plugin"));
    assert!(registry.is_plugin_loaded("dependent_plugin"));
    
    // Verify dependency was satisfied
    let load_order = registry.get_load_order();
    let base_index = load_order.iter().position(|name| name == "base_plugin").unwrap();
    let dependent_index = load_order.iter().position(|name| name == "dependent_plugin").unwrap();
    assert!(base_index < dependent_index);
}

#[tokio::test]
async fn test_mirai_compatibility_layer() {
    let mut app = App::new();
    app.add_plugin(CompatibilityTestPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build instance with compatibility plugin");
    
    // Test that existing mirai APIs still work
    assert!(instance.is_running());
    
    // Test BedrockClient compatibility
    let client_count = instance.get_client_count();
    assert_eq!(client_count, 0); // No clients connected yet
    
    // Test Service compatibility
    let services = instance.get_services();
    assert!(!services.is_empty()); // Should have at least basic services
}

#[tokio::test]
async fn test_ecs_world_integration_with_mirai() {
    let mut app = App::new();
    app.add_plugin(WorldIntegrationPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build instance with world integration");
    
    let world = instance.world();
    
    // Test that ECS world integrates with mirai's existing entity system
    let mirai_entity = instance.create_mirai_entity("test_entity");
    let ecs_entity = world.spawn_entity();
    
    // Test bridge between systems
    world.link_mirai_entity(ecs_entity, mirai_entity);
    
    let linked_mirai = world.get_linked_mirai_entity(ecs_entity);
    assert!(linked_mirai.is_some());
    assert_eq!(linked_mirai.unwrap(), mirai_entity);
}

#[test]
fn test_plugin_lifecycle_management() {
    let mut registry = PluginRegistry::new();
    
    // Register plugins
    registry.register_plugin(Box::new(LifecycleTestPlugin::new()));
    
    // Initialize plugins
    registry.initialize_all().expect("Failed to initialize plugins");
    
    // Verify plugin state
    let plugin = registry.get_plugin("lifecycle_test").unwrap();
    assert!(plugin.is_initialized());
    
    // Shutdown plugins
    registry.shutdown_all().expect("Failed to shutdown plugins");
    assert!(!plugin.is_initialized());
}

// Test plugin implementations

struct EcsTestPlugin;

impl Plugin for EcsTestPlugin {
    fn name(&self) -> &'static str {
        "ecs_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestEcsComponent>();
        app.add_system(health_increment_system);
    }
}

struct BasePlugin;

impl Plugin for BasePlugin {
    fn name(&self) -> &'static str {
        "base_plugin"
    }
    
    fn build(&self, app: &mut App) {
        // Base functionality
    }
}

struct DependentPlugin;

impl Plugin for DependentPlugin {
    fn name(&self) -> &'static str {
        "dependent_plugin"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["base_plugin"]
    }
    
    fn build(&self, app: &mut App) {
        // Dependent functionality
    }
}

struct CompatibilityTestPlugin;

impl Plugin for CompatibilityTestPlugin {
    fn name(&self) -> &'static str {
        "compatibility_test"
    }
    
    fn build(&self, app: &mut App) {
        // Test compatibility features
    }
    
    fn configure_mirai(&self, instance: &mut mirai_core::Instance) {
        // Configure mirai-specific features
        instance.add_service("test_service", Box::new(TestService));
    }
}

struct WorldIntegrationPlugin;

impl Plugin for WorldIntegrationPlugin {
    fn name(&self) -> &'static str {
        "world_integration"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().enable_mirai_bridge();
    }
}

struct LifecycleTestPlugin {
    initialized: Arc<Mutex<bool>>,
}

impl LifecycleTestPlugin {
    fn new() -> Self {
        Self {
            initialized: Arc::new(Mutex::new(false)),
        }
    }
    
    fn is_initialized(&self) -> bool {
        *self.initialized.try_lock().unwrap()
    }
}

impl Plugin for LifecycleTestPlugin {
    fn name(&self) -> &'static str {
        "lifecycle_test"
    }
    
    fn build(&self, app: &mut App) {
        // Plugin build logic
    }
}

impl PluginLifecycle for LifecycleTestPlugin {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.initialized.try_lock().unwrap() = true;
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.initialized.try_lock().unwrap() = false;
        Ok(())
    }
}

// Test components and systems

#[derive(Debug, Clone)]
struct TestEcsComponent {
    health: i32,
    name: String,
}

impl Component for TestEcsComponent {}

fn health_increment_system(world: &mut World) {
    for mut component in world.query::<&mut TestEcsComponent>() {
        component.health += 1;
    }
}

// Test service for compatibility testing
struct TestService;

impl mirai_core::Service for TestService {
    fn name(&self) -> &'static str {
        "test_service"
    }
    
    fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}