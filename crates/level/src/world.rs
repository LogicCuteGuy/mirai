//! Enhanced world management system integrating minecraft-server-core features with mirai's level system

use crate::{provider::Provider, SubChunk, DataKey, KeyType, WriteBatch, PaletteEntry, SubStorage, to_offset, settings::LevelSettings};
use anyhow::{Result, anyhow};
use proto::types::Dimension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use parking_lot::RwLock;
use dashmap::DashMap;
use util::Vector;

/// Enhanced game world that combines mirai's level system with minecraft-server-core features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedGameWorld {
    /// World identifier
    pub id: uuid::Uuid,
    /// World name
    pub name: String,
    /// World dimension
    pub dimension: Dimension,
    /// Spawn point coordinates
    pub spawn_point: Position,
    /// World border configuration
    pub world_border: WorldBorder,
    /// Game rules for this world
    pub game_rules: GameRules,
    /// World generation settings
    pub generation_settings: WorldGenerationSettings,
    /// World metadata
    pub metadata: WorldMetadata,
}

impl EnhancedGameWorld {
    /// Create a new enhanced game world
    pub fn new(name: impl Into<String>, dimension: Dimension) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            dimension,
            spawn_point: Position::default(),
            world_border: WorldBorder::default(),
            game_rules: GameRules::default(),
            generation_settings: WorldGenerationSettings::default(),
            metadata: WorldMetadata::default(),
        }
    }
    
    /// Load world from mirai provider
    pub fn load_from_provider(provider: &Provider) -> Result<Self> {
        // Try to load world metadata from provider
        let settings = provider.settings()?;
        
        Ok(Self {
            id: uuid::Uuid::new_v4(), // Generate new ID if not stored
            name: settings.level_name.clone(),
            dimension: Dimension::Overworld, // Default to overworld
            spawn_point: Position::new(
                settings.spawn_x as f64,
                settings.spawn_y as f64,
                settings.spawn_z as f64,
            ),
            world_border: WorldBorder::default(),
            game_rules: GameRules::from_level_settings(&settings),
            generation_settings: WorldGenerationSettings::from_level_settings(&settings),
            metadata: WorldMetadata::from_level_settings(&settings),
        })
    }
}

/// 3D position in the world
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    
    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(0.0, 64.0, 0.0)
    }
}

/// Block position (integer coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl From<Position> for BlockPos {
    fn from(pos: Position) -> Self {
        Self::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32)
    }
}

/// Chunk coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
    
    pub fn from_block_pos(block_pos: BlockPos) -> Self {
        Self::new(block_pos.x >> 4, block_pos.z >> 4)
    }
    
    /// Convert to mirai's Vector format for compatibility
    pub fn to_vector(self) -> Vector<i32, 2> {
        Vector::from([self.x, self.z])
    }
}

impl From<Vector<i32, 2>> for ChunkPos {
    fn from(vec: Vector<i32, 2>) -> Self {
        Self::new(vec.x, vec.y)
    }
}

/// World border configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBorder {
    pub center_x: f64,
    pub center_z: f64,
    pub size: f64,
    pub damage_per_block: f64,
    pub damage_buffer: f64,
    pub warning_distance: i32,
    pub warning_time: i32,
}

impl Default for WorldBorder {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_z: 0.0,
            size: 60_000_000.0,
            damage_per_block: 0.2,
            damage_buffer: 5.0,
            warning_distance: 5,
            warning_time: 15,
        }
    }
}

/// Game rules for the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRules {
    pub do_daylight_cycle: bool,
    pub do_weather_cycle: bool,
    pub keep_inventory: bool,
    pub mob_griefing: bool,
    pub do_mob_spawning: bool,
    pub do_fire_tick: bool,
    pub command_block_output: bool,
    pub natural_regeneration: bool,
    pub show_death_messages: bool,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            do_daylight_cycle: true,
            do_weather_cycle: true,
            keep_inventory: false,
            mob_griefing: true,
            do_mob_spawning: true,
            do_fire_tick: true,
            command_block_output: true,
            natural_regeneration: true,
            show_death_messages: true,
        }
    }
}

impl GameRules {
    /// Create game rules from mirai level settings
    pub fn from_level_settings(settings: &LevelSettings) -> Self {
        Self {
            do_daylight_cycle: !settings.daylight_lock, // Inverted logic
            do_weather_cycle: settings.weather_cycle,
            keep_inventory: settings.keep_inventory,
            mob_griefing: settings.mob_griefing,
            do_mob_spawning: settings.mob_spawning,
            do_fire_tick: settings.fire_tick,
            command_block_output: settings.command_block_output,
            natural_regeneration: settings.natural_regeneration,
            show_death_messages: settings.show_death_messages,
        }
    }
}

/// World generation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldGenerationSettings {
    pub generator_name: String,
    pub world_seed: i64,
    pub generate_structures: bool,
    pub sea_level: i32,
    pub world_height: i32,
    pub min_world_height: i32,
}

impl Default for WorldGenerationSettings {
    fn default() -> Self {
        Self {
            generator_name: "overworld".to_string(),
            world_seed: 0,
            generate_structures: true,
            sea_level: 64,
            world_height: 384,
            min_world_height: -64,
        }
    }
}

impl WorldGenerationSettings {
    /// Create generation settings from mirai level settings
    pub fn from_level_settings(settings: &LevelSettings) -> Self {
        Self {
            generator_name: match settings.generator {
                0 => "overworld".to_string(),
                1 => "flat".to_string(),
                _ => "overworld".to_string(),
            },
            world_seed: settings.random_seed,
            generate_structures: true, // Default to true
            sea_level: 64, // Default sea level
            world_height: 384, // Modern world height
            min_world_height: -64, // Modern minimum height
        }
    }
}

/// World metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    pub created_at: SystemTime,
    pub last_played: SystemTime,
    pub version: String,
    pub game_type: GameType,
    pub difficulty: Difficulty,
}

impl Default for WorldMetadata {
    fn default() -> Self {
        Self {
            created_at: SystemTime::now(),
            last_played: SystemTime::now(),
            version: "1.21.0".to_string(),
            game_type: GameType::Survival,
            difficulty: Difficulty::Normal,
        }
    }
}

impl WorldMetadata {
    /// Create metadata from mirai level settings
    pub fn from_level_settings(settings: &LevelSettings) -> Self {
        Self {
            created_at: SystemTime::now(),
            last_played: SystemTime::now(),
            version: "1.21.0".to_string(), // Default version
            game_type: GameType::from_level_settings(settings),
            difficulty: Difficulty::from_level_settings(settings),
        }
    }
}

/// Game type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameType {
    pub fn from_level_settings(settings: &LevelSettings) -> Self {
        match settings.game_mode {
            0 => Self::Survival,
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival, // Default to survival
        }
    }
}

/// Difficulty enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn from_level_settings(settings: &LevelSettings) -> Self {
        match settings.difficulty {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            3 => Self::Hard,
            _ => Self::Normal, // Default to normal
        }
    }
}

/// Chunk state for tracking loading/generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkState {
    /// Chunk is not loaded
    Unloaded,
    /// Chunk is being generated
    Generating,
    /// Chunk is loaded and ready
    Loaded,
    /// Chunk is being saved
    Saving,
    /// Chunk has unsaved changes
    Dirty,
}

/// Enhanced chunk that integrates with mirai's SubChunk system
#[derive(Debug, Clone)]
pub struct EnhancedChunk {
    /// Chunk coordinates
    pub pos: ChunkPos,
    /// Mirai subchunks (indexed by Y section)
    pub subchunks: HashMap<i8, SubChunk>,
    /// Biome data for each column (256 values for 16x16)
    pub biomes: Vec<u8>,
    /// Height map for each column (256 values for 16x16)
    pub height_map: Vec<u16>,
    /// Current state of the chunk
    pub state: ChunkState,
    /// Last modification time
    pub last_modified: SystemTime,
    /// Whether the chunk has been modified since last save
    pub dirty: bool,
    /// Dimension this chunk belongs to
    pub dimension: Dimension,
}

impl EnhancedChunk {
    /// Create a new empty enhanced chunk
    pub fn new(pos: ChunkPos, dimension: Dimension) -> Self {
        Self {
            pos,
            subchunks: HashMap::new(),
            biomes: vec![1; 256], // Default to plains biome
            height_map: vec![64; 256], // Default sea level
            state: ChunkState::Unloaded,
            last_modified: SystemTime::now(),
            dirty: false,
            dimension,
        }
    }
    
    /// Load chunk from mirai provider
    pub fn load_from_provider(provider: &Provider, pos: ChunkPos, dimension: Dimension) -> Result<Option<Self>> {
        let mut chunk = Self::new(pos, dimension);
        
        // Load biomes
        if let Some(biomes) = provider.biomes(pos.to_vector(), dimension)? {
            // Convert mirai biomes to our format
            chunk.biomes = biomes.to_biome_ids();
        }
        
        // Load subchunks for different Y levels
        let mut has_data = false;
        for y in -4..20 { // Cover extended world height
            let subchunk_pos = Vector::from([pos.x, y, pos.z]);
            if let Some(subchunk) = provider.subchunk(subchunk_pos, dimension)? {
                chunk.subchunks.insert(y as i8, subchunk);
                has_data = true;
            }
        }
        
        if has_data {
            chunk.state = ChunkState::Loaded;
            chunk.update_height_map();
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }
    
    /// Get or create a subchunk at the given Y level
    pub fn get_or_create_subchunk(&mut self, y: i8) -> &mut SubChunk {
        self.subchunks.entry(y).or_insert_with(|| SubChunk::empty(y))
    }
    
    /// Get a subchunk at the given Y level
    pub fn get_subchunk(&self, y: i8) -> Option<&SubChunk> {
        self.subchunks.get(&y)
    }
    
    /// Get biome at column coordinates
    pub fn get_biome(&self, x: u8, z: u8) -> u8 {
        let index = (z as usize) * 16 + (x as usize);
        self.biomes[index]
    }
    
    /// Set biome at column coordinates
    pub fn set_biome(&mut self, x: u8, z: u8, biome: u8) {
        let index = (z as usize) * 16 + (x as usize);
        self.biomes[index] = biome;
        self.mark_dirty();
    }
    
    /// Get height at column coordinates
    pub fn get_height(&self, x: u8, z: u8) -> u16 {
        let index = (z as usize) * 16 + (x as usize);
        self.height_map[index]
    }
    
    /// Set height at column coordinates
    pub fn set_height(&mut self, x: u8, z: u8, height: u16) {
        let index = (z as usize) * 16 + (x as usize);
        self.height_map[index] = height;
        self.mark_dirty();
    }
    
    /// Mark the chunk as dirty (needs saving)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_modified = SystemTime::now();
        if self.state == ChunkState::Loaded {
            self.state = ChunkState::Dirty;
        }
    }
    
    /// Mark the chunk as clean (saved)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        if self.state == ChunkState::Dirty {
            self.state = ChunkState::Loaded;
        }
    }
    
    /// Check if the chunk is empty (no subchunks with blocks)
    pub fn is_empty(&self) -> bool {
        self.subchunks.values().all(|subchunk| subchunk.is_empty())
    }
    
    /// Update height map based on subchunk data
    pub fn update_height_map(&mut self) {
        for x in 0..16 {
            for z in 0..16 {
                let mut height = 0u16;
                
                // Find the highest non-air block
                for y in (0..384).rev() {
                    let section_y = (y / 16) as i8;
                    if let Some(subchunk) = self.subchunks.get(&section_y) {
                        if !subchunk.is_empty() {
                            // Check if there's a block at this position
                            // This is a simplified check - in reality we'd need to examine the palette
                            height = y as u16;
                            break;
                        }
                    }
                }
                
                self.set_height(x, z, height);
            }
        }
    }
}

/// Enhanced world manager that combines mirai's provider system with minecraft-server-core features
pub struct EnhancedWorldManager {
    /// Mirai provider for LevelDB access
    provider: Arc<Provider>,
    /// Loaded chunks
    chunks: DashMap<ChunkPos, Arc<RwLock<EnhancedChunk>>>,
    /// World configuration
    world: Arc<RwLock<EnhancedGameWorld>>,
    /// Chunk loading queue
    loading_queue: Arc<RwLock<Vec<ChunkPos>>>,
    /// Maximum number of loaded chunks
    max_loaded_chunks: usize,
    /// Chunk view distance
    view_distance: u32,
    /// World generation pipeline
    generation_pipeline: Arc<RwLock<WorldGenerationPipeline>>,
}

impl EnhancedWorldManager {
    /// Create a new enhanced world manager
    pub fn new(
        provider: Provider,
        world: EnhancedGameWorld,
        max_loaded_chunks: usize,
        view_distance: u32,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
            chunks: DashMap::new(),
            world: Arc::new(RwLock::new(world)),
            loading_queue: Arc::new(RwLock::new(Vec::new())),
            max_loaded_chunks,
            view_distance,
            generation_pipeline: Arc::new(RwLock::new(WorldGenerationPipeline::new())),
        }
    }
    
    /// Create enhanced world manager from existing mirai world
    pub fn from_mirai_world<P: AsRef<Path>>(world_path: P) -> Result<Self> {
        let provider = Provider::open(&world_path)?;
        let world = EnhancedGameWorld::load_from_provider(&provider)?;
        
        Ok(Self::new(provider, world, 1000, 10))
    }
    
    /// Get the world configuration
    pub fn world(&self) -> Arc<RwLock<EnhancedGameWorld>> {
        self.world.clone()
    }
    
    /// Get the mirai provider
    pub fn provider(&self) -> Arc<Provider> {
        self.provider.clone()
    }
    
    /// Get a chunk if it's loaded
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<EnhancedChunk>>> {
        self.chunks.get(&pos).map(|entry| entry.value().clone())
    }
    
    /// Load a chunk (or return existing if already loaded)
    pub async fn load_chunk(&self, pos: ChunkPos) -> Result<Arc<RwLock<EnhancedChunk>>> {
        // Check if already loaded
        if let Some(chunk) = self.get_chunk(pos) {
            return Ok(chunk);
        }
        
        // Check if we need to unload chunks first
        if self.chunks.len() >= self.max_loaded_chunks {
            self.unload_distant_chunks(pos)?;
        }
        
        let dimension = self.world.read().dimension;
        
        // Try to load from mirai provider first
        let chunk = if let Some(chunk) = EnhancedChunk::load_from_provider(&self.provider, pos, dimension)? {
            tracing::debug!("Loaded chunk {:?} from mirai storage", pos);
            chunk
        } else {
            // Generate new chunk if not found in storage
            let chunk = self.generate_chunk(pos, dimension).await?;
            tracing::debug!("Generated new chunk {:?}", pos);
            chunk
        };
        
        let chunk_arc = Arc::new(RwLock::new(chunk));
        self.chunks.insert(pos, chunk_arc.clone());
        
        Ok(chunk_arc)
    }
    
    /// Generate a new chunk using the world generation pipeline
    async fn generate_chunk(&self, pos: ChunkPos, dimension: Dimension) -> Result<EnhancedChunk> {
        let world = self.world.read();
        let generator_name = world.generation_settings.generator_name.clone();
        let world_seed = world.generation_settings.world_seed;
        drop(world);
        
        let pipeline = self.generation_pipeline.read();
        pipeline.generate_enhanced_chunk(pos, dimension, Some(&generator_name), world_seed)
    }
    
    /// Unload a chunk
    pub fn unload_chunk(&self, pos: ChunkPos) -> Result<()> {
        if let Some((_, chunk_arc)) = self.chunks.remove(&pos) {
            // Check if dirty before acquiring lock
            let needs_save = {
                let chunk = chunk_arc.read();
                chunk.dirty
            };
            
            // Save chunk if dirty
            if needs_save {
                let chunk = chunk_arc.read();
                self.save_chunk_to_provider(&chunk)?;
                tracing::debug!("Saved dirty chunk at {:?}", pos);
            }
            
            tracing::debug!("Unloaded chunk at {:?}", pos);
        }
        Ok(())
    }
    
    /// Save chunk to mirai provider
    fn save_chunk_to_provider(&self, chunk: &EnhancedChunk) -> Result<()> {
        // This would require implementing save functionality to mirai's provider
        // For now, we'll just log that we would save it
        tracing::debug!("Would save chunk {:?} to mirai provider", chunk.pos);
        Ok(())
    }
    
    /// Unload chunks that are far from the given center position
    fn unload_distant_chunks(&self, center: ChunkPos) -> Result<()> {
        let max_distance = (self.view_distance + 2) as i32;
        let mut to_unload = Vec::new();
        
        for entry in self.chunks.iter() {
            let pos = *entry.key();
            let distance = (pos.x - center.x).abs().max((pos.z - center.z).abs());
            
            if distance > max_distance {
                to_unload.push(pos);
            }
        }
        
        for pos in to_unload {
            self.unload_chunk(pos)?;
        }
        
        Ok(())
    }
    
    /// Get all loaded chunk positions
    pub fn get_loaded_chunks(&self) -> Vec<ChunkPos> {
        self.chunks.iter().map(|entry| *entry.key()).collect()
    }
    
    /// Get chunks within view distance of a position
    pub fn get_chunks_in_range(&self, center: ChunkPos, distance: u32) -> Vec<ChunkPos> {
        let mut chunks = Vec::new();
        let distance = distance as i32;
        
        for x in (center.x - distance)..=(center.x + distance) {
            for z in (center.z - distance)..=(center.z + distance) {
                chunks.push(ChunkPos::new(x, z));
            }
        }
        
        chunks
    }
    
    /// Save all dirty chunks
    pub async fn save_all_chunks(&self) -> Result<()> {
        let mut saved_count = 0;
        
        for entry in self.chunks.iter() {
            let chunk_arc = entry.value();
            let mut chunk = chunk_arc.write();
            
            if chunk.dirty {
                self.save_chunk_to_provider(&chunk)?;
                chunk.mark_clean();
                saved_count += 1;
            }
        }
        
        if saved_count > 0 {
            tracing::info!("Saved {} dirty chunks to mirai storage", saved_count);
        }
        
        Ok(())
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> ChunkMemoryStats {
        let loaded_chunks = self.chunks.len();
        let estimated_memory = loaded_chunks * std::mem::size_of::<EnhancedChunk>();
        
        ChunkMemoryStats {
            loaded_chunks,
            estimated_memory_bytes: estimated_memory,
        }
    }
}

/// Memory usage statistics for chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMemoryStats {
    pub loaded_chunks: usize,
    pub estimated_memory_bytes: usize,
}

/// World generation pipeline that works with mirai's system
pub struct WorldGenerationPipeline {
    generators: HashMap<String, Box<dyn WorldGenerator>>,
    default_generator: String,
}

impl WorldGenerationPipeline {
    pub fn new() -> Self {
        let mut pipeline = Self {
            generators: HashMap::new(),
            default_generator: "flat".to_string(),
        };
        
        // Register default generators
        pipeline.register_generator("flat", Box::new(FlatWorldGenerator::new()));
        
        pipeline
    }
    
    /// Register a new world generator
    pub fn register_generator(&mut self, name: impl Into<String>, generator: Box<dyn WorldGenerator>) {
        let name = name.into();
        self.generators.insert(name, generator);
    }
    
    /// Set the default generator
    pub fn set_default_generator(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !self.generators.contains_key(&name) {
            return Err(anyhow!("Generator '{}' not found", name));
        }
        self.default_generator = name;
        Ok(())
    }
    
    /// Generate an enhanced chunk using the specified generator
    pub fn generate_enhanced_chunk(
        &self,
        pos: ChunkPos,
        dimension: Dimension,
        generator_name: Option<&str>,
        world_seed: i64,
    ) -> Result<EnhancedChunk> {
        let generator_name = generator_name.unwrap_or(&self.default_generator);
        
        let generator = self.generators.get(generator_name)
            .ok_or_else(|| anyhow!("Generator '{}' not found", generator_name))?;
        
        generator.generate_enhanced_chunk(pos, dimension, world_seed)
    }
}

impl Default for WorldGenerationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// World generator trait for enhanced chunks
pub trait WorldGenerator: Send + Sync {
    /// Generate an enhanced chunk at the given position
    fn generate_enhanced_chunk(&self, pos: ChunkPos, dimension: Dimension, world_seed: i64) -> Result<EnhancedChunk>;
    
    /// Get the name of this generator
    fn name(&self) -> &'static str;
    
    /// Get the default spawn point for this generator
    fn get_spawn_point(&self, _world_seed: i64) -> Position {
        Position::new(0.0, 64.0, 0.0)
    }
    
    /// Get the sea level for this generator
    fn get_sea_level(&self) -> i32 {
        64
    }
}

/// Flat world generator for testing
pub struct FlatWorldGenerator {
    block_layers: Vec<(String, u8)>, // (block_name, layer_height)
}

impl FlatWorldGenerator {
    pub fn new() -> Self {
        Self {
            block_layers: vec![
                ("minecraft:bedrock".to_string(), 1),
                ("minecraft:stone".to_string(), 3),
                ("minecraft:dirt".to_string(), 1),
                ("minecraft:grass_block".to_string(), 1),
            ],
        }
    }
    
    pub fn with_layers(layers: Vec<(String, u8)>) -> Self {
        Self {
            block_layers: layers,
        }
    }
}

impl Default for FlatWorldGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldGenerator for FlatWorldGenerator {
    fn generate_enhanced_chunk(&self, pos: ChunkPos, dimension: Dimension, _world_seed: i64) -> Result<EnhancedChunk> {
        let mut chunk = EnhancedChunk::new(pos, dimension);
        chunk.state = ChunkState::Generating;
        
        // Generate flat layers using mirai's SubChunk system
        let mut current_y = 0i32;
        for (block_name, height) in &self.block_layers {
            for layer in 0..*height {
                let y = current_y + layer as i32;
                if y >= 384 { break; } // Height limit
                
                let section_y = (y / 16) as i8;
                let subchunk = chunk.get_or_create_subchunk(section_y);
                
                // For now, we'll create a simple flat world by marking the subchunk as non-empty
                // In a real implementation, we would properly set up the palette and indices
                // This is a placeholder to demonstrate the structure
                
                // Create a simple air block for the palette
                let air_block = crate::PaletteEntry {
                    name: "minecraft:air".to_string(),
                    version: None,
                    states: HashMap::new(),
                };
                
                let block_entry = crate::PaletteEntry {
                    name: block_name.clone(),
                    version: None,
                    states: HashMap::new(),
                };
                
                // Ensure subchunk has at least one layer
                if subchunk.layers.is_empty() {
                    subchunk.layers.push(crate::SubStorage::empty());
                }
                
                // Add blocks to palette (simplified approach)
                let layer = &mut subchunk.layers[0];
                if layer.palette.is_empty() {
                    layer.palette.push(air_block);
                    layer.palette.push(block_entry);
                }
                
                // Set blocks in this Y level (simplified - would need proper coordinate mapping)
                // This is a placeholder implementation
                for x in 0..16 {
                    for z in 0..16 {
                        let local_y = (y % 16) as u8;
                        if local_y < 16 {
                            let offset = crate::to_offset(Vector::from([x, local_y, z]));
                            if offset < layer.indices.len() {
                                layer.indices[offset] = 1; // Index 1 in palette (the block)
                            }
                        }
                    }
                }
            }
            current_y += *height as i32;
        }
        
        // Set height map
        let surface_height = self.block_layers.iter().map(|(_, h)| *h as u16).sum::<u16>();
        for x in 0..16 {
            for z in 0..16 {
                chunk.set_height(x, z, surface_height);
            }
        }
        
        chunk.state = ChunkState::Loaded;
        Ok(chunk)
    }
    
    fn name(&self) -> &'static str {
        "flat"
    }
    
    fn get_spawn_point(&self, _world_seed: i64) -> Position {
        let surface_height = self.block_layers.iter().map(|(_, h)| *h as f64).sum::<f64>();
        Position::new(0.0, surface_height + 1.0, 0.0)
    }
}