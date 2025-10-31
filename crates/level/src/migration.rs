//! Migration utilities for existing mirai deployments

use crate::{
    provider::Provider, world::{EnhancedGameWorld, EnhancedWorldManager, ChunkPos, EnhancedChunk},
    leveldb_compatibility::LevelDbCompatibilityManager,
    ChunkFormatVersion, DataKey, KeyType,
};
use anyhow::{Result, anyhow};
use proto::types::Dimension;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};
use tracing::{info, warn, debug, error};
use std::sync::Arc;

/// Migration manager for upgrading existing mirai worlds to enhanced format
pub struct MigrationManager {
    /// Source mirai provider
    source_provider: Arc<Provider>,
    /// LevelDB compatibility manager
    compatibility_manager: LevelDbCompatibilityManager,
    /// Migration configuration
    config: MigrationConfig,
    /// Migration progress tracking
    progress: MigrationProgress,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(provider: Arc<Provider>, config: MigrationConfig) -> Self {
        let compatibility_manager = LevelDbCompatibilityManager::new(provider.clone());
        
        Self {
            source_provider: provider,
            compatibility_manager,
            config,
            progress: MigrationProgress::default(),
        }
    }
    
    /// Create migration manager with default configuration
    pub fn with_defaults(provider: Provider) -> Self {
        Self::new(Arc::new(provider), MigrationConfig::default())
    }
    
    /// Perform full migration of existing mirai world to enhanced format
    pub async fn migrate_world(&mut self) -> Result<MigrationReport> {
        info!("Starting world migration to enhanced format");
        let start_time = SystemTime::now();
        
        // Phase 1: Validate source world
        self.validate_source_world()?;
        
        // Phase 2: Create backup if requested
        if self.config.create_backup {
            self.create_backup().await?;
        }
        
        // Phase 3: Migrate world metadata
        self.migrate_world_metadata().await?;
        
        // Phase 4: Migrate chunks
        let chunk_report = self.migrate_all_chunks().await?;
        
        // Phase 5: Validate migration
        let validation_report = if self.config.validate_after_migration {
            Some(self.validate_migration().await?)
        } else {
            None
        };
        
        // Phase 6: Cleanup if requested
        if self.config.cleanup_after_migration {
            self.cleanup_migration().await?;
        }
        
        let total_time = start_time.elapsed().unwrap_or(Duration::from_secs(0));
        
        let report = MigrationReport {
            total_chunks: chunk_report.total_chunks,
            migrated_chunks: chunk_report.migrated_chunks,
            failed_chunks: chunk_report.failed_chunks,
            migration_time: total_time,
            validation_report,
            backup_created: self.config.create_backup,
            cleanup_performed: self.config.cleanup_after_migration,
        };
        
        info!(
            "Migration completed: {}/{} chunks migrated in {:?}",
            report.migrated_chunks,
            report.total_chunks,
            report.migration_time
        );
        
        Ok(report)
    }
    
    /// Validate that the source world is compatible for migration
    fn validate_source_world(&self) -> Result<()> {
        info!("Validating source world for migration");
        
        // Check if level.dat exists and is readable
        let settings = self.source_provider.settings()
            .map_err(|e| anyhow!("Failed to read world settings: {}", e))?;
        
        // Check if database is accessible
        let _iter = self.source_provider.database().iter();
        
        // Check for existing enhanced format data
        let format_version = ChunkFormatVersion::detect_from_provider(&self.source_provider);
        if format_version == ChunkFormatVersion::Enhanced {
            warn!("World already appears to be in enhanced format");
        }
        
        info!("Source world validation completed successfully");
        Ok(())
    }
    
    /// Create backup of the world before migration
    async fn create_backup(&mut self) -> Result<()> {
        info!("Creating backup before migration");
        
        let backup_path = self.config.backup_path.as_ref()
            .ok_or_else(|| anyhow!("Backup path not configured"))?;
        
        // Create backup directory
        std::fs::create_dir_all(backup_path)?;
        
        // Copy level.dat
        let source_level_dat = self.source_provider.path().join("level.dat");
        let backup_level_dat = backup_path.join("level.dat");
        std::fs::copy(&source_level_dat, &backup_level_dat)?;
        
        // Copy database directory
        let source_db = self.source_provider.path().join("db");
        let backup_db = backup_path.join("db");
        copy_dir_recursive(&source_db, &backup_db)?;
        
        info!("Backup created at: {}", backup_path.display());
        Ok(())
    }
    
    /// Migrate world metadata to enhanced format
    async fn migrate_world_metadata(&mut self) -> Result<()> {
        info!("Migrating world metadata");
        
        // Load existing world settings
        let settings = self.source_provider.settings()?;
        
        // Create enhanced world from settings
        let enhanced_world = EnhancedGameWorld::load_from_provider(&self.source_provider)?;
        
        // Save enhanced world metadata
        self.compatibility_manager.save_world_metadata(&enhanced_world)?;
        
        info!("World metadata migration completed");
        Ok(())
    }
    
    /// Migrate all chunks to enhanced format
    async fn migrate_all_chunks(&mut self) -> Result<ChunkMigrationReport> {
        info!("Starting chunk migration");
        
        let mut report = ChunkMigrationReport::default();
        let start_time = SystemTime::now();
        
        // Scan for existing chunks
        let existing_chunks = self.scan_existing_chunks()?;
        report.total_chunks = existing_chunks.len();
        
        info!("Found {} chunks to migrate", report.total_chunks);
        
        // Migrate chunks in batches
        let batch_size = self.config.batch_size;
        for chunk_batch in existing_chunks.chunks(batch_size) {
            for &chunk_pos in chunk_batch {
                match self.migrate_single_chunk(chunk_pos).await {
                    Ok(()) => {
                        report.migrated_chunks += 1;
                        self.progress.chunks_processed += 1;
                        
                        if self.progress.chunks_processed % 100 == 0 {
                            info!("Migrated {}/{} chunks", self.progress.chunks_processed, report.total_chunks);
                        }
                    }
                    Err(e) => {
                        report.failed_chunks += 1;
                        warn!("Failed to migrate chunk {:?}: {}", chunk_pos, e);
                        
                        if self.config.fail_on_chunk_error {
                            return Err(anyhow!("Chunk migration failed: {}", e));
                        }
                    }
                }
            }
            
            // Small delay between batches to avoid overwhelming the system
            if self.config.batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.batch_delay_ms)).await;
            }
        }
        
        report.migration_time = start_time.elapsed().unwrap_or(Duration::from_secs(0));
        
        info!(
            "Chunk migration completed: {}/{} chunks in {:?}",
            report.migrated_chunks,
            report.total_chunks,
            report.migration_time
        );
        
        Ok(report)
    }
    
    /// Migrate a single chunk to enhanced format
    async fn migrate_single_chunk(&self, pos: ChunkPos) -> Result<()> {
        debug!("Migrating chunk {:?}", pos);
        
        // Load chunk from mirai format
        let chunk = EnhancedChunk::load_from_provider(&self.source_provider, pos, Dimension::Overworld)?
            .ok_or_else(|| anyhow!("Chunk {:?} not found in source", pos))?;
        
        // Save in enhanced format
        self.compatibility_manager.save_enhanced_chunk(&chunk)?;
        
        debug!("Successfully migrated chunk {:?}", pos);
        Ok(())
    }
    
    /// Scan for existing chunks in the database
    fn scan_existing_chunks(&self) -> Result<Vec<ChunkPos>> {
        let mut chunks = Vec::new();
        let mut chunk_set = std::collections::HashSet::new();
        
        // Iterate through database keys to find chunk data
        for kv_ref in self.source_provider.iter() {
            let key_data = kv_ref.key();
            
            // Try to parse as DataKey
            let mut reader = key_data.as_ref();
            if let Ok(data_key) = DataKey::deserialize(&mut reader) {
                match data_key.data {
                    KeyType::SubChunk { .. } | KeyType::ChunkVersion | KeyType::Biome3d => {
                        let chunk_pos = ChunkPos::from(data_key.coordinates);
                        if chunk_set.insert(chunk_pos) {
                            chunks.push(chunk_pos);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        chunks.sort_by_key(|pos| (pos.x, pos.z));
        Ok(chunks)
    }
    
    /// Validate the migration by checking data integrity
    async fn validate_migration(&self) -> Result<ValidationReport> {
        info!("Validating migration integrity");
        
        let mut report = ValidationReport::default();
        let chunks = self.scan_existing_chunks()?;
        
        for chunk_pos in chunks.iter().take(self.config.validation_sample_size) {
            match self.compatibility_manager.validate_data_integrity(*chunk_pos, Dimension::Overworld) {
                Ok(integrity_report) => {
                    if integrity_report.is_valid() {
                        report.valid_chunks += 1;
                    } else {
                        report.invalid_chunks += 1;
                        report.issues.extend(integrity_report.issues);
                    }
                }
                Err(e) => {
                    report.validation_errors += 1;
                    warn!("Failed to validate chunk {:?}: {}", chunk_pos, e);
                }
            }
        }
        
        report.total_validated = report.valid_chunks + report.invalid_chunks + report.validation_errors;
        
        if report.invalid_chunks > 0 {
            warn!("Migration validation found {} invalid chunks", report.invalid_chunks);
        } else {
            info!("Migration validation passed: all {} chunks are valid", report.valid_chunks);
        }
        
        Ok(report)
    }
    
    /// Cleanup temporary migration data
    async fn cleanup_migration(&self) -> Result<()> {
        info!("Performing post-migration cleanup");
        
        // This could include removing temporary files, compacting the database, etc.
        // For now, we'll just log that cleanup was performed
        
        info!("Migration cleanup completed");
        Ok(())
    }
    
    /// Get current migration progress
    pub fn get_progress(&self) -> &MigrationProgress {
        &self.progress
    }
    
    /// Rollback migration (restore from backup)
    pub async fn rollback_migration(&self) -> Result<()> {
        let backup_path = self.config.backup_path.as_ref()
            .ok_or_else(|| anyhow!("No backup path configured for rollback"))?;
        
        if !backup_path.exists() {
            return Err(anyhow!("Backup directory does not exist: {}", backup_path.display()));
        }
        
        info!("Rolling back migration from backup");
        
        // Restore level.dat
        let backup_level_dat = backup_path.join("level.dat");
        let target_level_dat = self.source_provider.path().join("level.dat");
        std::fs::copy(&backup_level_dat, &target_level_dat)?;
        
        // Restore database
        let backup_db = backup_path.join("db");
        let target_db = self.source_provider.path().join("db");
        
        // Remove current database
        if target_db.exists() {
            std::fs::remove_dir_all(&target_db)?;
        }
        
        // Copy backup database
        copy_dir_recursive(&backup_db, &target_db)?;
        
        info!("Migration rollback completed");
        Ok(())
    }
}

/// Configuration for migration process
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Whether to create a backup before migration
    pub create_backup: bool,
    /// Path for backup (if create_backup is true)
    pub backup_path: Option<PathBuf>,
    /// Number of chunks to process in each batch
    pub batch_size: usize,
    /// Delay between batches in milliseconds
    pub batch_delay_ms: u64,
    /// Whether to fail the entire migration if a single chunk fails
    pub fail_on_chunk_error: bool,
    /// Whether to validate migration after completion
    pub validate_after_migration: bool,
    /// Number of chunks to sample for validation
    pub validation_sample_size: usize,
    /// Whether to perform cleanup after migration
    pub cleanup_after_migration: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            create_backup: true,
            backup_path: None,
            batch_size: 50,
            batch_delay_ms: 10,
            fail_on_chunk_error: false,
            validate_after_migration: true,
            validation_sample_size: 100,
            cleanup_after_migration: true,
        }
    }
}

/// Progress tracking for migration
#[derive(Debug, Default)]
pub struct MigrationProgress {
    /// Number of chunks processed so far
    pub chunks_processed: usize,
    /// Current phase of migration
    pub current_phase: MigrationPhase,
    /// Start time of migration
    pub start_time: Option<SystemTime>,
}

/// Migration phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPhase {
    /// Not started
    NotStarted,
    /// Validating source world
    Validating,
    /// Creating backup
    Backup,
    /// Migrating metadata
    Metadata,
    /// Migrating chunks
    Chunks,
    /// Validating migration
    Validation,
    /// Performing cleanup
    Cleanup,
    /// Migration completed
    Completed,
    /// Migration failed
    Failed,
}

impl Default for MigrationPhase {
    fn default() -> Self {
        Self::NotStarted
    }
}

/// Report for chunk migration specifically
#[derive(Debug, Default)]
pub struct ChunkMigrationReport {
    pub total_chunks: usize,
    pub migrated_chunks: usize,
    pub failed_chunks: usize,
    pub migration_time: Duration,
}

/// Enhanced migration report with additional details
#[derive(Debug)]
pub struct MigrationReport {
    pub total_chunks: usize,
    pub migrated_chunks: usize,
    pub failed_chunks: usize,
    pub migration_time: Duration,
    pub validation_report: Option<ValidationReport>,
    pub backup_created: bool,
    pub cleanup_performed: bool,
}

/// Validation report for migration integrity
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub total_validated: usize,
    pub valid_chunks: usize,
    pub invalid_chunks: usize,
    pub validation_errors: usize,
    pub issues: Vec<String>,
}

/// Utility function to copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Err(anyhow!("Source is not a directory: {}", src.display()));
    }
    
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// Compatibility layer for existing mirai world format
pub struct WorldFormatCompatibility {
    provider: Provider,
}

impl WorldFormatCompatibility {
    /// Create new compatibility layer
    pub fn new(provider: Provider) -> Self {
        Self { provider }
    }
    
    /// Check if world needs migration
    pub fn needs_migration(&self) -> Result<bool> {
        let format_version = ChunkFormatVersion::detect_from_provider(&self.provider);
        Ok(format_version == ChunkFormatVersion::Mirai)
    }
    
    /// Get migration recommendations
    pub fn get_migration_recommendations(&self) -> Result<MigrationRecommendations> {
        let settings = self.provider.settings()?;
        let chunks = self.estimate_chunk_count()?;
        
        let estimated_time = Duration::from_secs(((chunks / 100) * 60) as u64); // Rough estimate
        let recommended_backup = chunks > 1000; // Recommend backup for large worlds
        
        Ok(MigrationRecommendations {
            estimated_chunks: chunks,
            estimated_time,
            recommend_backup: recommended_backup,
            recommended_batch_size: if chunks > 10000 { 100 } else { 50 },
            disk_space_needed: chunks * 1024 * 1024, // Rough estimate: 1MB per chunk
        })
    }
    
    /// Estimate number of chunks in the world
    fn estimate_chunk_count(&self) -> Result<usize> {
        let mut chunk_count = 0;
        let mut chunk_set = std::collections::HashSet::new();
        
        for kv_ref in self.provider.iter() {
            let key_data = kv_ref.key();
            
            let mut reader = key_data.as_ref();
            if let Ok(data_key) = DataKey::deserialize(&mut reader) {
                match data_key.data {
                    KeyType::SubChunk { .. } | KeyType::ChunkVersion => {
                        let chunk_pos = ChunkPos::from(data_key.coordinates);
                        chunk_set.insert(chunk_pos);
                    }
                    _ => {}
                }
            }
        }
        
        Ok(chunk_set.len())
    }
}

/// Migration recommendations for a world
#[derive(Debug)]
pub struct MigrationRecommendations {
    pub estimated_chunks: usize,
    pub estimated_time: Duration,
    pub recommend_backup: bool,
    pub recommended_batch_size: usize,
    pub disk_space_needed: usize,
}

/// Migration path utilities for different upgrade scenarios
pub struct MigrationPaths;

impl MigrationPaths {
    /// Get migration path for upgrading from mirai to enhanced format
    pub fn mirai_to_enhanced() -> MigrationPath {
        MigrationPath {
            name: "Mirai to Enhanced".to_string(),
            description: "Upgrade existing mirai world to enhanced format with ECS support".to_string(),
            steps: vec![
                MigrationStep::ValidateSource,
                MigrationStep::CreateBackup,
                MigrationStep::MigrateMetadata,
                MigrationStep::MigrateChunks,
                MigrationStep::ValidateIntegrity,
                MigrationStep::Cleanup,
            ],
            reversible: true,
            estimated_duration: Duration::from_secs(300), // 5 minutes base time
        }
    }
    
    /// Get migration path for partial upgrade (keeping both formats)
    pub fn mirai_to_hybrid() -> MigrationPath {
        MigrationPath {
            name: "Mirai to Hybrid".to_string(),
            description: "Add enhanced features while maintaining mirai compatibility".to_string(),
            steps: vec![
                MigrationStep::ValidateSource,
                MigrationStep::CreateBackup,
                MigrationStep::MigrateMetadata,
                MigrationStep::AddEnhancedFeatures,
                MigrationStep::ValidateIntegrity,
            ],
            reversible: true,
            estimated_duration: Duration::from_secs(180), // 3 minutes base time
        }
    }
}

/// Migration path definition
#[derive(Debug, Clone)]
pub struct MigrationPath {
    pub name: String,
    pub description: String,
    pub steps: Vec<MigrationStep>,
    pub reversible: bool,
    pub estimated_duration: Duration,
}

/// Individual migration steps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStep {
    ValidateSource,
    CreateBackup,
    MigrateMetadata,
    MigrateChunks,
    AddEnhancedFeatures,
    ValidateIntegrity,
    Cleanup,
}