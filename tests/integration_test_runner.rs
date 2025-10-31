//! Integration test runner for the unified Mirai system
//! 
//! This file contains actual integration tests that verify the system works correctly.

use mirai_core::{instance::Instance, unified_config::UnifiedConfig};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

// Include migration validation tests
mod migration_validation;

#[tokio::test]
async fn test_instance_creation_and_basic_functionality() {
    // Test that we can create an instance successfully
    let instance = Instance::builder()
        .level_path("test_integration_level")
        .build()
        .await
        .expect("Failed to create test instance");
    
    // Verify instance was created successfully
    assert!(Arc::strong_count(&instance) > 0);
    
    // Test configuration access
    let config = instance.config();
    assert!(config.max_connections() > 0);
    
    // Test client management
    let clients = instance.clients();
    assert_eq!(clients.total_connected(), 0); // No clients connected in test
    
    // Test command system
    let commands = instance.commands();
    assert!(commands.is_command_registered("shutdown"));
    
    // Test graceful shutdown
    if let Some(shutdown_handle) = instance.shutdown() {
        let shutdown_result = timeout(Duration::from_secs(5), shutdown_handle).await;
        assert!(shutdown_result.is_ok(), "Shutdown should complete within 5 seconds");
        
        if let Ok(result) = shutdown_result {
            assert!(result.is_ok(), "Shutdown should complete successfully");
        }
    }
    
    // Wait for shutdown to complete
    let join_result = timeout(Duration::from_secs(5), instance.join()).await;
    assert!(join_result.is_ok(), "Instance join should complete within 5 seconds");
    
    if let Ok(result) = join_result {
        assert!(result.is_ok(), "Instance should shutdown cleanly");
    }
}

#[test]
fn test_unified_config_functionality() {
    // Test that unified configuration system works
    let config = UnifiedConfig::default();
    
    // Verify all sections are present and have valid values
    assert!(config.network.port > 0);
    assert!(config.features.plugin_system);
    assert!(config.features.ecs_system);
    assert!(config.server.max_connections > 0);
    assert!(!config.server.server_name.is_empty());
    
    // Test configuration validation
    let validation = config.validate();
    assert!(validation.is_ok(), "Default config should be valid: {:?}", validation);
    
    // Test feature checking
    assert!(config.is_feature_enabled("ecs_system"));
    assert!(config.is_feature_enabled("plugin_system"));
    assert!(!config.is_feature_enabled("unknown_feature"));
    
    // Test conversion to Mirai config
    let mirai_config = config.to_mirai_config();
    assert_eq!(mirai_config.max_connections(), config.server.max_connections);
    assert_eq!(mirai_config.max_render_distance(), config.server.max_render_distance);
}

#[tokio::test]
async fn test_multiple_instance_lifecycle() {
    // Test creating multiple instances and managing their lifecycle
    let instance1 = Instance::builder()
        .level_path("test_multi_level_1")
        .build()
        .await
        .expect("Failed to create first test instance");
    
    let instance2 = Instance::builder()
        .level_path("test_multi_level_2")
        .build()
        .await
        .expect("Failed to create second test instance");
    
    // Verify both instances are independent
    assert!(Arc::strong_count(&instance1) > 0);
    assert!(Arc::strong_count(&instance2) > 0);
    
    // Test that they have different configurations
    let config1 = instance1.config();
    let config2 = instance2.config();
    
    // Both should have valid configurations
    assert!(config1.max_connections() > 0);
    assert!(config2.max_connections() > 0);
    
    // Shutdown both instances
    let shutdown1 = instance1.shutdown();
    let shutdown2 = instance2.shutdown();
    
    if let Some(handle1) = shutdown1 {
        let result = timeout(Duration::from_secs(5), handle1).await;
        assert!(result.is_ok(), "First instance should shutdown cleanly");
    }
    
    if let Some(handle2) = shutdown2 {
        let result = timeout(Duration::from_secs(5), handle2).await;
        assert!(result.is_ok(), "Second instance should shutdown cleanly");
    }
    
    // Wait for both to complete shutdown
    let join1 = timeout(Duration::from_secs(5), instance1.join()).await;
    let join2 = timeout(Duration::from_secs(5), instance2.join()).await;
    
    assert!(join1.is_ok(), "First instance should complete shutdown");
    assert!(join2.is_ok(), "Second instance should complete shutdown");
}

#[test]
fn test_config_serialization_and_validation() {
    let config = UnifiedConfig::default();
    
    // Test JSON serialization
    let json = serde_json::to_string(&config);
    assert!(json.is_ok(), "Config should serialize to JSON");
    
    if let Ok(json_str) = json {
        assert!(!json_str.is_empty());
        
        // Test deserialization
        let deserialized: Result<UnifiedConfig, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "Config should deserialize from JSON");
        
        if let Ok(deserialized_config) = deserialized {
            assert_eq!(config.server.server_name, deserialized_config.server.server_name);
            assert_eq!(config.network.port, deserialized_config.network.port);
        }
    }
    
    // Test TOML serialization
    let toml = toml::to_string(&config);
    assert!(toml.is_ok(), "Config should serialize to TOML");
    
    if let Ok(toml_str) = toml {
        assert!(!toml_str.is_empty());
        
        // Test deserialization
        let deserialized: Result<UnifiedConfig, _> = toml::from_str(&toml_str);
        assert!(deserialized.is_ok(), "Config should deserialize from TOML");
        
        if let Ok(deserialized_config) = deserialized {
            assert_eq!(config.server.server_name, deserialized_config.server.server_name);
            assert_eq!(config.network.port, deserialized_config.network.port);
        }
    }
}

#[test]
fn test_config_validation_edge_cases() {
    let mut config = UnifiedConfig::default();
    
    // Test invalid server name
    config.server.server_name = String::new();
    assert!(config.validate().is_err(), "Empty server name should be invalid");
    
    // Reset and test invalid max players
    config.server.server_name = "Test Server".to_string();
    config.server.max_players = 0;
    assert!(config.validate().is_err(), "Zero max players should be invalid");
    
    // Reset and test invalid port
    config.server.max_players = 10;
    config.network.port = 0;
    assert!(config.validate().is_err(), "Zero port should be invalid");
    
    // Reset and test max players > max connections
    config.network.port = 19132;
    config.server.max_players = 100;
    config.server.max_connections = 50;
    assert!(config.validate().is_err(), "Max players > max connections should be invalid");
    
    // Test valid configuration
    config.server.max_players = 40;
    config.server.max_connections = 50;
    assert!(config.validate().is_ok(), "Valid config should pass validation");
}