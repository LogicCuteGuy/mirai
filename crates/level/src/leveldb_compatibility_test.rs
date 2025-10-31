#[cfg(test)]
mod tests {
    use super::*;
    use crate::{provider::Provider, EnhancedChunk, ChunkPos, LevelDbCompatibilityManager, ChunkFormatVersion, ChunkState, MigrationConfig, EnhancedGameWorld, Position, BlockPos, WorldBorder, GameRules};
    use proto::types::Dimension;
    use util::Vector;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

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
        // Add minimal NBT data (simplified)
        let mut level_data = minimal_level_data;
        level_data.extend_from_slice(&[0; 96]); // Padding to match size
        std::fs::write(&level_dat_path, level_data).expect("Failed to write level.dat");
        
        let provider = Provider::open(temp_dir.path()).expect("Failed to open provider");
        (Arc::new(provider), temp_dir)
    }

    #[test]
    fn test_compatibility_manager_creation() {
        let (provider, _temp_dir) = create_test_provider();
        let compatibility_manager = LevelDbCompatibilityManager::new(provider);
        
        // Just test that creation succeeds - provider should be accessible
        let _provider_ref = compatibility_manager.provider();
        // If we get here without panicking, creation succeeded
    }

    // #[test]
    // fn test_custom_key_creation() {
    //     let coordinates = Vector::from([10, 20]);
    //     let dimension = Dimension::Overworld;
    //     
    //     let metadata_key = create_custom_key(coordinates, dimension, "enhanced_metadata");
    //     assert!(is_custom_enhanced_key(&metadata_key));
    //     assert_eq!(get_custom_key_type(&metadata_key), Some("enhanced_metadata"));
    //     
    //     let world_key = create_custom_key(coordinates, dimension, "world_metadata");
    //     assert!(is_custom_enhanced_key(&world_key));
    //     assert_eq!(get_custom_key_type(&world_key), Some("world_metadata"));
    // }

    #[test]
    fn test_enhanced_chunk_creation() {
        let pos = ChunkPos::new(5, 10);
        let dimension = Dimension::Overworld;
        
        let chunk = EnhancedChunk::new(pos, dimension);
        
        assert_eq!(chunk.pos, pos);
        assert_eq!(chunk.dimension, dimension);
        assert_eq!(chunk.state, ChunkState::Unloaded);
        assert!(!chunk.dirty);
        assert_eq!(chunk.biomes.len(), 256);
        assert_eq!(chunk.height_map.len(), 256);
    }

    #[test]
    fn test_chunk_biome_operations() {
        let pos = ChunkPos::new(0, 0);
        let mut chunk = EnhancedChunk::new(pos, Dimension::Overworld);
        
        // Test setting and getting biomes
        chunk.set_biome(5, 10, 42);
        assert_eq!(chunk.get_biome(5, 10), 42);
        assert!(chunk.dirty);
        
        // Test height operations
        chunk.set_height(3, 7, 128);
        assert_eq!(chunk.get_height(3, 7), 128);
    }

    #[test]
    fn test_chunk_subchunk_operations() {
        let pos = ChunkPos::new(0, 0);
        let mut chunk = EnhancedChunk::new(pos, Dimension::Overworld);
        
        // Test getting or creating subchunks
        let subchunk = chunk.get_or_create_subchunk(5);
        assert_eq!(subchunk.index(), 5);
        
        // Test getting existing subchunk
        let same_subchunk = chunk.get_subchunk(5);
        assert!(same_subchunk.is_some());
        assert_eq!(same_subchunk.unwrap().index(), 5);
        
        // Test getting non-existent subchunk
        let missing_subchunk = chunk.get_subchunk(10);
        assert!(missing_subchunk.is_none());
    }

    #[test]
    fn test_chunk_state_management() {
        let pos = ChunkPos::new(0, 0);
        let mut chunk = EnhancedChunk::new(pos, Dimension::Overworld);
        
        assert_eq!(chunk.state, ChunkState::Unloaded);
        assert!(!chunk.dirty);
        
        chunk.mark_dirty();
        assert!(chunk.dirty);
        
        chunk.mark_clean();
        assert!(!chunk.dirty);
    }

    #[test]
    fn test_migration_config_defaults() {
        let config = MigrationConfig::default();
        
        assert!(config.create_backup);
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.batch_delay_ms, 10);
        assert!(!config.fail_on_chunk_error);
        assert!(config.validate_after_migration);
        assert_eq!(config.validation_sample_size, 100);
        assert!(config.cleanup_after_migration);
    }

    #[test]
    fn test_chunk_pos_conversions() {
        let pos = ChunkPos::new(10, 20);
        let vector = pos.to_vector();
        let converted_back = ChunkPos::from(vector.clone());
        
        assert_eq!(pos, converted_back);
        assert_eq!(vector.x, 10);
        assert_eq!(vector.y, 20);
    }

    #[test]
    fn test_format_version_detection() {
        let (provider, _temp_dir) = create_test_provider();
        let format_version = ChunkFormatVersion::detect_from_provider(&provider);
        
        // Should detect as Mirai format since no enhanced metadata exists
        assert_eq!(format_version, ChunkFormatVersion::Mirai);
    }

    #[test]
    fn test_enhanced_world_creation() {
        let world = EnhancedGameWorld::new("TestWorld", Dimension::Overworld);
        
        assert_eq!(world.name, "TestWorld");
        assert_eq!(world.dimension, Dimension::Overworld);
        assert_eq!(world.spawn_point, Position::default());
        assert_eq!(world.game_rules.do_daylight_cycle, true);
        assert_eq!(world.generation_settings.generator_name, "overworld");
    }

    #[test]
    fn test_position_operations() {
        let pos1 = Position::new(0.0, 0.0, 0.0);
        let pos2 = Position::new(3.0, 4.0, 0.0);
        
        let distance = pos1.distance_to(&pos2);
        assert_eq!(distance, 5.0); // 3-4-5 triangle
        
        let block_pos = BlockPos::from(Position::new(5.7, 10.3, -2.8));
        assert_eq!(block_pos, BlockPos::new(5, 10, -3));
    }

    #[test]
    fn test_world_border_defaults() {
        let border = WorldBorder::default();
        
        assert_eq!(border.center_x, 0.0);
        assert_eq!(border.center_z, 0.0);
        assert_eq!(border.size, 60_000_000.0);
        assert_eq!(border.damage_per_block, 0.2);
    }

    #[test]
    fn test_game_rules_defaults() {
        let rules = GameRules::default();
        
        assert!(rules.do_daylight_cycle);
        assert!(rules.do_weather_cycle);
        assert!(!rules.keep_inventory);
        assert!(rules.mob_griefing);
        assert!(rules.do_mob_spawning);
    }
}