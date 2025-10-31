//! Vanilla mobs plugin for Mirai implementing basic mob AI, spawning, and behavior systems
//! 
//! This plugin provides reference implementations for vanilla Minecraft mobs
//! including AI systems, spawning logic, and behavior trees using the ECS architecture
//! integrated with Mirai's existing entity management.

use crate::core::plugin::{Plugin, PluginInfo};
use crate::core::ecs::{
    World, Entity, Component, Resource, System, EntityId, EntityManager,
    MiraiWorld, BedrockClientEcsExt
};
use crate::core::instance::Instance;
use crate::level::{Position, ChunkPosition};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, Duration, Instant};
use std::sync::{Arc, Weak};

/// Vanilla mobs plugin for Mirai
pub struct VanillaMobsPlugin {
    instance: Weak<Instance>,
}

impl VanillaMobsPlugin {
    /// Create a new vanilla mobs plugin
    pub fn new(instance: Weak<Instance>) -> Self {
        Self { instance }
    }
}

impl Plugin for VanillaMobsPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("vanilla_mobs", semver::Version::new(1, 0, 0))
            .with_description("Vanilla mob AI, spawning, and behavior systems for Mirai")
            .with_author("Mirai Team")
    }
    
    fn build(&self, app: &mut crate::core::plugin::App) -> Result<()> {
        // Add mob-related resources
        app.insert_resource(MobSpawnConfig::default())
           .insert_resource(MobRegistry::default())
           .insert_resource(MobAIConfig::default());
        
        // Add mob systems
        app.add_system(MobSpawningSystem::new(self.instance.clone()))
           .add_system(MobAISystem::new())
           .add_system(MobBehaviorSystem::new())
           .add_system(MobPathfindingSystem::new())
           .add_system(MobDespawnSystem::new());
        
        tracing::info!("Vanilla mobs plugin initialized for Mirai");
        Ok(())
    }
}

/// Mob type enumeration for different vanilla mobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobType {
    // Passive mobs
    Pig,
    Cow,
    Sheep,
    Chicken,
    Rabbit,
    Horse,
    
    // Neutral mobs
    Wolf,
    IronGolem,
    PolarBear,
    Bee,
    
    // Hostile mobs
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    Enderman,
    Witch,
    
    // Boss mobs
    EnderDragon,
    Wither,
}

impl MobType {
    /// Get the mob category
    pub fn category(&self) -> MobCategory {
        match self {
            Self::Pig | Self::Cow | Self::Sheep | Self::Chicken | Self::Rabbit | Self::Horse => {
                MobCategory::Passive
            }
            Self::Wolf | Self::IronGolem | Self::PolarBear | Self::Bee => {
                MobCategory::Neutral
            }
            Self::Zombie | Self::Skeleton | Self::Creeper | Self::Spider | Self::Enderman | Self::Witch => {
                MobCategory::Hostile
            }
            Self::EnderDragon | Self::Wither => {
                MobCategory::Boss
            }
        }
    }
    
    /// Get the mob's maximum health
    pub fn max_health(&self) -> f32 {
        match self {
            Self::Pig => 10.0,
            Self::Cow => 10.0,
            Self::Sheep => 8.0,
            Self::Chicken => 4.0,
            Self::Rabbit => 3.0,
            Self::Horse => 30.0,
            Self::Wolf => 20.0,
            Self::IronGolem => 100.0,
            Self::PolarBear => 30.0,
            Self::Bee => 10.0,
            Self::Zombie => 20.0,
            Self::Skeleton => 20.0,
            Self::Creeper => 20.0,
            Self::Spider => 16.0,
            Self::Enderman => 40.0,
            Self::Witch => 26.0,
            Self::EnderDragon => 200.0,
            Self::Wither => 300.0,
        }
    }
    
    /// Get the mob's movement speed
    pub fn movement_speed(&self) -> f64 {
        match self {
            Self::Pig => 0.25,
            Self::Cow => 0.25,
            Self::Sheep => 0.23,
            Self::Chicken => 0.25,
            Self::Rabbit => 0.3,
            Self::Horse => 0.45,
            Self::Wolf => 0.3,
            Self::IronGolem => 0.25,
            Self::PolarBear => 0.3,
            Self::Bee => 0.6,
            Self::Zombie => 0.23,
            Self::Skeleton => 0.25,
            Self::Creeper => 0.25,
            Self::Spider => 0.3,
            Self::Enderman => 0.3,
            Self::Witch => 0.25,
            Self::EnderDragon => 0.5,
            Self::Wither => 0.6,
        }
    }
    
    /// Get the mob's detection range for players
    pub fn detection_range(&self) -> f64 {
        match self {
            Self::Pig | Self::Cow | Self::Sheep | Self::Chicken | Self::Rabbit | Self::Horse => 0.0, // Passive mobs don't actively detect
            Self::Wolf => 16.0,
            Self::IronGolem => 16.0,
            Self::PolarBear => 20.0,
            Self::Bee => 10.0,
            Self::Zombie => 35.0,
            Self::Skeleton => 16.0,
            Self::Creeper => 16.0,
            Self::Spider => 16.0,
            Self::Enderman => 64.0,
            Self::Witch => 16.0,
            Self::EnderDragon => 100.0,
            Self::Wither => 50.0,
        }
    }
}

/// Mob behavior categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobCategory {
    Passive,
    Neutral,
    Hostile,
    Boss,
}

/// Mob AI state for behavior trees
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobAIState {
    Idle,
    Wandering,
    Following,
    Fleeing,
    Attacking,
    Patrolling,
    Sleeping,
    Eating,
    Breeding,
}

/// Mob component that stores mob-specific data
#[derive(Debug, Clone)]
pub struct Mob {
    pub mob_type: MobType,
    pub ai_state: MobAIState,
    pub target: Option<EntityId>,
    pub last_ai_update: Instant,
    pub spawn_time: SystemTime,
    pub home_position: Option<Position>,
    pub wander_target: Option<Position>,
    pub path: Vec<Position>,
    pub path_index: usize,
    pub custom_data: HashMap<String, String>,
    pub health: f32,
    pub max_health: f32,
}

impl Component for Mob {}

impl Mob {
    /// Create a new mob
    pub fn new(mob_type: MobType) -> Self {
        let max_health = mob_type.max_health();
        Self {
            mob_type,
            ai_state: MobAIState::Idle,
            target: None,
            last_ai_update: Instant::now(),
            spawn_time: SystemTime::now(),
            home_position: None,
            wander_target: None,
            path: Vec::new(),
            path_index: 0,
            custom_data: HashMap::new(),
            health: max_health,
            max_health,
        }
    }
    
    /// Set the mob's home position
    pub fn set_home(&mut self, position: Position) {
        self.home_position = Some(position);
    }
    
    /// Set the mob's target
    pub fn set_target(&mut self, target: Option<EntityId>) {
        self.target = target;
        if target.is_some() {
            self.ai_state = match self.mob_type.category() {
                MobCategory::Passive => MobAIState::Fleeing,
                MobCategory::Neutral | MobCategory::Hostile | MobCategory::Boss => MobAIState::Attacking,
            };
        } else {
            self.ai_state = MobAIState::Idle;
        }
    }
    
    /// Check if the mob should despawn naturally
    pub fn should_despawn(&self, distance_to_nearest_player: f64) -> bool {
        // Don't despawn if too close to players
        if distance_to_nearest_player < 32.0 {
            return false;
        }
        
        // Despawn if far from players and old enough
        let age = self.spawn_time.elapsed().unwrap_or(Duration::ZERO);
        distance_to_nearest_player > 128.0 && age > Duration::from_secs(30)
    }
    
    /// Take damage
    pub fn take_damage(&mut self, damage: f32) {
        self.health = (self.health - damage).max(0.0);
    }
    
    /// Check if the mob is dead
    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }
    
    /// Heal the mob
    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
    }
}

/// Velocity component for mob movement
#[derive(Debug, Clone, Default)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Component for Velocity {}

impl Velocity {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    
    pub fn normalize(&mut self) {
        let mag = self.magnitude();
        if mag > 0.0 {
            self.x /= mag;
            self.y /= mag;
            self.z /= mag;
        }
    }
}

/// Mob spawning configuration
#[derive(Debug, Clone)]
pub struct MobSpawnConfig {
    pub enabled: bool,
    pub max_mobs_per_chunk: usize,
    pub spawn_rate: f32,
    pub spawn_ranges: HashMap<MobType, (f64, f64)>, // (min_distance, max_distance) from players
    pub biome_spawns: HashMap<String, Vec<MobType>>,
    pub light_level_requirements: HashMap<MobType, (u8, u8)>, // (min, max) light levels
}

impl Resource for MobSpawnConfig {}

impl Default for MobSpawnConfig {
    fn default() -> Self {
        let mut spawn_ranges = HashMap::new();
        spawn_ranges.insert(MobType::Pig, (24.0, 128.0));
        spawn_ranges.insert(MobType::Cow, (24.0, 128.0));
        spawn_ranges.insert(MobType::Sheep, (24.0, 128.0));
        spawn_ranges.insert(MobType::Chicken, (24.0, 128.0));
        spawn_ranges.insert(MobType::Zombie, (24.0, 128.0));
        spawn_ranges.insert(MobType::Skeleton, (24.0, 128.0));
        spawn_ranges.insert(MobType::Creeper, (24.0, 128.0));
        spawn_ranges.insert(MobType::Spider, (24.0, 128.0));
        
        let mut biome_spawns = HashMap::new();
        biome_spawns.insert("plains".to_string(), vec![MobType::Pig, MobType::Cow, MobType::Sheep, MobType::Chicken]);
        biome_spawns.insert("forest".to_string(), vec![MobType::Pig, MobType::Cow, MobType::Wolf]);
        biome_spawns.insert("desert".to_string(), vec![MobType::Rabbit]);
        
        let mut light_requirements = HashMap::new();
        // Passive mobs spawn in light
        light_requirements.insert(MobType::Pig, (9, 15));
        light_requirements.insert(MobType::Cow, (9, 15));
        light_requirements.insert(MobType::Sheep, (9, 15));
        light_requirements.insert(MobType::Chicken, (9, 15));
        // Hostile mobs spawn in darkness
        light_requirements.insert(MobType::Zombie, (0, 7));
        light_requirements.insert(MobType::Skeleton, (0, 7));
        light_requirements.insert(MobType::Creeper, (0, 7));
        light_requirements.insert(MobType::Spider, (0, 7));
        
        Self {
            enabled: true,
            max_mobs_per_chunk: 10,
            spawn_rate: 0.1,
            spawn_ranges,
            biome_spawns,
            light_level_requirements: light_requirements,
        }
    }
}

/// Registry of all mob types and their configurations
#[derive(Debug, Clone)]
pub struct MobRegistry {
    pub registered_mobs: HashSet<MobType>,
    pub mob_configs: HashMap<MobType, MobConfig>,
}

impl Resource for MobRegistry {}

impl Default for MobRegistry {
    fn default() -> Self {
        let mut registry = Self {
            registered_mobs: HashSet::new(),
            mob_configs: HashMap::new(),
        };
        
        // Register all vanilla mobs
        for mob_type in [
            MobType::Pig, MobType::Cow, MobType::Sheep, MobType::Chicken,
            MobType::Zombie, MobType::Skeleton, MobType::Creeper, MobType::Spider,
            MobType::Wolf, MobType::Enderman,
        ] {
            registry.register_mob(mob_type, MobConfig::default_for_type(mob_type));
        }
        
        registry
    }
}

impl MobRegistry {
    /// Register a new mob type
    pub fn register_mob(&mut self, mob_type: MobType, config: MobConfig) {
        self.registered_mobs.insert(mob_type);
        self.mob_configs.insert(mob_type, config);
    }
    
    /// Check if a mob type is registered
    pub fn is_registered(&self, mob_type: MobType) -> bool {
        self.registered_mobs.contains(&mob_type)
    }
    
    /// Get mob configuration
    pub fn get_config(&self, mob_type: MobType) -> Option<&MobConfig> {
        self.mob_configs.get(&mob_type)
    }
}

/// Configuration for individual mob types
#[derive(Debug, Clone)]
pub struct MobConfig {
    pub enabled: bool,
    pub max_health: f32,
    pub movement_speed: f64,
    pub detection_range: f64,
    pub attack_damage: f32,
    pub attack_range: f64,
    pub ai_update_interval: Duration,
    pub pathfinding_enabled: bool,
    pub can_despawn: bool,
}

impl MobConfig {
    /// Create default configuration for a mob type
    pub fn default_for_type(mob_type: MobType) -> Self {
        Self {
            enabled: true,
            max_health: mob_type.max_health(),
            movement_speed: mob_type.movement_speed(),
            detection_range: mob_type.detection_range(),
            attack_damage: match mob_type.category() {
                MobCategory::Passive => 0.0,
                MobCategory::Neutral => 4.0,
                MobCategory::Hostile => 6.0,
                MobCategory::Boss => 15.0,
            },
            attack_range: 2.0,
            ai_update_interval: Duration::from_millis(50),
            pathfinding_enabled: true,
            can_despawn: !matches!(mob_type.category(), MobCategory::Boss),
        }
    }
}

/// AI configuration for mob behavior
#[derive(Debug, Clone)]
pub struct MobAIConfig {
    pub ai_enabled: bool,
    pub pathfinding_enabled: bool,
    pub max_pathfinding_distance: f64,
    pub wander_range: f64,
    pub update_interval: Duration,
}

impl Resource for MobAIConfig {}

impl Default for MobAIConfig {
    fn default() -> Self {
        Self {
            ai_enabled: true,
            pathfinding_enabled: true,
            max_pathfinding_distance: 32.0,
            wander_range: 16.0,
            update_interval: Duration::from_millis(50),
        }
    }
}/// Syst
em for spawning mobs based on configuration
pub struct MobSpawningSystem {
    last_spawn_check: Instant,
    instance: Weak<Instance>,
}

impl MobSpawningSystem {
    pub fn new(instance: Weak<Instance>) -> Self {
        Self {
            last_spawn_check: Instant::now(),
            instance,
        }
    }
    
    /// Spawn a mob at the given position
    fn spawn_mob(&self, world: &mut World, mob_type: MobType, position: Position) -> Result<EntityId> {
        // Create a new entity
        let entity_id = world.spawn_entity();
        
        // Add mob component
        let mut mob = Mob::new(mob_type);
        mob.set_home(position);
        world.insert_component(entity_id, mob)?;
        
        // Add position component
        world.insert_component(entity_id, position)?;
        
        // Add velocity component
        world.insert_component(entity_id, Velocity::default())?;
        
        tracing::debug!("Spawned {:?} at {:?}", mob_type, position);
        Ok(entity_id)
    }
    
    /// Check if a position is suitable for spawning the given mob type
    fn can_spawn_at(&self, _world: &World, _mob_type: MobType, _position: Position) -> bool {
        // In a real implementation, this would check:
        // - Light levels
        // - Block types (solid ground, water, etc.)
        // - Biome compatibility
        // - Distance from players
        // - Existing mob density
        true // Simplified for this example
    }
    
    /// Get nearby players for spawn distance calculations
    fn get_nearby_players(&self, _position: Position) -> Vec<EntityId> {
        // In a real implementation, this would query the Mirai instance
        // for connected players and their positions
        Vec::new() // Simplified for this example
    }
}

impl System for MobSpawningSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_spawn_check) < Duration::from_secs(5) {
            return Ok(());
        }
        self.last_spawn_check = now;
        
        let spawn_config = world.get_resource::<MobSpawnConfig>().cloned();
        let mob_registry = world.get_resource::<MobRegistry>().cloned();
        
        if let (Some(config), Some(registry)) = (spawn_config, mob_registry) {
            if !config.enabled {
                return Ok(());
            }
            
            // Simple spawning logic - in a real implementation this would be much more sophisticated
            for mob_type in registry.registered_mobs {
                if let Some(mob_config) = registry.get_config(mob_type) {
                    if mob_config.enabled && fastrand::f32() < config.spawn_rate {
                        // Try to spawn near a random position
                        let x = fastrand::f64() * 100.0 - 50.0;
                        let z = fastrand::f64() * 100.0 - 50.0;
                        let position = Position::new(x, 64.0, z, 0.0, 0.0);
                        
                        if self.can_spawn_at(world, mob_type, position) {
                            if let Err(e) = self.spawn_mob(world, mob_type, position) {
                                tracing::warn!("Failed to spawn {:?}: {}", mob_type, e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "mob_spawning"
    }
}

/// System for mob AI behavior
pub struct MobAISystem {
    last_update: Instant,
}

impl MobAISystem {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
        }
    }
    
    /// Update AI for a single mob
    fn update_mob_ai(&self, world: &mut World, entity_id: EntityId, mob: &mut Mob) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(mob.last_ai_update) < Duration::from_millis(100) {
            return Ok(());
        }
        mob.last_ai_update = now;
        
        // Get mob position
        let position = world.get_component::<Position>(entity_id).copied();
        if position.is_none() {
            return Ok(());
        }
        let position = position.unwrap();
        
        // Simple AI state machine
        match mob.ai_state {
            MobAIState::Idle => {
                // Randomly start wandering
                if fastrand::f32() < 0.1 {
                    mob.ai_state = MobAIState::Wandering;
                    self.set_wander_target(mob, position);
                }
            }
            MobAIState::Wandering => {
                // Move towards wander target
                if let Some(target) = mob.wander_target {
                    let distance = ((target.x - position.x).powi(2) + (target.z - position.z).powi(2)).sqrt();
                    if distance < 2.0 {
                        // Reached target, go idle
                        mob.ai_state = MobAIState::Idle;
                        mob.wander_target = None;
                    } else {
                        // Move towards target
                        self.move_towards(world, entity_id, position, target, mob.mob_type.movement_speed());
                    }
                } else {
                    mob.ai_state = MobAIState::Idle;
                }
            }
            MobAIState::Attacking => {
                // Attack behavior for hostile mobs
                if let Some(target_id) = mob.target {
                    if let Some(target_pos) = world.get_component::<Position>(target_id) {
                        let distance = ((target_pos.x - position.x).powi(2) + (target_pos.z - position.z).powi(2)).sqrt();
                        if distance > mob.mob_type.detection_range() {
                            // Lost target
                            mob.set_target(None);
                        } else if distance > 2.0 {
                            // Move towards target
                            self.move_towards(world, entity_id, position, *target_pos, mob.mob_type.movement_speed());
                        } else {
                            // Attack target (simplified)
                            tracing::debug!("{:?} attacking target", mob.mob_type);
                        }
                    } else {
                        // Target no longer exists
                        mob.set_target(None);
                    }
                } else {
                    mob.ai_state = MobAIState::Idle;
                }
            }
            MobAIState::Fleeing => {
                // Flee behavior for passive mobs
                if let Some(target_id) = mob.target {
                    if let Some(target_pos) = world.get_component::<Position>(target_id) {
                        let distance = ((target_pos.x - position.x).powi(2) + (target_pos.z - position.z).powi(2)).sqrt();
                        if distance > 16.0 {
                            // Safe distance reached
                            mob.set_target(None);
                        } else {
                            // Flee from target
                            let flee_x = position.x + (position.x - target_pos.x).signum() * 2.0;
                            let flee_z = position.z + (position.z - target_pos.z).signum() * 2.0;
                            let flee_pos = Position::new(flee_x, position.y, flee_z, 0.0, 0.0);
                            self.move_towards(world, entity_id, position, flee_pos, mob.mob_type.movement_speed() * 1.5);
                        }
                    } else {
                        mob.set_target(None);
                    }
                } else {
                    mob.ai_state = MobAIState::Idle;
                }
            }
            _ => {
                // Other states not implemented yet
                mob.ai_state = MobAIState::Idle;
            }
        }
        
        Ok(())
    }
    
    /// Set a random wander target around the mob's current position
    fn set_wander_target(&self, mob: &mut Mob, current_pos: Position) {
        let range = 8.0;
        let target_x = current_pos.x + (fastrand::f64() - 0.5) * range * 2.0;
        let target_z = current_pos.z + (fastrand::f64() - 0.5) * range * 2.0;
        mob.wander_target = Some(Position::new(target_x, current_pos.y, target_z, 0.0, 0.0));
    }
    
    /// Move an entity towards a target position
    fn move_towards(&self, world: &mut World, entity_id: EntityId, current: Position, target: Position, speed: f64) {
        if let Some(velocity) = world.get_component_mut::<Velocity>(entity_id) {
            let dx = target.x - current.x;
            let dz = target.z - current.z;
            let distance = (dx * dx + dz * dz).sqrt();
            
            if distance > 0.0 {
                velocity.x = (dx / distance) * speed;
                velocity.z = (dz / distance) * speed;
            }
        }
    }
}

impl System for MobAISystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let ai_config = world.get_resource::<MobAIConfig>().cloned();
        if let Some(config) = ai_config {
            if !config.ai_enabled {
                return Ok(());
            }
        }
        
        // Get all entities with Mob components
        let mob_entities: Vec<EntityId> = world.query::<&Mob>()
            .iter()
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        for entity_id in mob_entities {
            if let Some(mut mob) = world.get_component_mut::<Mob>(entity_id).cloned() {
                if let Err(e) = self.update_mob_ai(world, entity_id, &mut mob) {
                    tracing::warn!("Failed to update AI for mob {}: {}", entity_id, e);
                }
                // Update the mob component
                if let Some(mob_component) = world.get_component_mut::<Mob>(entity_id) {
                    *mob_component = mob;
                }
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "mob_ai"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["mob_spawning".to_string()]
    }
}

/// System for mob behavior trees (placeholder for more complex behavior)
pub struct MobBehaviorSystem;

impl MobBehaviorSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for MobBehaviorSystem {
    fn run(&mut self, _world: &mut World) -> Result<()> {
        // Placeholder for behavior tree execution
        // In a real implementation, this would execute behavior trees for each mob
        Ok(())
    }
    
    fn name(&self) -> &str {
        "mob_behavior"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["mob_ai".to_string()]
    }
}

/// System for mob pathfinding
pub struct MobPathfindingSystem;

impl MobPathfindingSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for MobPathfindingSystem {
    fn run(&mut self, _world: &mut World) -> Result<()> {
        // Placeholder for pathfinding implementation
        // In a real implementation, this would calculate paths for mobs that need them
        Ok(())
    }
    
    fn name(&self) -> &str {
        "mob_pathfinding"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["mob_ai".to_string()]
    }
}

/// System for despawning mobs that should be removed
pub struct MobDespawnSystem {
    last_check: Instant,
}

impl MobDespawnSystem {
    pub fn new() -> Self {
        Self {
            last_check: Instant::now(),
        }
    }
}

impl System for MobDespawnSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_check) < Duration::from_secs(10) {
            return Ok(());
        }
        self.last_check = now;
        
        // Get all entities with Mob components
        let mob_entities: Vec<EntityId> = world.query::<&Mob>()
            .iter()
            .map(|(entity_id, _)| entity_id)
            .collect();
        
        let mut to_despawn = Vec::new();
        
        for entity_id in mob_entities {
            if let Some(mob) = world.get_component::<Mob>(entity_id) {
                // Simple despawn logic - check distance to nearest player
                let distance_to_player = 200.0; // Placeholder - would calculate actual distance
                
                if mob.should_despawn(distance_to_player) || mob.is_dead() {
                    to_despawn.push(entity_id);
                }
            }
        }
        
        // Despawn mobs that should be removed
        for entity_id in to_despawn {
            world.despawn_entity(entity_id);
            tracing::debug!("Despawned mob entity {}", entity_id);
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "mob_despawn"
    }
    
    fn dependencies(&self) -> Vec<String> {
        vec!["mob_behavior".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mob_type_properties() {
        assert_eq!(MobType::Zombie.category(), MobCategory::Hostile);
        assert_eq!(MobType::Pig.category(), MobCategory::Passive);
        assert_eq!(MobType::Wolf.category(), MobCategory::Neutral);
        assert_eq!(MobType::EnderDragon.category(), MobCategory::Boss);
        
        assert!(MobType::Zombie.max_health() > 0.0);
        assert!(MobType::Zombie.movement_speed() > 0.0);
        assert!(MobType::Zombie.detection_range() > 0.0);
    }
    
    #[test]
    fn test_mob_creation() {
        let mob = Mob::new(MobType::Zombie);
        assert_eq!(mob.mob_type, MobType::Zombie);
        assert_eq!(mob.ai_state, MobAIState::Idle);
        assert!(mob.target.is_none());
        assert!(mob.home_position.is_none());
        assert_eq!(mob.health, mob.max_health);
    }
    
    #[test]
    fn test_mob_target_setting() {
        let mut mob = Mob::new(MobType::Zombie);
        let target_id = EntityId::new(1);
        
        mob.set_target(Some(target_id));
        assert_eq!(mob.target, Some(target_id));
        assert_eq!(mob.ai_state, MobAIState::Attacking);
        
        mob.set_target(None);
        assert!(mob.target.is_none());
        assert_eq!(mob.ai_state, MobAIState::Idle);
    }
    
    #[test]
    fn test_mob_health_system() {
        let mut mob = Mob::new(MobType::Zombie);
        let initial_health = mob.health;
        
        mob.take_damage(5.0);
        assert_eq!(mob.health, initial_health - 5.0);
        assert!(!mob.is_dead());
        
        mob.take_damage(100.0);
        assert_eq!(mob.health, 0.0);
        assert!(mob.is_dead());
        
        let mut healthy_mob = Mob::new(MobType::Pig);
        healthy_mob.take_damage(2.0);
        healthy_mob.heal(1.0);
        assert_eq!(healthy_mob.health, healthy_mob.max_health - 1.0);
    }
    
    #[test]
    fn test_velocity_component() {
        let mut velocity = Velocity::new(3.0, 4.0, 0.0);
        assert_eq!(velocity.magnitude(), 5.0);
        
        velocity.normalize();
        assert!((velocity.magnitude() - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_mob_registry() {
        let mut registry = MobRegistry::default();
        
        assert!(registry.is_registered(MobType::Zombie));
        assert!(registry.get_config(MobType::Zombie).is_some());
        
        let custom_config = MobConfig::default_for_type(MobType::Creeper);
        registry.register_mob(MobType::Creeper, custom_config);
        
        assert!(registry.is_registered(MobType::Creeper));
    }
    
    #[test]
    fn test_mob_config() {
        let config = MobConfig::default_for_type(MobType::Zombie);
        assert!(config.enabled);
        assert!(config.max_health > 0.0);
        assert!(config.movement_speed > 0.0);
        assert!(config.attack_damage > 0.0);
        assert!(config.can_despawn);
        
        let boss_config = MobConfig::default_for_type(MobType::EnderDragon);
        assert!(!boss_config.can_despawn);
        assert!(boss_config.attack_damage > config.attack_damage);
    }
    
    #[test]
    fn test_spawn_config() {
        let config = MobSpawnConfig::default();
        assert!(config.enabled);
        assert!(config.max_mobs_per_chunk > 0);
        assert!(config.spawn_rate > 0.0);
        assert!(!config.spawn_ranges.is_empty());
        assert!(!config.biome_spawns.is_empty());
    }
    
    #[test]
    fn test_mob_despawn_logic() {
        let mob = Mob::new(MobType::Zombie);
        
        // Should not despawn when close to players
        assert!(!mob.should_despawn(16.0));
        
        // Should despawn when far from players (but need to wait for age)
        assert!(!mob.should_despawn(200.0)); // Too young
        
        // Test with older mob would require manipulating spawn_time
    }
}