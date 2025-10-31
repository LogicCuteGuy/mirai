#[cfg(test)]
mod basic_tests {
    use super::*;
    use crate::{ChunkPos, ChunkState, ChunkFormatVersion, create_custom_key, is_custom_enhanced_key, get_custom_key_type, EnhancedChunkMetadata};
    use proto::types::Dimension;
    use util::Vector;
    use std::time::SystemTime;

    #[test]
    fn test_chunk_format_version() {
        // Test format version enum
        assert_eq!(ChunkFormatVersion::Mirai as u8, 0);
        assert_ne!(ChunkFormatVersion::Enhanced, ChunkFormatVersion::Mirai);
    }

    #[test]
    fn test_chunk_pos_operations() {
        let pos = ChunkPos::new(10, 20);
        assert_eq!(pos.x, 10);
        assert_eq!(pos.z, 20);
        
        let vector = pos.to_vector();
        let converted_back = ChunkPos::from(vector);
        assert_eq!(pos, converted_back);
    }

    #[test]
    fn test_enhanced_chunk_metadata() {
        let metadata = EnhancedChunkMetadata {
            format_version: ChunkFormatVersion::Enhanced,
            last_modified: SystemTime::now(),
            dirty: false,
            height_map: vec![64; 256],
            subchunk_range: (-4, 15),
            entity_count: 0,
            enhanced_features: vec!["streaming".to_string()],
        };
        
        assert_eq!(metadata.format_version, ChunkFormatVersion::Enhanced);
        assert!(!metadata.dirty);
        assert_eq!(metadata.height_map.len(), 256);
        assert_eq!(metadata.subchunk_range, (-4, 15));
    }

    #[test]
    fn test_migration_status() {
        use crate::MigrationStatus;
        
        assert_eq!(MigrationStatus::NotMigrated as u8, 0);
        assert_ne!(MigrationStatus::Migrated, MigrationStatus::NotMigrated);
    }

    #[test]
    fn test_custom_key_helpers() {
        use crate::{create_custom_key, is_custom_enhanced_key, get_custom_key_type};
        use util::Vector;
        use proto::types::Dimension;
        
        let coordinates = Vector::from([10, 20]);
        let dimension = Dimension::Overworld;
        
        let metadata_key = create_custom_key(coordinates.clone(), dimension, "enhanced_metadata");
        assert!(is_custom_enhanced_key(&metadata_key));
        assert_eq!(get_custom_key_type(&metadata_key), Some("enhanced_metadata"));
        
        let world_key = create_custom_key(coordinates, dimension, "world_metadata");
        assert!(is_custom_enhanced_key(&world_key));
        assert_eq!(get_custom_key_type(&world_key), Some("world_metadata"));
    }
}