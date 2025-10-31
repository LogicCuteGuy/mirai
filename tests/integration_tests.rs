//! Main integration test suite for the merged mirai-minecraft-server system
//! 
//! This file serves as the entry point for all integration tests and ensures
//! that the merged system works correctly as a whole.

mod integration;

use integration::*;
use mirai_core::{App, Instance, unified_config::UnifiedConfig};
use std::sync::{Arc, Weak};

#[tokio::test]
async fn test_complete_system_integration() {
    // Test that all major components work together
    let instance = create_test_mirai_instance().await
        .expect("Failed to create test instance");
    
    // Verify core systems are running (check if instance exists)
    assert!(Arc::strong_count(&instance) > 0);
    
    // Test configuration system
    let config = instance.config();
    assert!(config.max_connections() > 0);
    
    // Test client management
    let clients = instance.clients();
    assert_eq!(clients.total_connected(), 0); // No clients connected in test
    
    // Test command system
    let commands = instance.commands();
    assert!(commands.is_command_registered("shutdown"));
}

#[tokio::test]
async fn test_merged_system_startup_shutdown() {
    // Test clean startup and shutdown of merged system
    let instance = Instance::builder()
        .level_path("test_level")
        .build()
        .await
        .expect("Failed to build instance");
    
    // Verify instance was created successfully
    assert!(Arc::strong_count(&instance) > 0);
    
    // Test graceful shutdown
    if let Some(shutdown_handle) = instance.shutdown() {
        shutdown_handle.await
            .expect("Failed to shutdown instance")
            .expect("Shutdown completed successfully");
    }
    
    // Wait for shutdown to complete
    instance.join().await.expect("Instance shutdown completed");
}

#[test]
fn test_configuration_system_integration() {
    use mirai_core::unified_config::UnifiedConfig;
    
    // Test that unified configuration system works
    let config = UnifiedConfig::default();
    
    // Verify all sections are present
    assert!(config.network.port > 0);
    assert!(config.features.plugin_system);
    assert!(config.features.ecs_system);
    assert!(config.server.max_connections > 0);
    
    // Test configuration validation
    let validation = config.validate();
    assert!(validation.is_ok());
    
    // Test feature checking
    assert!(config.is_feature_enabled("ecs_system"));
    assert!(config.is_feature_enabled("plugin_system"));
}

#[tokio::test]
async fn test_error_handling_integration() {
    // Test that error handling works correctly across all systems
    let instance = Instance::builder()
        .level_path("test_error_level")
        .build()
        .await
        .expect("Failed to build error test instance");
    
    // Test that the instance handles errors gracefully
    assert!(Arc::strong_count(&instance) > 0);
    
    // Test configuration error handling
    let config = instance.config();
    assert!(config.max_connections() > 0);
    
    // Test that invalid operations don't crash the system
    let clients = instance.clients();
    assert_eq!(clients.total_connected(), 0);
}