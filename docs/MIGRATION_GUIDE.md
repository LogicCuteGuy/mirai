# Migration Guide: From Legacy Systems to Merged Mirai

This guide helps developers migrate existing code to the new merged Mirai system that combines the best features from both the original mirai project and the minecraft-server crates.

## Overview

The merged system provides:
- Enhanced ECS architecture
- Unified plugin system
- Improved performance optimizations
- Better configuration management
- Comprehensive event system

## Migration Paths

### 1. From Original Mirai Extensions

#### Legacy Extension Pattern

```rust
// Old Mirai extension
pub struct LegacyExtension {
    config: ExtensionConfig,
}

impl MiraiExtension for LegacyExtension {
    fn on_player_join(&mut self, player: &Player) {
        // Handle player join
        println!("Player {} joined", player.name());
    }
    
    fn on_block_place(&mut self, player: &Player, block: &Block, position: Position) {
        // Handle block placement
        if self.config.log_block_changes {
            println!("Player {} placed block at {:?}", player.name(), position);
        }
    }
    
    fn update(&mut self, instance: &Instance) {
        // Regular update logic
        self.cleanup_expired_data();
    }
}
```

#### Migrated Plugin

```rust
// New plugin system
use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::ecs::{World, System, Resource, Event};
use mirai::events::{PlayerJoinEvent, BlockPlaceEvent};

pub struct MigratedPlugin {
    config: MigratedConfig,
}

impl Plugin for MigratedPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("migrated_plugin", semver::Version::new(2, 0, 0))
            .with_description("Migrated from legacy extension")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        // Add configuration as resource
        app.insert_resource(self.config.clone());
        
        // Add event handling systems
        app.add_system(PlayerJoinHandler::new());
        app.add_system(BlockPlaceHandler::new());
        app.add_system(UpdateSystem::new());
        
        Ok(())
    }
}

// Event handling systems
pub struct PlayerJoinHandler;

impl System for PlayerJoinHandler {
    fn run(&mut self, world: &mut World) -> Result<()> {
        for event in world.get_events::<PlayerJoinEvent>() {
            println!("Player {} joined", event.player.name());
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "player_join_handler" }
}

pub struct BlockPlaceHandler;

impl System for BlockPlaceHandler {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let config = world.get_resource::<MigratedConfig>();
        
        for event in world.get_events::<BlockPlaceEvent>() {
            if let Some(config) = config {
                if config.log_block_changes {
                    println!("Player {} placed block at {:?}", 
                        event.player.name(), event.position);
                }
            }
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "block_place_handler" }
}
```

### 2. From minecraft-server-plugins

#### Original Plugin

```rust
// Original minecraft-server plugin
use minecraft_server_plugins::{Plugin, PluginInfo, App};

pub struct OriginalPlugin;

impl Plugin for OriginalPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("original_plugin", semver::Version::new(1, 0, 0))
    }
    
    fn build_plugin(&self, app: &mut App) -> Result<()> {
        app.add_system(OriginalSystem::new());
        Ok(())
    }
}
```

#### Adapted for Mirai

```rust
// Adapted for Mirai integration
use mirai::core::plugin::{Plugin, PluginInfo, App};
use mirai::core::instance::Instance;

pub struct AdaptedPlugin;

impl Plugin for AdaptedPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("adapted_plugin", semver::Version::new(2, 0, 0))
            .with_description("Adapted for Mirai integration")
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        // Original functionality
        app.add_system(AdaptedSystem::new());
        
        // Mirai-specific enhancements
        app.add_system(MiraiIntegrationSystem::new());
        
        Ok(())
    }
    
    fn configure_mirai(&self, instance: &mut Instance) -> Result<()> {
        // Mirai-specific configuration
        instance.register_packet_handler(CustomPacketHandler::new());
        Ok(())
    }
}
```

## Configuration Migration

### Legacy Configuration

```toml
# Old mirai.toml
[extension.my_extension]
enabled = true
log_level = "info"
max_entities = 100

[extension.my_extension.features]
auto_cleanup = true
performance_monitoring = false
```

### Unified Configuration

```toml
# New unified_config.toml
[plugins.my_plugin]
enabled = true
update_interval = "1s"
max_entities = 100

[plugins.my_plugin.features]
auto_cleanup = true
performance_monitoring = true
mirai_integration = true

[plugins.my_plugin.mirai]
packet_handling = true
world_integration = true
```

### Configuration Code Migration

```rust
// Old configuration
#[derive(Deserialize)]
pub struct LegacyConfig {
    pub enabled: bool,
    pub log_level: String,
    pub max_entities: usize,
}

// New unified configuration
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct UnifiedPluginConfig {
    pub enabled: bool,
    pub update_interval: Duration,
    pub max_entities: usize,
    pub features: FeatureConfig,
    pub mirai: MiraiIntegrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub auto_cleanup: bool,
    pub performance_monitoring: bool,
    pub mirai_integration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiraiIntegrationConfig {
    pub packet_handling: bool,
    pub world_integration: bool,
}
```

## Data Structure Migration

### Entity Management

```rust
// Old entity handling
impl LegacyExtension {
    fn handle_entity(&mut self, entity_id: u64, data: &EntityData) {
        // Direct entity manipulation
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.update(data);
        }
    }
}

// New ECS approach
impl System for EntityHandlerSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Query entities with specific components
        let entities: Vec<EntityId> = world.query::<&MyComponent>()
            .iter()
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        for entity_id in entities {
            if let Some(mut component) = world.get_component_mut::<MyComponent>(entity_id) {
                component.update();
            }
        }
        
        Ok(())
    }
}
```

### Event System Migration

```rust
// Old callback-based events
impl MiraiExtension for LegacyExtension {
    fn on_player_join(&mut self, player: &Player) {
        self.handle_player_join(player);
    }
}

// New event-driven system
#[derive(Debug, Clone)]
pub struct PlayerJoinEvent {
    pub player: Player,
    pub timestamp: SystemTime,
}

impl Event for PlayerJoinEvent {}

impl System for PlayerEventHandler {
    fn run(&mut self, world: &mut World) -> Result<()> {
        for event in world.get_events::<PlayerJoinEvent>() {
            self.handle_player_join(&event.player);
        }
        Ok(())
    }
}
```

## Performance Migration

### Memory Management

```rust
// Old manual memory management
pub struct LegacyExtension {
    entities: HashMap<u64, EntityData>,
    cleanup_timer: Instant,
}

impl LegacyExtension {
    fn update(&mut self, _instance: &Instance) {
        if self.cleanup_timer.elapsed() > Duration::from_secs(60) {
            self.cleanup_expired_entities();
            self.cleanup_timer = Instant::now();
        }
    }
}

// New automated memory management
use mirai::core::memory::ObjectPool;

pub struct OptimizedSystem {
    entity_pool: ObjectPool<EntityData>,
    last_cleanup: Instant,
}

impl System for OptimizedSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Use object pools for better memory management
        let entity_data = self.entity_pool.get();
        
        // Process entity
        self.process_entity(entity_data);
        
        // Return to pool
        self.entity_pool.return_object(entity_data);
        
        // Automatic cleanup through ECS
        if self.last_cleanup.elapsed() > Duration::from_secs(60) {
            world.cleanup_expired_entities();
            self.last_cleanup = Instant::now();
        }
        
        Ok(())
    }
}
```

## Testing Migration

### Legacy Testing

```rust
#[cfg(test)]
mod legacy_tests {
    #[test]
    fn test_extension() {
        let mut extension = LegacyExtension::new();
        let player = create_test_player();
        
        extension.on_player_join(&player);
        
        assert!(extension.players.contains_key(&player.id()));
    }
}
```

### Modern Testing

```rust
#[cfg(test)]
mod modern_tests {
    use mirai::core::testing::{TestWorld, TestApp};
    
    #[test]
    fn test_plugin_system() {
        let mut world = TestWorld::new()
            .with_resource(PluginConfig::default())
            .build();
        
        let mut system = PlayerJoinHandler::new();
        
        // Send test event
        world.send_event(PlayerJoinEvent {
            player: create_test_player(),
            timestamp: SystemTime::now(),
        });
        
        // Run system
        system.run(&mut world).unwrap();
        
        // Verify behavior
        assert_eq!(world.get_events::<PlayerJoinEvent>().count(), 0);
    }
    
    #[tokio::test]
    async fn test_plugin_integration() {
        let mut app = TestApp::new();
        app.add_plugin(MigratedPlugin::new());
        
        // Run for several ticks
        for _ in 0..10 {
            app.update().await;
        }
        
        // Verify plugin state
        assert!(app.has_resource::<PluginConfig>());
    }
}
```

## Step-by-Step Migration Process

### 1. Analyze Existing Code

- Identify callback methods and convert to event handlers
- Map data structures to ECS components
- Identify global state for resources
- Review performance-critical sections

### 2. Create Plugin Structure

```rust
// Start with basic plugin structure
pub struct MigratedPlugin {
    // Keep existing configuration
    config: LegacyConfig,
}

impl Plugin for MigratedPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("migrated_plugin", semver::Version::new(2, 0, 0))
    }
    
    fn build(&self, app: &mut App) -> Result<()> {
        // Gradually add systems
        Ok(())
    }
}
```

### 3. Migrate Systems Incrementally

```rust
// Phase 1: Basic functionality
impl Plugin for MigratedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        app.add_system(BasicMigratedSystem::new());
        Ok(())
    }
}

// Phase 2: Add event handling
impl Plugin for MigratedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        app.add_system(BasicMigratedSystem::new())
           .add_system(EventHandlerSystem::new());
        Ok(())
    }
}

// Phase 3: Add performance optimizations
impl Plugin for MigratedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        app.add_system(BasicMigratedSystem::new())
           .add_system(EventHandlerSystem::new())
           .add_system(PerformanceOptimizedSystem::new());
        Ok(())
    }
}
```

### 4. Test and Validate

- Run unit tests for each migrated component
- Perform integration testing
- Validate performance improvements
- Test with existing Mirai deployments

## Common Migration Patterns

### Pattern 1: Callback to Event Handler

```rust
// Before: Direct callback
fn on_player_chat(&mut self, player: &Player, message: &str) {
    if message.starts_with("!") {
        self.handle_command(player, message);
    }
}

// After: Event-driven
impl System for ChatHandlerSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        for event in world.get_events::<PlayerChatEvent>() {
            if event.message.starts_with("!") {
                self.handle_command(world, &event.player, &event.message);
            }
        }
        Ok(())
    }
}
```

### Pattern 2: Global State to Resource

```rust
// Before: Global state in extension
pub struct LegacyExtension {
    player_data: HashMap<PlayerId, PlayerData>,
    server_stats: ServerStats,
}

// After: ECS resources
#[derive(Resource)]
pub struct PlayerDataResource {
    data: HashMap<PlayerId, PlayerData>,
}

#[derive(Resource)]
pub struct ServerStatsResource {
    stats: ServerStats,
}
```

### Pattern 3: Manual Updates to Systems

```rust
// Before: Manual update loop
impl MiraiExtension for LegacyExtension {
    fn update(&mut self, instance: &Instance) {
        self.update_players();
        self.update_world_state();
        self.cleanup_expired_data();
    }
}

// After: Separate systems
pub struct PlayerUpdateSystem;
pub struct WorldStateSystem;
pub struct CleanupSystem;

impl Plugin for MigratedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        app.add_system(PlayerUpdateSystem::new())
           .add_system(WorldStateSystem::new())
           .add_system(CleanupSystem::new());
        Ok(())
    }
}
```

## Troubleshooting Migration Issues

### Common Problems

1. **Missing Dependencies**: Ensure all required systems are added
2. **Resource Not Found**: Check resource registration in plugin build
3. **Event Not Firing**: Verify event senders and handlers
4. **Performance Regression**: Review system ordering and parallel safety

### Debugging Tips

```rust
// Add debug logging
impl System for DebuggingSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        tracing::debug!("System running with {} entities", 
            world.entity_count());
        
        // Your system logic
        
        Ok(())
    }
}

// Use performance monitoring
impl System for MonitoredSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let start = Instant::now();
        
        // System logic
        
        let duration = start.elapsed();
        if duration > Duration::from_millis(10) {
            tracing::warn!("System took {:?} to execute", duration);
        }
        
        Ok(())
    }
}
```

This migration guide provides a comprehensive path for updating existing code to work with the merged Mirai system. The key is to migrate incrementally, test thoroughly, and take advantage of the new performance and architectural improvements.