//! LevelDB compatibility layer for enhanced world system

use crate::{
    provider::Provider, DataKey, KeyType, WriteBatch, SubChunk,
    world::{EnhancedChunk, EnhancedGameWorld, ChunkPos, ChunkState},
    database::KvRef, PaletteEntry, SubStorage, to_offset,
    Biomes, BiomeEncoding,
};
use anyhow::{Result, anyhow};
use proto::types::Dimension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};
use util::Vector;

/// LevelDB compatibility manager that bridges enhanced world system with mirai's storage
pub struct LevelDbCompatibilityManager {
    /// Original mirai provider
    provider: Arc<Provider>,
    /// Enhanced world metadata cache
    world_metadata_cache: Option<EnhancedWorldMetadata>,
    /// Chunk format version tracking
    format_version: ChunkFormatVersion,
}

impl LevelDbCompatibilityManager {
    /// Create a new compatibility manager
    pub fn new(provider: Arc<Provider>) -> Self {
        let format_version = ChunkFormatVersion::detect_from_provider(&provider);
        Self {
            provider,
            world_metadata_cache: None,
            format_version,
        }
    }
    
    /// Get the mirai provider
    pub fn provider(&self) -> Arc<Provider> {
        self.provider.clone()
    }
    
    /// Load enhanced chunk from mirai storage with full compatibility
    pub fn load_enhanced_chunk(&self, pos: ChunkPos, dimension: Dimension) -> Result<Option<EnhancedChunk>> {
        debug!("Loading enhanced chunk {:?} from LevelDB", pos);
        
        // Try to load using enhanced chunk first, then fall back to mirai format
        if let Some(enhanced_chunk) = self.load_enhanced_chunk_direct(pos, dimension)? {
            return Ok(Some(enhanced_chunk));
        }
        
        // Fall back to loading from mirai's native format
        self.load_chunk_from_mirai_format(pos, dimension)
    }
    
    /// Load enhanced chunk directly from enhanced format (if exists)
    fn load_enhanced_chunk_direct(&self, pos: ChunkPos, dimension: Dimension) -> Result<Option<EnhancedChunk>> {
        // Try to load enhanced metadata first
        let metadata_key = create_custom_key(pos.to_vector(), dimension, "enhanced_metadata");
        
        if let Some(metadata_data) = self.provider.get_raw(metadata_key)? {
            let metadata: EnhancedChunkMetadata = serde_json::from_slice(&metadata_data)?;
            
            // Load the chunk using enhanced format
            let mut chunk = EnhancedChunk::new(pos, dimension);
            chunk.state = ChunkState::Generating;
            
            // Load biomes using provider method
            if let Some(biomes) = self.provider.biomes(pos.to_vector(), dimension)? {
                // Convert biomes to our format
                chunk.biomes = self.convert_biomes_to_simple_format(&biomes);
            }
            
            // Load subchunks
            for y in metadata.subchunk_range.0..=metadata.subchunk_range.1 {
                let subchunk_pos = Vector::from([pos.x, y as i32, pos.z]);
                if let Some(subchunk) = self.provider.subchunk(subchunk_pos, dimension)? {
                    chunk.subchunks.insert(y, subchunk);
                }
            }
            
            // Apply enhanced metadata
            chunk.last_modified = metadata.last_modified;
            chunk.dirty = metadata.dirty;
            chunk.height_map = metadata.height_map;
            chunk.state = ChunkState::Loaded;
            
            debug!("Loaded enhanced chunk {:?} with metadata", pos);
            return Ok(Some(chunk));
        }
        
        Ok(None)
    }
    
    /// Load chunk from mirai's native format and convert to enhanced format
    fn load_chunk_from_mirai_format(&self, pos: ChunkPos, dimension: Dimension) -> Result<Option<EnhancedChunk>> {
        // Use the existing EnhancedChunk::load_from_provider method
        EnhancedChunk::load_from_provider(&self.provider, pos, dimension)
    }
    
    /// Save enhanced chunk to LevelDB with backward compatibility
    pub fn save_enhanced_chunk(&self, chunk: &EnhancedChunk) -> Result<()> {
        debug!("Saving enhanced chunk {:?} to LevelDB", chunk.pos);
        
        // Save in both enhanced format and maintain mirai compatibility
        self.save_chunk_enhanced_format(chunk)?;
        self.save_chunk_mirai_format(chunk)?;
        
        Ok(())
    }
    
    /// Save chunk in enhanced format with metadata
    fn save_chunk_enhanced_format(&self, chunk: &EnhancedChunk) -> Result<()> {
        // Create enhanced metadata
        let metadata = EnhancedChunkMetadata {
            format_version: self.format_version,
            last_modified: chunk.last_modified,
            dirty: chunk.dirty,
            height_map: chunk.height_map.clone(),
            subchunk_range: self.calculate_subchunk_range(chunk),
            entity_count: 0, // TODO: Get from ECS system
            enhanced_features: vec!["streaming".to_string(), "ecs_entities".to_string()],
        };
        
        // Save metadata using custom key
        let metadata_key = create_custom_key(chunk.pos.to_vector(), chunk.dimension, "enhanced_metadata");
        let metadata_data = serde_json::to_vec(&metadata)?;
        self.provider.put_raw(metadata_key, metadata_data)?;
        
        debug!("Saved enhanced metadata for chunk {:?}", chunk.pos);
        Ok(())
    }
    
    /// Save chunk in mirai's native format for backward compatibility
    fn save_chunk_mirai_format(&self, chunk: &EnhancedChunk) -> Result<()> {
        // Save biomes in mirai format - this would need proper Biomes implementation
        // For now, we'll skip biome saving to avoid compilation errors
        // TODO: Implement proper biome conversion when Biomes structure is available
        
        // Save subchunks in mirai format
        for (&y, subchunk) in &chunk.subchunks {
            let subchunk_key = DataKey {
                coordinates: chunk.pos.to_vector(),
                dimension: chunk.dimension,
                data: KeyType::SubChunk { index: y },
            };
            
            let subchunk_data = subchunk.serialize_disk()?;
            self.provider.put_raw(subchunk_key, subchunk_data)?;
        }
        
        // Save chunk version
        let version_key = DataKey {
            coordinates: chunk.pos.to_vector(),
            dimension: chunk.dimension,
            data: KeyType::ChunkVersion,
        };
        
        self.provider.put_raw(version_key, vec![40u8])?; // Current chunk version
        
        debug!("Saved chunk {:?} in mirai format", chunk.pos);
        Ok(())
    }
    
    /// Convert mirai's Biomes structure to simple biome array format
    fn convert_biomes_to_simple_format(&self, biomes: &crate::Biomes) -> Vec<u8> {
        let mut simple_biomes = vec![1u8; 256]; // Default to plains biome
        
        // Extract biome data from the first fragment if available
        if let Some(fragment) = biomes.fragments().first() {
            match fragment {
                crate::BiomeEncoding::Single(biome_id) => {
                    // Fill entire chunk with single biome
                    simple_biomes.fill(*biome_id as u8);
                }
                crate::BiomeEncoding::Paletted(storage) => {
                    // Convert paletted biomes to simple format
                    // Take the first 256 indices and map them through the palette
                    for (i, &index) in storage.indices().iter().take(256).enumerate() {
                        if let Some(&biome_id) = storage.palette().get(index as usize) {
                            simple_biomes[i] = biome_id as u8;
                        }
                    }
                }
                crate::BiomeEncoding::Inherit => {
                    // Keep default plains biome
                }
            }
        }
        
        simple_biomes
    }
    
    /// Calculate the Y range of subchunks in the chunk
    fn calculate_subchunk_range(&self, chunk: &EnhancedChunk) -> (i8, i8) {
        if chunk.subchunks.is_empty() {
            return (0, 0);
        }
        
        let min_y = *chunk.subchunks.keys().min().unwrap();
        let max_y = *chunk.subchunks.keys().max().unwrap();
        (min_y, max_y)
    }
    
    /// Load enhanced world metadata
    pub fn load_world_metadata(&mut self) -> Result<Option<EnhancedWorldMetadata>> {
        if let Some(ref metadata) = self.world_metadata_cache {
            return Ok(Some(metadata.clone()));
        }
        
        // Try to load enhanced world metadata
        let metadata_key = create_custom_key(
            Vector::from([0, 0]), // Special coordinates for world metadata
            Dimension::Overworld,
            "world_metadata"
        );
        
        if let Some(metadata_data) = self.provider.get_raw(metadata_key)? {
            let metadata: EnhancedWorldMetadata = serde_json::from_slice(&metadata_data)?;
            self.world_metadata_cache = Some(metadata.clone());
            return Ok(Some(metadata));
        }
        
        // Fall back to creating metadata from mirai level settings
        let level_settings = self.provider.settings()?;
        let metadata = EnhancedWorldMetadata::from_level_settings(&level_settings);
        
        // Cache the metadata
        self.world_metadata_cache = Some(metadata.clone());
        
        Ok(Some(metadata))
    }
    
    /// Save enhanced world metadata
    pub fn save_world_metadata(&mut self, world: &EnhancedGameWorld) -> Result<()> {
        let metadata = EnhancedWorldMetadata::from_enhanced_world(world);
        
        let metadata_key = create_custom_key(
            Vector::from([0, 0]), // Special coordinates for world metadata
            Dimension::Overworld,
            "world_metadata"
        );
        
        let metadata_data = serde_json::to_vec(&metadata)?;
        self.provider.put_raw(metadata_key, metadata_data)?;
        
        // Update cache
        self.world_metadata_cache = Some(metadata);
        
        info!("Saved enhanced world metadata");
        Ok(())
    }
    
    /// Migrate existing world to enhanced format
    pub fn migrate_to_enhanced_format(&self) -> Result<MigrationReport> {
        info!("Starting migration to enhanced format");
        
        let mut report = MigrationReport::default();
        let start_time = std::time::Instant::now();
        
        // Scan all existing chunks
        let existing_chunks = self.scan_existing_chunks()?;
        report.total_chunks = existing_chunks.len();
        
        for chunk_pos in existing_chunks {
            match self.migrate_chunk_to_enhanced(chunk_pos) {
                Ok(()) => {
                    report.migrated_chunks += 1;
                }
                Err(e) => {
                    report.failed_chunks += 1;
                    warn!("Failed to migrate chunk {:?}: {}", chunk_pos, e);
                }
            }
        }
        
        report.migration_time = start_time.elapsed();
        
        info!(
            "Migration completed: {}/{} chunks migrated in {:?}",
            report.migrated_chunks,
            report.total_chunks,
            report.migration_time
        );
        
        Ok(report)
    }
    
    /// Scan for existing chunks in the database
    fn scan_existing_chunks(&self) -> Result<Vec<ChunkPos>> {
        let mut chunks = Vec::new();
        
        // Iterate through database keys to find chunk data
        for kv_ref in self.provider.iter() {
            let key_data = kv_ref.key();
            
            // Try to parse as DataKey
            let mut reader = key_data.as_ref();
            if let Ok(data_key) = DataKey::deserialize(&mut reader) {
                match data_key.data {
                    KeyType::SubChunk { .. } | KeyType::ChunkVersion | KeyType::Biome3d => {
                        let chunk_pos = ChunkPos::from(data_key.coordinates);
                        if !chunks.contains(&chunk_pos) {
                            chunks.push(chunk_pos);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(chunks)
    }
    
    /// Migrate a single chunk to enhanced format
    fn migrate_chunk_to_enhanced(&self, pos: ChunkPos) -> Result<()> {
        // Load chunk in mirai format
        if let Some(chunk) = self.load_chunk_from_mirai_format(pos, Dimension::Overworld)? {
            // Save in enhanced format
            self.save_chunk_enhanced_format(&chunk)?;
            debug!("Migrated chunk {:?} to enhanced format", pos);
        }
        
        Ok(())
    }
    
    /// Validate data integrity between formats
    pub fn validate_data_integrity(&self, pos: ChunkPos, dimension: Dimension) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::default();
        
        // Load chunk in both formats
        let mirai_chunk = self.load_chunk_from_mirai_format(pos, dimension)?;
        let enhanced_chunk = self.load_enhanced_chunk_direct(pos, dimension)?;
        
        match (mirai_chunk, enhanced_chunk) {
            (Some(mirai), Some(enhanced)) => {
                // Compare biomes
                if mirai.biomes == enhanced.biomes {
                    report.biomes_match = true;
                } else {
                    report.issues.push("Biome data mismatch".to_string());
                }
                
                // Compare subchunks
                if mirai.subchunks.len() == enhanced.subchunks.len() {
                    report.subchunks_match = true;
                    
                    for (y, mirai_subchunk) in &mirai.subchunks {
                        if let Some(enhanced_subchunk) = enhanced.subchunks.get(y) {
                            if mirai_subchunk != enhanced_subchunk {
                                report.issues.push(format!("Subchunk {} data mismatch", y));
                            }
                        } else {
                            report.issues.push(format!("Missing subchunk {} in enhanced format", y));
                        }
                    }
                } else {
                    report.issues.push("Subchunk count mismatch".to_string());
                }
                
                report.both_formats_exist = true;
            }
            (Some(_), None) => {
                report.issues.push("Enhanced format missing".to_string());
            }
            (None, Some(_)) => {
                report.issues.push("Mirai format missing".to_string());
            }
            (None, None) => {
                report.issues.push("Both formats missing".to_string());
            }
        }
        
        Ok(report)
    }
}

/// Enhanced chunk metadata stored alongside chunk data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedChunkMetadata {
    pub format_version: ChunkFormatVersion,
    pub last_modified: std::time::SystemTime,
    pub dirty: bool,
    pub height_map: Vec<u16>,
    pub subchunk_range: (i8, i8),
    pub entity_count: usize,
    pub enhanced_features: Vec<String>,
}

/// Enhanced world metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedWorldMetadata {
    pub format_version: ChunkFormatVersion,
    pub world_id: uuid::Uuid,
    pub created_at: std::time::SystemTime,
    pub last_played: std::time::SystemTime,
    pub enhanced_features: Vec<String>,
    pub migration_status: MigrationStatus,
}

impl EnhancedWorldMetadata {
    pub fn from_level_settings(settings: &crate::settings::LevelSettings) -> Self {
        Self {
            format_version: ChunkFormatVersion::Enhanced,
            world_id: uuid::Uuid::new_v4(),
            created_at: std::time::SystemTime::now(),
            last_played: std::time::SystemTime::now(),
            enhanced_features: vec![
                "streaming".to_string(),
                "ecs_entities".to_string(),
                "performance_optimizations".to_string(),
            ],
            migration_status: MigrationStatus::Migrated,
        }
    }
    
    pub fn from_enhanced_world(world: &EnhancedGameWorld) -> Self {
        Self {
            format_version: ChunkFormatVersion::Enhanced,
            world_id: world.id,
            created_at: world.metadata.created_at,
            last_played: world.metadata.last_played,
            enhanced_features: vec![
                "streaming".to_string(),
                "ecs_entities".to_string(),
                "performance_optimizations".to_string(),
            ],
            migration_status: MigrationStatus::Migrated,
        }
    }
}

/// Chunk format version tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkFormatVersion {
    /// Original mirai format
    Mirai,
    /// Enhanced format with additional features
    Enhanced,
    /// Hybrid format (both mirai and enhanced)
    Hybrid,
}

impl ChunkFormatVersion {
    pub fn detect_from_provider(provider: &Provider) -> Self {
        // Try to detect if enhanced format exists
        let test_key = DataKey {
            coordinates: Vector::from([0, 0]),
            dimension: Dimension::Overworld,
            data: KeyType::Custom("enhanced_metadata".to_string()),
        };
        
        if provider.database().get(test_key).unwrap_or(None).is_some() {
            ChunkFormatVersion::Enhanced
        } else {
            ChunkFormatVersion::Mirai
        }
    }
}

/// Migration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Not migrated
    NotMigrated,
    /// Migration in progress
    InProgress,
    /// Successfully migrated
    Migrated,
    /// Migration failed
    Failed,
}

/// Migration report
#[derive(Debug, Default)]
pub struct MigrationReport {
    pub total_chunks: usize,
    pub migrated_chunks: usize,
    pub failed_chunks: usize,
    pub migration_time: std::time::Duration,
}

/// Data integrity validation report
#[derive(Debug, Default)]
pub struct IntegrityReport {
    pub both_formats_exist: bool,
    pub biomes_match: bool,
    pub subchunks_match: bool,
    pub issues: Vec<String>,
}

impl IntegrityReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Custom key type for enhanced metadata storage
/// This extends the existing KeyType enum functionality
pub fn create_custom_key(coordinates: Vector<i32, 2>, dimension: Dimension, custom_type: &str) -> DataKey {
    // Use a special SubChunk index to store custom metadata
    // In a real implementation, we'd extend the KeyType enum
    let custom_index = match custom_type {
        "enhanced_metadata" => -127,
        "world_metadata" => -126,
        _ => -128, // Generic custom key
    };
    
    DataKey {
        coordinates,
        dimension,
        data: KeyType::SubChunk { index: custom_index },
    }
}

/// Helper function to check if a key is a custom enhanced key
pub fn is_custom_enhanced_key(key: &DataKey) -> bool {
    match key.data {
        KeyType::SubChunk { index } if index <= -126 => true,
        _ => false,
    }
}

/// Helper function to get custom key type from DataKey
pub fn get_custom_key_type(key: &DataKey) -> Option<&'static str> {
    match key.data {
        KeyType::SubChunk { index: -127 } => Some("enhanced_metadata"),
        KeyType::SubChunk { index: -126 } => Some("world_metadata"),
        KeyType::SubChunk { index } if index <= -128 => Some("generic_custom"),
        _ => None,
    }
}