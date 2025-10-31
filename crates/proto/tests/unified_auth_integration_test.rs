//! Integration tests for unified authentication and encryption system
//! 
//! This test suite verifies that the unified authentication and encryption system
//! correctly handles both Java and Bedrock authentication and encryption.

use mirai_proto::{
    UnifiedAuthService, UnifiedAuthConfig, UnifiedAuthData, ProtocolType,
    JavaAuthConfig, BedrockAuthConfig, UnifiedConnectionState
};
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn test_unified_auth_service_creation() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let stats = auth_service.get_auth_stats();
    assert_eq!(stats.total_authentications, 0);
    assert_eq!(stats.java_authentications, 0);
    assert_eq!(stats.bedrock_authentications, 0);
}

#[test]
fn test_java_offline_authentication() {
    let mut config = UnifiedAuthConfig::default();
    config.java_config.offline_mode = true;
    
    let auth_service = UnifiedAuthService::new(config);
    
    let mut auth_data = UnifiedAuthData::default();
    auth_data.username = Some("TestPlayer".to_string());
    
    let result = auth_service.authenticate_player(ProtocolType::Java, &auth_data);
    assert!(result.is_ok());
    
    let profile = result.unwrap();
    assert_eq!(profile.username, "TestPlayer");
    assert_eq!(profile.protocol_type, ProtocolType::Java);
    assert!(profile.xuid.is_none());
    assert!(profile.skin_data.is_none());
}

#[test]
fn test_bedrock_authentication() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let mut auth_data = UnifiedAuthData::default();
    auth_data.username = Some("BedrockPlayer".to_string());
    auth_data.xuid = Some("1234567890".to_string());
    
    let result = auth_service.authenticate_player(ProtocolType::Bedrock, &auth_data);
    assert!(result.is_ok());
    
    let profile = result.unwrap();
    assert_eq!(profile.username, "BedrockPlayer");
    assert_eq!(profile.protocol_type, ProtocolType::Bedrock);
    assert_eq!(profile.xuid, Some(1234567890));
}

#[test]
fn test_bedrock_identity_chain_authentication() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let mut auth_data = UnifiedAuthData::default();
    auth_data.username = Some("BedrockPlayer".to_string());
    auth_data.identity_chain = Some(vec![
        "mock_jwt_token_1".to_string(),
        "mock_jwt_token_2".to_string(),
    ]);
    auth_data.xuid = Some("9876543210".to_string());
    
    let result = auth_service.authenticate_player(ProtocolType::Bedrock, &auth_data);
    assert!(result.is_ok());
    
    let profile = result.unwrap();
    assert_eq!(profile.username, "BedrockPlayer");
    assert_eq!(profile.protocol_type, ProtocolType::Bedrock);
    assert_eq!(profile.xuid, Some(9876543210));
}

#[test]
fn test_encryption_management() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let connection_id = Uuid::new_v4();
    let shared_secret = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    
    // Enable encryption for Java connection
    let result = auth_service.enable_encryption(connection_id, ProtocolType::Java, shared_secret.clone());
    assert!(result.is_ok());
    
    // Test encryption/decryption
    let test_data = b"Hello, World!";
    let encrypted = auth_service.encrypt_data(connection_id, test_data).unwrap();
    let decrypted = auth_service.decrypt_data(connection_id, &encrypted).unwrap();
    
    assert_eq!(decrypted.as_ref(), test_data);
}

#[test]
fn test_bedrock_encryption_token_generation() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let client_public_key = "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAE8ELkixyLcwlZryUQcu1TvPOmI2B7vX83ndnWRUaXm74wFfa5f/lwQNTfrLVHa2PmenpGI6JhIMUJaWZrjmMj90NoKNFSNBuKdm8rYiXsfaz3K36x/1U26HpG0ZxK/V1V";
    
    let result = auth_service.generate_bedrock_encryption_token(client_public_key);
    assert!(result.is_ok());
    
    let token = result.unwrap();
    assert!(!token.is_empty());
}

#[test]
fn test_verify_token_generation() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    let token1 = auth_service.generate_verify_token();
    let token2 = auth_service.generate_verify_token();
    
    assert_eq!(token1.len(), 4);
    assert_eq!(token2.len(), 4);
    // Tokens should be different (very high probability)
    assert_ne!(token1, token2);
}

#[test]
fn test_authentication_statistics() {
    let mut config = UnifiedAuthConfig::default();
    config.java_config.offline_mode = true;
    
    let auth_service = UnifiedAuthService::new(config);
    
    // Perform multiple authentications
    for i in 0..5 {
        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some(format!("Player{}", i));
        
        let result = auth_service.authenticate_player(ProtocolType::Java, &auth_data);
        assert!(result.is_ok());
    }
    
    for i in 0..3 {
        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some(format!("BedrockPlayer{}", i));
        
        let result = auth_service.authenticate_player(ProtocolType::Bedrock, &auth_data);
        assert!(result.is_ok());
    }
    
    let stats = auth_service.get_auth_stats();
    assert_eq!(stats.total_authentications, 8);
    assert_eq!(stats.java_authentications, 5);
    assert_eq!(stats.bedrock_authentications, 3);
}

#[test]
fn test_skin_validation() {
    let config = UnifiedAuthConfig::default();
    let auth_service = UnifiedAuthService::new(config);
    
    // Valid skin data (64x64 RGBA)
    let valid_skin = vec![0u8; 64 * 64 * 4];
    let result = auth_service.validate_skin(&valid_skin);
    assert!(result.is_ok());
    
    // Invalid skin data (too small)
    let invalid_skin = vec![0u8; 100];
    let result = auth_service.validate_skin(&invalid_skin);
    assert!(result.is_err());
    
    // Invalid skin data (too large)
    let invalid_skin = vec![0u8; 256 * 256 * 4];
    let result = auth_service.validate_skin(&invalid_skin);
    assert!(result.is_err());
}

#[test]
fn test_unified_auth_config_default() {
    let config = UnifiedAuthConfig::default();
    
    assert!(!config.java_config.offline_mode);
    assert!(config.bedrock_config.require_xbox_live);
    assert!(config.enable_caching);
    assert_eq!(config.cache_timeout, 300);
}

#[test]
fn test_protocol_type_enum() {
    assert_ne!(ProtocolType::Java, ProtocolType::Bedrock);
    
    // Test Debug formatting
    assert_eq!(format!("{:?}", ProtocolType::Java), "Java");
    assert_eq!(format!("{:?}", ProtocolType::Bedrock), "Bedrock");
}

#[test]
fn test_unified_connection_state_transitions() {
    assert!(UnifiedConnectionState::Handshaking.is_active());
    assert!(UnifiedConnectionState::Play.is_active());
    assert!(!UnifiedConnectionState::Disconnected.is_active());
    assert!(UnifiedConnectionState::Disconnected.is_closed());
}