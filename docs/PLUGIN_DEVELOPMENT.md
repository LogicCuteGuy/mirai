# Plugin Development Guide for Mirai

This comprehensive guide covers how to develop plugins for the Mirai Minecraft server using the merged ECS architecture and plugin system. The merged system combines the best features from both the original mirai project and the minecraft-server crates, providing a powerful and flexible plugin development environment.

## Table of Contents

1. [Overview](#overview)
2. [Plugin Architecture](#plugin-architecture)
3. [Creating Your First Plugin](#creating-your-first-plugin)
4. [ECS Integration](#ecs-integration)
5. [Advanced Plugin Development](#advanced-plugin-development)
6. [Migration Guide](#migration-guide)
7. [Plugin Templates](#plugin-templates)
8. [Best Practices](#best-practices)
9. [Testing Plugins](#testing-plugins)
10. [Performance Considerations](#performance-considerations)
11. [Troubleshooting](#troubleshooting)

## Overview

The Mirai plugin system is built on top of a powerful Entity Component System (ECS) architecture that has been merged and enhanced from multiple Minecraft server implementations. This system provides:

- **ECS Architecture**: Efficient entity management with components and systems
- **Plugin Lifecycle Management**: Automatic plugin loading, initialization, and cleanup
- **Event-Driven Architecture**: Comprehensive event system for inter-plugin communication
- **Performance Optimization**: Built-in performance monitoring and optimization features
- **Mirai Integration**: Seamless integration with Mirai's existing infrastructure

### Key Features

- **Bevy-like Plugin Interface**: Familiar API for developers coming from game development
- **Hot Reloading**: Support for plugin hot reloading during development
- **Dependency Management**: Automatic plugin dependency resolution
- **Configuration Management**: Unified configuration system with runtime updates
- **Performance Monitoring**: Built-in metrics and performance tracking
- **Compatibility Layer**: Support for existing Mirai extensions

## Plugin Architecture

### Core Components

The plugin system consists of several key components:

```rust
// Core plugin trait
pub trait Plugin: Send + Sync {
    fn info(&self) -> PluginInfo;
    fn build(&self, app: &mut App) -> Result<()>;
    fn configure_mirai(&self, instance: &mut Instance) -> Result<()> { Ok(()) }
}

// Plugin information
pub struct PluginInfo {
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
}

// Main application structure
pub struct App {
    world: World,
    instance: Instance,
    plugins: Vec<Box<dyn Plugin>>,
    config: UnifiedConfig,
}
```

### ECS Components

The ECS system provides the foundation for all plugin functionality:

- **Entities**: Unique identifiers for game objects
- **Components**: Data containers attached to entities
- **Systems**: Logic that operates on entities with specific components
- **Resources**: Global data shared across systems
- **Events**: Messages for communication between systems

## Creating Your First Plugin

### Basic Plugin Structure

Here's a minimal plugin implementation:

```rust
use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, Component, Resource, System};
use anyhow::Result;

pub struct MyFirstPlugin;

impl Plugin for MyFirstPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("my_first_plugin", semver::Version::new(1, 0, 0))
            .with_description("My first Mirai plugin")
            .with_author("Your Name")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        // Add resources
        app.insert_resource(MyPluginConfig::default());
        
        // Add systems
        app.add_system(MyPluginSystem::new());
        
        tracing::info!("My first plugin initialized!");
        Ok(())
    }
}

// Plugin configuration
#[derive(Debug, Clone, Resource)]
pub struct MyPluginConfig {
    pub enabled: bool,
    pub update_interval: Duration,
}

impl Default for MyPluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: Duration::from_secs(1),
        }
    }
}

// Plugin system
pub struct MyPluginSystem {
    last_update: Instant,
}

impl MyPluginSystem {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
        }
    }
}

impl System for MyPluginSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<MyPluginConfig>().cloned();
        if let Some(config) = config {
            if !config.enabled {
                return Ok(());
            }
            
            if self.last_update.elapsed() >= config.update_interval {
                // Your plugin logic here
                tracing::debug!("My plugin is running!");
                self.last_update = Instant::now();
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "my_plugin_system"
    }
}
```

### Plugin Registration

To register your plugin with Mirai:

```rust
// In your main application or plugin loader
fn main() {
    let mut app = App::new();
    
    // Add your plugin
    app.add_plugin(MyFirstPlugin);
    
    // Run the server
    app.run();
}
```
## 
ECS Integration

### Working with Entities

Entities are the fundamental building blocks in the ECS system:

```rust
use mirai::core::ecs::{World, EntityId, EntityManager};
use mirai::level::Position;

// Spawning entities
fn spawn_custom_entity(world: &mut World) -> Result<EntityId> {
    let entity_id = world.spawn_entity();
    
    // Add components to the entity
    world.insert_component(entity_id, Position::new(0.0, 64.0, 0.0, 0.0, 0.0))?;
    world.insert_component(entity_id, MyCustomComponent::new())?;
    
    Ok(entity_id)
}

// Querying entities
fn find_entities_with_component(world: &World) -> Vec<EntityId> {
    world.query::<&MyCustomComponent>()
        .iter()
        .map(|(entity_id, _)| entity_id)
        .collect()
}
```

### Creating Components

Components store data associated with entities:

```rust
use mirai::core::ecs::Component;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Component)]
pub struct MyCustomComponent {
    pub value: i32,
    pub name: String,
    pub active: bool,
}

impl MyCustomComponent {
    pub fn new() -> Self {
        Self {
            value: 0,
            name: String::new(),
            active: true,
        }
    }
    
    pub fn with_value(mut self, value: i32) -> Self {
        self.value = value;
        self
    }
}
```

### System Development

Systems contain the logic that operates on entities:

```rust
use mirai::core::ecs::{System, World, EntityId};

pub struct MyCustomSystem {
    processed_count: u64,
}

impl MyCustomSystem {
    pub fn new() -> Self {
        Self {
            processed_count: 0,
        }
    }
}

impl System for MyCustomSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Query entities with specific components
        let entities: Vec<EntityId> = world.query::<&MyCustomComponent>()
            .iter()
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        for entity_id in entities {
            if let Some(mut component) = world.get_component_mut::<MyCustomComponent>(entity_id) {
                // Process the component
                component.value += 1;
                self.processed_count += 1;
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "my_custom_system"
    }
    
    fn dependencies(&self) -> Vec<String> {
        // List systems this system depends on
        vec!["some_other_system".to_string()]
    }
    
    fn is_parallel_safe(&self) -> bool {
        // Return true if this system can run in parallel
        true
    }
}
```

### Resources and Global State

Resources provide global state accessible to all systems:

```rust
use mirai::core::ecs::Resource;

#[derive(Debug, Resource)]
pub struct MyGlobalState {
    pub total_entities: usize,
    pub server_uptime: Duration,
    pub custom_data: HashMap<String, String>,
}

impl MyGlobalState {
    pub fn new() -> Self {
        Self {
            total_entities: 0,
            server_uptime: Duration::ZERO,
            custom_data: HashMap::new(),
        }
    }
}

// Using resources in systems
impl System for MyCustomSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Access resource
        if let Some(mut state) = world.get_resource_mut::<MyGlobalState>() {
            state.total_entities = world.entity_count();
        }
        
        Ok(())
    }
}
```

## Advanced Plugin Development

### Event System

The event system enables communication between plugins and systems:

```rust
use mirai::core::ecs::Event;

// Define custom events
#[derive(Debug, Clone)]
pub struct MyCustomEvent {
    pub entity: EntityId,
    pub event_type: MyEventType,
    pub data: HashMap<String, String>,
}

impl Event for MyCustomEvent {}

#[derive(Debug, Clone)]
pub enum MyEventType {
    EntityCreated,
    EntityUpdated,
    EntityDestroyed,
}

// Sending events
fn send_event(world: &mut World, entity: EntityId) {
    let event = MyCustomEvent {
        entity,
        event_type: MyEventType::EntityCreated,
        data: HashMap::new(),
    };
    
    world.send_event(event);
}

// Handling events in systems
impl System for MyEventHandlerSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        for event in world.get_events::<MyCustomEvent>() {
            match event.event_type {
                MyEventType::EntityCreated => {
                    tracing::info!("Entity {} was created", event.entity);
                }
                MyEventType::EntityUpdated => {
                    tracing::debug!("Entity {} was updated", event.entity);
                }
                MyEventType::EntityDestroyed => {
                    tracing::info!("Entity {} was destroyed", event.entity);
                }
            }
        }
        
        Ok(())
    }
}
```

### Configuration Management

The unified configuration system supports runtime updates:

```rust
use mirai::core::config::UnifiedConfig;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginConfig {
    pub enabled: bool,
    pub max_entities: usize,
    pub update_frequency: f64,
    pub features: Vec<String>,
}

impl Default for MyPluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entities: 100,
            update_frequency: 20.0, // 20 TPS
            features: vec!["basic".to_string()],
        }
    }
}

// Runtime configuration updates
fn update_plugin_config(world: &mut World, new_config: MyPluginConfig) -> Result<()> {
    if let Some(mut config) = world.get_resource_mut::<MyPluginConfig>() {
        *config = new_config;
        
        // Send configuration change event
        world.send_event(ConfigurationChangedEvent {
            plugin_name: "my_plugin".to_string(),
        });
    }
    
    Ok(())
}
```

### Performance Monitoring

Built-in performance monitoring helps optimize your plugins:

```rust
use mirai::core::performance::{PerformanceMetrics, PerformanceSample};

pub struct MyPerformanceAwareSystem {
    metrics: PerformanceMetrics,
}

impl System for MyPerformanceAwareSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let start_time = Instant::now();
        
        // Your system logic here
        self.process_entities(world)?;
        
        // Record performance metrics
        let processing_time = start_time.elapsed();
        self.metrics.record_processing_time(processing_time);
        
        // Add performance sample
        let sample = PerformanceSample {
            timestamp: Instant::now(),
            processing_time,
            entities_processed: self.get_processed_count(),
            memory_usage: self.estimate_memory_usage(),
        };
        
        if let Some(mut global_metrics) = world.get_resource_mut::<GlobalPerformanceMetrics>() {
            global_metrics.add_sample("my_system", sample);
        }
        
        Ok(())
    }
}
```

## Migration Guide

### From Original Mirai Extensions

If you have existing Mirai extensions, here's how to migrate them:

```rust
// Old Mirai extension
impl MiraiExtension for MyOldExtension {
    fn on_player_join(&mut self, player: &Player) {
        // Old logic
    }
}

// New plugin system
impl Plugin for MyMigratedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        app.add_system(PlayerJoinSystem::new());
        Ok(())
    }
}

pub struct PlayerJoinSystem;

impl System for PlayerJoinSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Handle player join events
        for event in world.get_events::<PlayerJoinEvent>() {
            // Migrated logic
            self.handle_player_join(world, &event.player)?;
        }
        Ok(())
    }
}
```

### From minecraft-server-plugins

Plugins from the minecraft-server-plugins crate can be adapted:

```rust
// Original minecraft-server plugin
impl MinecraftPlugin for MyOriginalPlugin {
    fn build_plugin(&self, app: &mut App) -> Result<()> {
        // Original logic
    }
}

// Adapted for Mirai
impl Plugin for MyAdaptedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        // Adapted logic with Mirai integration
        self.configure_mirai_integration(app)?;
        
        // Original plugin logic
        self.build_original_functionality(app)?;
        
        Ok(())
    }
    
    fn configure_mirai(&self, instance: &mut Instance) -> Result<()> {
        // Mirai-specific configuration
        Ok(())
    }
}
```

## Plugin Templates

### Basic Plugin Template

Use this template for simple plugins:

```rust
//! Basic Plugin Template
//! 
//! Copy this template and modify it to create your own plugin.

use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, Component, Resource, System, EntityId};
use anyhow::Result;
use std::time::{Duration, Instant};

pub struct BasicPlugin;

impl Plugin for BasicPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("basic_plugin", semver::Version::new(1, 0, 0))
            .with_description("A basic plugin template")
            .with_author("Your Name")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        app.insert_resource(BasicConfig::default())
           .add_system(BasicSystem::new());
        
        tracing::info!("Basic plugin initialized");
        Ok(())
    }
}

#[derive(Debug, Clone, Resource)]
pub struct BasicConfig {
    pub enabled: bool,
    pub update_interval: Duration,
}

impl Default for BasicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct BasicComponent {
    pub value: i32,
    pub name: String,
}

pub struct BasicSystem {
    last_update: Instant,
}

impl BasicSystem {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
        }
    }
}

impl System for BasicSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<BasicConfig>().cloned();
        if let Some(config) = config {
            if !config.enabled {
                return Ok(());
            }
            
            if self.last_update.elapsed() >= config.update_interval {
                // Your plugin logic here
                self.last_update = Instant::now();
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "basic_system"
    }
}
```

### Advanced Plugin Template

For more complex plugins, use the advanced template (see `examples/advanced_plugin_template.rs`).

## Best Practices

### Performance Guidelines

1. **Batch Operations**: Process multiple entities in batches rather than one at a time
2. **Use Parallel Systems**: Mark systems as parallel-safe when possible
3. **Minimize Allocations**: Reuse data structures and avoid unnecessary allocations
4. **Profile Regularly**: Use the built-in performance monitoring tools

```rust
impl System for OptimizedSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Batch entity processing
        let entities: Vec<EntityId> = world.query::<&MyComponent>()
            .iter()
            .take(100) // Process in batches of 100
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        // Process batch
        for entity_id in entities {
            self.process_entity(world, entity_id)?;
        }
        
        Ok(())
    }
    
    fn is_parallel_safe(&self) -> bool {
        true // Enable parallel execution
    }
}
```

### Error Handling

Always handle errors gracefully:

```rust
impl System for RobustSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let entities: Vec<EntityId> = world.query::<&MyComponent>()
            .iter()
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        for entity_id in entities {
            if let Err(e) = self.process_entity(world, entity_id) {
                tracing::warn!("Failed to process entity {}: {}", entity_id, e);
                // Continue processing other entities
            }
        }
        
        Ok(())
    }
}
```

### Resource Management

Properly manage resources and cleanup:

```rust
impl Plugin for ResourceAwarePlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        // Initialize resources
        app.insert_resource(MyResource::new());
        
        // Add cleanup system
        app.add_system(CleanupSystem::new());
        
        Ok(())
    }
}

pub struct CleanupSystem {
    last_cleanup: Instant,
}

impl System for CleanupSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        if self.last_cleanup.elapsed() > Duration::from_secs(60) {
            // Perform cleanup
            self.cleanup_expired_entities(world)?;
            self.last_cleanup = Instant::now();
        }
        
        Ok(())
    }
}
```

## Testing Plugins

### Unit Testing

Test your plugin components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mirai::core::testing::TestWorld;
    
    #[test]
    fn test_basic_component() {
        let component = BasicComponent {
            value: 42,
            name: "test".to_string(),
        };
        
        assert_eq!(component.value, 42);
        assert_eq!(component.name, "test");
    }
    
    #[test]
    fn test_basic_system() {
        let mut world = TestWorld::new()
            .with_resource(BasicConfig::default())
            .build();
        
        let mut system = BasicSystem::new();
        
        // Run the system
        system.run(&mut world).unwrap();
        
        // Verify expected behavior
        // Add your assertions here
    }
}
```

### Integration Testing

Test plugin integration with the full system:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mirai::core::testing::{TestApp, TestWorld};
    
    #[tokio::test]
    async fn test_plugin_integration() {
        let mut app = TestApp::new();
        app.add_plugin(BasicPlugin);
        
        // Run for a few ticks
        for _ in 0..10 {
            app.update().await;
        }
        
        // Verify plugin behavior
        assert!(app.has_resource::<BasicConfig>());
    }
}
```

## Performance Considerations

### System Ordering

Order systems for optimal performance:

```rust
impl Plugin for OptimizedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        // Input systems first
        app.add_system_to_stage(InputSystem::new(), "input");
        
        // Logic systems in the middle
        app.add_system_to_stage(LogicSystem::new(), "update");
        
        // Output systems last
        app.add_system_to_stage(OutputSystem::new(), "output");
        
        Ok(())
    }
}
```

### Memory Management

Use object pools for frequently created objects:

```rust
use mirai::core::memory::ObjectPool;

pub struct PooledSystem {
    entity_pool: ObjectPool<EntityData>,
}

impl System for PooledSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Get object from pool instead of allocating
        let mut entity_data = self.entity_pool.get();
        
        // Use the object
        entity_data.process();
        
        // Return to pool
        self.entity_pool.return_object(entity_data);
        
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **Plugin Not Loading**: Check plugin registration and dependencies
2. **System Not Running**: Verify system dependencies and stage configuration
3. **Performance Issues**: Use profiling tools and check system ordering
4. **Memory Leaks**: Ensure proper cleanup in systems

### Debugging Tools

Use the built-in debugging features:

```rust
// Enable debug logging
tracing::debug!("Plugin state: {:?}", self.state);

// Performance profiling
let _span = tracing::info_span!("my_system_processing").entered();

// Memory usage tracking
if let Some(metrics) = world.get_resource::<PerformanceMetrics>() {
    tracing::info!("Memory usage: {} bytes", metrics.memory_usage);
}
```

### Getting Help

- Check the Mirai documentation
- Review example plugins in the `examples/` directory
- Use the built-in diagnostic tools
- Enable debug logging for detailed information

## Working Examples

### Complete Working Plugin Example

Here's a complete, tested plugin that demonstrates all the concepts covered in this guide:

```rust
//! Complete Working Plugin Example
//! This plugin demonstrates a fully functional implementation with all features

use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, Component, Resource, System, EntityId};
use mirai::core::instance::Instance;
use mirai::events::{PlayerJoinEvent, PlayerLeaveEvent, BlockPlaceEvent};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct WorkingExamplePlugin;

impl Plugin for WorkingExamplePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("working_example", semver::Version::new(1, 0, 0))
            .with_description("A complete working plugin example")
            .with_author("Mirai Team")
            .with_dependencies(vec!["mirai_core".to_string()])
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        // Add configuration
        app.insert_resource(WorkingConfig::default());
        
        // Add state tracking
        app.insert_resource(WorkingState::new());
        
        // Add systems in proper order
        app.add_system(PlayerTrackingSystem::new());
        app.add_system(BlockAnalysisSystem::new());
        app.add_system(StatisticsSystem::new());
        app.add_system(CleanupSystem::new());
        
        tracing::info!("Working example plugin loaded successfully");
        Ok(())
    }
    
    fn configure_mirai(&self, instance: &mut Instance) -> Result<()> {
        // Configure Mirai-specific features
        instance.register_event_handler("player_join", Box::new(|event| {
            tracing::info!("Player joined via Mirai handler: {:?}", event);
        }));
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct WorkingConfig {
    pub track_players: bool,
    pub analyze_blocks: bool,
    pub cleanup_interval: Duration,
    pub max_tracked_entities: usize,
}

impl Default for WorkingConfig {
    fn default() -> Self {
        Self {
            track_players: true,
            analyze_blocks: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_tracked_entities: 1000,
        }
    }
}

#[derive(Debug, Resource)]
pub struct WorkingState {
    pub players_online: HashMap<String, Instant>,
    pub blocks_placed: u64,
    pub plugin_start_time: Instant,
    pub last_cleanup: Instant,
}

impl WorkingState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            players_online: HashMap::new(),
            blocks_placed: 0,
            plugin_start_time: now,
            last_cleanup: now,
        }
    }
    
    pub fn uptime(&self) -> Duration {
        self.plugin_start_time.elapsed()
    }
}

#[derive(Debug, Clone, Component)]
pub struct TrackedPlayer {
    pub name: String,
    pub join_time: Instant,
    pub blocks_placed: u32,
    pub last_activity: Instant,
}

impl TrackedPlayer {
    pub fn new(name: String) -> Self {
        let now = Instant::now();
        Self {
            name,
            join_time: now,
            blocks_placed: 0,
            last_activity: now,
        }
    }
    
    pub fn session_duration(&self) -> Duration {
        self.join_time.elapsed()
    }
    
    pub fn time_since_activity(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

pub struct PlayerTrackingSystem {
    last_run: Instant,
}

impl PlayerTrackingSystem {
    pub fn new() -> Self {
        Self {
            last_run: Instant::now(),
        }
    }
}

impl System for PlayerTrackingSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<WorkingConfig>().cloned();
        if let Some(config) = config {
            if !config.track_players {
                return Ok(());
            }
        }
        
        // Handle player join events
        for event in world.get_events::<PlayerJoinEvent>() {
            if let Some(mut state) = world.get_resource_mut::<WorkingState>() {
                state.players_online.insert(event.player.name().to_string(), Instant::now());
            }
            
            // Create tracked player component
            let tracked_player = TrackedPlayer::new(event.player.name().to_string());
            
            // In a real implementation, you'd attach this to the player entity
            tracing::info!("Started tracking player: {}", event.player.name());
        }
        
        // Handle player leave events
        for event in world.get_events::<PlayerLeaveEvent>() {
            if let Some(mut state) = world.get_resource_mut::<WorkingState>() {
                if let Some(join_time) = state.players_online.remove(event.player.name()) {
                    let session_duration = join_time.elapsed();
                    tracing::info!("Player {} left after {:?}", event.player.name(), session_duration);
                }
            }
        }
        
        self.last_run = Instant::now();
        Ok(())
    }
    
    fn name(&self) -> &str {
        "player_tracking"
    }
}

pub struct BlockAnalysisSystem {
    last_run: Instant,
}

impl BlockAnalysisSystem {
    pub fn new() -> Self {
        Self {
            last_run: Instant::now(),
        }
    }
}

impl System for BlockAnalysisSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<WorkingConfig>().cloned();
        if let Some(config) = config {
            if !config.analyze_blocks {
                return Ok(());
            }
        }
        
        // Handle block place events
        for event in world.get_events::<BlockPlaceEvent>() {
            if let Some(mut state) = world.get_resource_mut::<WorkingState>() {
                state.blocks_placed += 1;
                
                // Update player's block count if they're tracked
                if let Some(join_time) = state.players_online.get_mut(event.player.name()) {
                    *join_time = Instant::now(); // Update last activity
                }
            }
            
            tracing::debug!("Block placed by {}: {:?} at {:?}", 
                event.player.name(), event.block_type, event.position);
        }
        
        self.last_run = Instant::now();
        Ok(())
    }
    
    fn name(&self) -> &str {
        "block_analysis"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["player_tracking".to_string()]
    }
}

pub struct StatisticsSystem {
    last_report: Instant,
    report_interval: Duration,
}

impl StatisticsSystem {
    pub fn new() -> Self {
        Self {
            last_report: Instant::now(),
            report_interval: Duration::from_secs(60), // Report every minute
        }
    }
}

impl System for StatisticsSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        if self.last_report.elapsed() < self.report_interval {
            return Ok(());
        }
        
        if let Some(state) = world.get_resource::<WorkingState>() {
            tracing::info!("Plugin Statistics:");
            tracing::info!("  Uptime: {:?}", state.uptime());
            tracing::info!("  Players Online: {}", state.players_online.len());
            tracing::info!("  Total Blocks Placed: {}", state.blocks_placed);
            tracing::info!("  Last Cleanup: {:?} ago", state.last_cleanup.elapsed());
        }
        
        self.last_report = Instant::now();
        Ok(())
    }
    
    fn name(&self) -> &str {
        "statistics"
    }
}

pub struct CleanupSystem {
    last_cleanup: Instant,
}

impl CleanupSystem {
    pub fn new() -> Self {
        Self {
            last_cleanup: Instant::now(),
        }
    }
}

impl System for CleanupSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<WorkingConfig>().cloned();
        if let Some(config) = config {
            if self.last_cleanup.elapsed() < config.cleanup_interval {
                return Ok(());
            }
        }
        
        if let Some(mut state) = world.get_resource_mut::<WorkingState>() {
            // Remove inactive players (example: offline for more than 1 hour)
            let inactive_threshold = Duration::from_secs(3600);
            let inactive_players: Vec<String> = state.players_online
                .iter()
                .filter(|(_, &join_time)| join_time.elapsed() > inactive_threshold)
                .map(|(name, _)| name.clone())
                .collect();
            
            for player_name in inactive_players {
                state.players_online.remove(&player_name);
                tracing::debug!("Cleaned up inactive player: {}", player_name);
            }
            
            state.last_cleanup = Instant::now();
        }
        
        self.last_cleanup = Instant::now();
        Ok(())
    }
    
    fn name(&self) -> &str {
        "cleanup"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["statistics".to_string()]
    }
}

// Helper functions
impl WorkingExamplePlugin {
    pub fn get_statistics(world: &World) -> Option<PluginStatistics> {
        let state = world.get_resource::<WorkingState>()?;
        
        Some(PluginStatistics {
            uptime: state.uptime(),
            players_online: state.players_online.len(),
            blocks_placed: state.blocks_placed,
            last_cleanup: state.last_cleanup.elapsed(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PluginStatistics {
    pub uptime: Duration,
    pub players_online: usize,
    pub blocks_placed: u64,
    pub last_cleanup: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirai::core::testing::TestWorld;
    
    #[test]
    fn test_working_plugin() {
        let mut world = TestWorld::new()
            .with_resource(WorkingConfig::default())
            .with_resource(WorkingState::new())
            .build();
        
        let mut system = PlayerTrackingSystem::new();
        system.run(&mut world).unwrap();
        
        // Verify system ran without errors
        assert!(world.has_resource::<WorkingState>());
    }
    
    #[test]
    fn test_tracked_player() {
        let player = TrackedPlayer::new("TestPlayer".to_string());
        assert_eq!(player.name, "TestPlayer");
        assert_eq!(player.blocks_placed, 0);
        assert!(player.session_duration() < Duration::from_secs(1));
    }
}
```

### Real-World Plugin Examples

#### 1. Economy Plugin

```rust
use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, Component, Resource, System};
use std::collections::HashMap;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("economy", semver::Version::new(1, 0, 0))
            .with_description("Player economy system")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        app.insert_resource(EconomyState::new())
           .add_system(EconomySystem::new())
           .add_system(TransactionSystem::new());
        Ok(())
    }
}

#[derive(Debug, Resource)]
pub struct EconomyState {
    pub player_balances: HashMap<String, f64>,
    pub transaction_history: Vec<Transaction>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub from: Option<String>,
    pub to: String,
    pub amount: f64,
    pub reason: String,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Component)]
pub struct PlayerWallet {
    pub balance: f64,
    pub last_transaction: std::time::Instant,
}
```

#### 2. Protection Plugin

```rust
use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, Component, System};
use mirai::level::Position;

pub struct ProtectionPlugin;

impl Plugin for ProtectionPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("protection", semver::Version::new(1, 0, 0))
            .with_description("Area protection system")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        app.insert_resource(ProtectionManager::new())
           .add_system(ProtectionSystem::new());
        Ok(())
    }
}

#[derive(Debug, Component)]
pub struct ProtectedArea {
    pub owner: String,
    pub min_pos: Position,
    pub max_pos: Position,
    pub permissions: Vec<String>,
}

impl ProtectedArea {
    pub fn contains(&self, pos: &Position) -> bool {
        pos.x >= self.min_pos.x && pos.x <= self.max_pos.x &&
        pos.y >= self.min_pos.y && pos.y <= self.max_pos.y &&
        pos.z >= self.min_pos.z && pos.z <= self.max_pos.z
    }
}
```

This guide provides a comprehensive foundation for developing plugins in the merged Mirai system. The combination of ECS architecture, performance optimization, and Mirai integration creates a powerful platform for extending Minecraft server functionality.