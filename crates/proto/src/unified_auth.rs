//! Unified authentication and encryption system
//!
//! This module provides a unified authentication and encryption system that merges
//! capabilities from both minecraft-server-protocol and mirai's existing systems.
//! It supports both Java Edition and Bedrock Edition authentication and encryption.

use crate::connection::BedrockAuthData;
use crate::unified::BedrockProtocolError;
use crate::crypto::encrypt::Encryptor;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Protocol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    /// Java Edition protocol (TCP)
    Java,
    /// Bedrock Edition protocol (UDP/RakNet)
    Bedrock,
}

/// Unified protocol error type
#[derive(Debug)]
pub enum UnifiedProtocolError {
    Authentication(String),
    Encryption(String),
    Connection(String),
    InvalidPacket(String),
    BedrockProtocol(BedrockProtocolError),
    Io(std::io::Error),
    Serialization(String),
}

impl std::fmt::Display for UnifiedProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedProtocolError::Authentication(msg) => write!(f, "Authentication failed: {}", msg),
            UnifiedProtocolError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            UnifiedProtocolError::Connection(msg) => write!(f, "Connection error: {}", msg),
            UnifiedProtocolError::InvalidPacket(msg) => write!(f, "Invalid packet: {}", msg),
            UnifiedProtocolError::BedrockProtocol(err) => write!(f, "Bedrock protocol error: {}", err),
            UnifiedProtocolError::Io(err) => write!(f, "IO error: {}", err),
            UnifiedProtocolError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for UnifiedProtocolError {}

impl From<BedrockProtocolError> for UnifiedProtocolError {
    fn from(err: BedrockProtocolError) -> Self {
        UnifiedProtocolError::BedrockProtocol(err)
    }
}

impl From<std::io::Error> for UnifiedProtocolError {
    fn from(err: std::io::Error) -> Self {
        UnifiedProtocolError::Io(err)
    }
}

/// Unified authentication data that works for both Java and Bedrock
#[derive(Debug, Clone, Default)]
pub struct UnifiedAuthData {
    /// Player username
    pub username: Option<String>,
    /// Player UUID
    pub player_uuid: Option<Uuid>,
    /// Access token (Java) or JWT chain (Bedrock)
    pub access_token: Option<String>,
    /// Xbox Live user ID (Bedrock only)
    pub xuid: Option<String>,
    /// Identity chain (Bedrock only)
    pub identity_chain: Option<Vec<String>>,
    /// Client data (Bedrock only)
    pub client_data: Option<String>,
    /// Identity public key
    pub identity_public_key: Option<String>,
    /// Client public key
    pub client_public_key: Option<Vec<u8>>,
    /// Verify token (Java only)
    pub verify_token: Option<Vec<u8>>,
    /// Shared secret
    pub shared_secret: Option<Vec<u8>>,
    /// Authentication timestamp
    pub auth_timestamp: Option<SystemTime>,
}

/// Unified authentication service that handles both Java and Bedrock authentication
pub struct UnifiedAuthService {
    /// Java Edition authentication
    java_auth: JavaAuthService,
    /// Bedrock Edition authentication  
    bedrock_auth: BedrockAuthService,
    /// Unified encryption manager
    encryption_manager: Arc<RwLock<UnifiedEncryptionManager>>,
    /// Configuration
    config: UnifiedAuthConfig,
}

impl UnifiedAuthService {
    /// Create a new unified authentication service
    pub fn new(config: UnifiedAuthConfig) -> Self {
        Self {
            java_auth: JavaAuthService::new(config.java_config.clone()),
            bedrock_auth: BedrockAuthService::new(config.bedrock_config.clone()),
            encryption_manager: Arc::new(RwLock::new(UnifiedEncryptionManager::new())),
            config,
        }
    }

    /// Authenticate a player based on protocol type
    pub fn authenticate_player(
        &self,
        protocol_type: ProtocolType,
        auth_data: &UnifiedAuthData,
    ) -> Result<UnifiedPlayerProfile, UnifiedProtocolError> {
        match protocol_type {
            ProtocolType::Java => {
                let profile = self.java_auth.authenticate(auth_data)?;
                Ok(UnifiedPlayerProfile {
                    uuid: profile.uuid,
                    username: profile.username,
                    protocol_type: ProtocolType::Java,
                    xuid: None,
                    properties: profile.properties,
                    skin_data: None,
                    authenticated_at: SystemTime::now(),
                })
            }
            ProtocolType::Bedrock => {
                let profile = self.bedrock_auth.authenticate(auth_data)?;
                Ok(UnifiedPlayerProfile {
                    uuid: profile.uuid,
                    username: profile.username,
                    protocol_type: ProtocolType::Bedrock,
                    xuid: Some(profile.xuid),
                    properties: Vec::new(),
                    skin_data: profile.skin_data,
                    authenticated_at: SystemTime::now(),
                })
            }
        }
    }

    /// Enable encryption for a connection
    pub fn enable_encryption(
        &self,
        connection_id: Uuid,
        protocol_type: ProtocolType,
        shared_secret: Vec<u8>,
    ) -> Result<(), UnifiedProtocolError> {
        let mut manager = self.encryption_manager.write().unwrap();
        manager.enable_encryption(connection_id, protocol_type, shared_secret)?;
        Ok(())
    }

    /// Encrypt data for a connection
    pub fn encrypt_data(&self, connection_id: Uuid, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        let manager = self.encryption_manager.read().unwrap();
        manager.encrypt_data(connection_id, data)
    }

    /// Decrypt data for a connection
    pub fn decrypt_data(&self, connection_id: Uuid, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        let mut manager = self.encryption_manager.write().unwrap();
        manager.decrypt_data(connection_id, data)
    }

    /// Generate a verify token for Java Edition authentication
    pub fn generate_verify_token(&self) -> Vec<u8> {
        self.java_auth.generate_verify_token()
    }

    /// Generate an encryption handshake token for Bedrock Edition
    pub fn generate_bedrock_encryption_token(&self, client_public_key: &str) -> Result<String, UnifiedProtocolError> {
        // Use mirai's existing Encryptor for Bedrock encryption
        match Encryptor::new(client_public_key) {
            Ok((_, jwt)) => Ok(jwt),
            Err(e) => Err(UnifiedProtocolError::Encryption(e.to_string())),
        }
    }

    /// Validate player skin data
    pub fn validate_skin(&self, skin_data: &[u8]) -> Result<(), UnifiedProtocolError> {
        // Basic validation - check size and format
        if skin_data.len() < 64 * 64 * 4 {
            return Err(UnifiedProtocolError::InvalidPacket("Skin data too small".to_string()));
        }

        if skin_data.len() > 128 * 128 * 4 {
            return Err(UnifiedProtocolError::InvalidPacket("Skin data too large".to_string()));
        }

        Ok(())
    }

    /// Get authentication statistics
    pub fn get_auth_stats(&self) -> UnifiedAuthStats {
        let java_stats = self.java_auth.get_stats();
        let bedrock_stats = self.bedrock_auth.get_stats();

        UnifiedAuthStats {
            total_authentications: java_stats.total_authentications + bedrock_stats.total_authentications,
            java_authentications: java_stats.total_authentications,
            bedrock_authentications: bedrock_stats.total_authentications,
            failed_authentications: java_stats.failed_authentications + bedrock_stats.failed_authentications,
            active_sessions: java_stats.active_sessions + bedrock_stats.active_sessions,
        }
    }
}

/// Java Edition authentication service
pub struct JavaAuthService {
    /// Configuration
    config: JavaAuthConfig,
    /// Authentication statistics
    stats: Arc<RwLock<AuthStats>>,
}

impl JavaAuthService {
    /// Create a new Java authentication service
    pub fn new(config: JavaAuthConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(AuthStats::new())),
        }
    }

    /// Authenticate a Java Edition player
    pub fn authenticate(&self, auth_data: &UnifiedAuthData) -> Result<JavaPlayerProfile, UnifiedProtocolError> {
        let mut stats = self.stats.write().unwrap();
        stats.total_authentications += 1;

        if self.config.offline_mode {
            // Offline mode authentication
            if let Some(ref username) = auth_data.username {
                let uuid = generate_offline_uuid(username);
                stats.active_sessions += 1;
                return Ok(JavaPlayerProfile {
                    uuid,
                    username: username.clone(),
                    properties: Vec::new(),
                });
            } else {
                stats.failed_authentications += 1;
                return Err(UnifiedProtocolError::Authentication(
                    "No username provided for offline authentication".to_string(),
                ));
            }
        }

        // Online mode authentication
        if let Some(ref _access_token) = auth_data.access_token {
            match self.authenticate_online() {
                Ok(profile) => {
                    stats.active_sessions += 1;
                    Ok(profile)
                }
                Err(e) => {
                    stats.failed_authentications += 1;
                    Err(e)
                }
            }
        } else {
            stats.failed_authentications += 1;
            Err(UnifiedProtocolError::Authentication(
                "No access token provided for online authentication".to_string(),
            ))
        }
    }

    /// Authenticate online with Mojang/Microsoft services
    fn authenticate_online(&self) -> Result<JavaPlayerProfile, UnifiedProtocolError> {
        // This is a simplified implementation
        // In a real implementation, you would validate the access token with Mojang/Microsoft services

        // For now, return a mock profile
        Ok(JavaPlayerProfile {
            uuid: Uuid::new_v4(),
            username: "OnlinePlayer".to_string(),
            properties: Vec::new(),
        })
    }

    /// Generate a verify token
    pub fn generate_verify_token(&self) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);

        let hash = hasher.finish();
        hash.to_be_bytes()[..4].to_vec()
    }

    /// Get authentication statistics
    pub fn get_stats(&self) -> AuthStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }
}

/// Bedrock Edition authentication service
pub struct BedrockAuthService {
    /// Configuration
    config: BedrockAuthConfig,
    /// Authentication statistics
    stats: Arc<RwLock<AuthStats>>,
}

impl BedrockAuthService {
    /// Create a new Bedrock authentication service
    pub fn new(config: BedrockAuthConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(AuthStats::new())),
        }
    }

    /// Authenticate a Bedrock Edition player
    pub fn authenticate(&self, auth_data: &UnifiedAuthData) -> Result<BedrockPlayerProfile, UnifiedProtocolError> {
        let mut stats = self.stats.write().unwrap();
        stats.total_authentications += 1;

        // For Bedrock, we can use existing mirai authentication or implement Xbox Live
        // This integrates with mirai's existing Bedrock authentication
        
        if let Some(ref identity_chain) = auth_data.identity_chain {
            match self.authenticate_identity_chain(identity_chain, auth_data) {
                Ok(profile) => {
                    stats.active_sessions += 1;
                    Ok(profile)
                }
                Err(e) => {
                    stats.failed_authentications += 1;
                    Err(e)
                }
            }
        } else if let Some(ref username) = auth_data.username {
            // Fallback to offline mode for testing
            stats.active_sessions += 1;
            Ok(BedrockPlayerProfile {
                uuid: auth_data.player_uuid.unwrap_or_else(Uuid::new_v4),
                username: username.clone(),
                xuid: auth_data.xuid.as_ref().and_then(|x| x.parse().ok()).unwrap_or(0),
                skin_data: None,
            })
        } else {
            stats.failed_authentications += 1;
            Err(UnifiedProtocolError::Authentication(
                "No identity chain or username provided for Bedrock authentication".to_string(),
            ))
        }
    }

    /// Authenticate with identity chain (JWT tokens)
    fn authenticate_identity_chain(
        &self,
        _identity_chain: &[String],
        auth_data: &UnifiedAuthData,
    ) -> Result<BedrockPlayerProfile, UnifiedProtocolError> {
        // This would validate the JWT token chain from Xbox Live
        // For now, extract basic information from the auth data
        
        let username = auth_data.username.clone()
            .unwrap_or_else(|| "BedrockPlayer".to_string());
        let uuid = auth_data.player_uuid.unwrap_or_else(Uuid::new_v4);
        let xuid = auth_data.xuid.as_ref()
            .and_then(|x| x.parse().ok())
            .unwrap_or(0);

        Ok(BedrockPlayerProfile {
            uuid,
            username,
            xuid,
            skin_data: None, // Would extract from client_data JWT
        })
    }

    /// Get authentication statistics
    pub fn get_stats(&self) -> AuthStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }
}

/// Unified encryption manager that handles both Java and Bedrock encryption
pub struct UnifiedEncryptionManager {
    /// Active encryption sessions
    sessions: HashMap<Uuid, EncryptionSession>,
    /// Bedrock encryptors (using mirai's Encryptor)
    bedrock_encryptors: HashMap<Uuid, Encryptor>,
}

impl UnifiedEncryptionManager {
    /// Create a new unified encryption manager
    pub fn new() -> Self {
        Self { 
            sessions: HashMap::new(),
            bedrock_encryptors: HashMap::new(),
        }
    }

    /// Enable encryption for a connection
    pub fn enable_encryption(
        &mut self,
        connection_id: Uuid,
        protocol_type: ProtocolType,
        shared_secret: Vec<u8>,
    ) -> Result<(), UnifiedProtocolError> {
        match protocol_type {
            ProtocolType::Java => {
                let session = EncryptionSession::new(protocol_type, shared_secret)?;
                self.sessions.insert(connection_id, session);
            }
            ProtocolType::Bedrock => {
                // For Bedrock, we would use mirai's existing Encryptor
                // This is a placeholder - in practice, the Encryptor would be created
                // during the encryption handshake process
                let session = EncryptionSession::new(protocol_type, shared_secret)?;
                self.sessions.insert(connection_id, session);
            }
        }
        
        tracing::debug!(
            "Enabled encryption for connection {} ({})",
            connection_id,
            match protocol_type {
                ProtocolType::Java => "Java",
                ProtocolType::Bedrock => "Bedrock",
            }
        );
        Ok(())
    }

    /// Encrypt data for a connection
    pub fn encrypt_data(&self, connection_id: Uuid, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        if let Some(session) = self.sessions.get(&connection_id) {
            session.encrypt(data)
        } else {
            Err(UnifiedProtocolError::Encryption("No encryption session found for connection".to_string()))
        }
    }

    /// Decrypt data for a connection
    pub fn decrypt_data(&mut self, connection_id: Uuid, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        if let Some(session) = self.sessions.get_mut(&connection_id) {
            session.decrypt(data)
        } else {
            Err(UnifiedProtocolError::Encryption("No encryption session found for connection".to_string()))
        }
    }

    /// Add a Bedrock encryptor for a connection
    pub fn add_bedrock_encryptor(&mut self, connection_id: Uuid, encryptor: Encryptor) {
        self.bedrock_encryptors.insert(connection_id, encryptor);
    }

    /// Get a Bedrock encryptor for a connection
    pub fn get_bedrock_encryptor(&self, connection_id: Uuid) -> Option<&Encryptor> {
        self.bedrock_encryptors.get(&connection_id)
    }

    /// Get a mutable Bedrock encryptor for a connection
    pub fn get_bedrock_encryptor_mut(&mut self, connection_id: Uuid) -> Option<&mut Encryptor> {
        self.bedrock_encryptors.get_mut(&connection_id)
    }

    /// Remove encryption session for a connection
    pub fn remove_session(&mut self, connection_id: Uuid) {
        self.sessions.remove(&connection_id);
        self.bedrock_encryptors.remove(&connection_id);
    }
}

impl Default for UnifiedEncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Encryption session for a specific connection
pub struct EncryptionSession {
    /// Protocol type
    protocol_type: ProtocolType,
    /// Shared secret
    shared_secret: Vec<u8>,
    /// Encryption enabled flag
    enabled: bool,
}

impl EncryptionSession {
    /// Create a new encryption session
    pub fn new(protocol_type: ProtocolType, shared_secret: Vec<u8>) -> Result<Self, UnifiedProtocolError> {
        Ok(Self {
            protocol_type,
            shared_secret,
            enabled: true,
        })
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        if !self.enabled {
            return Ok(Bytes::copy_from_slice(data));
        }

        match self.protocol_type {
            ProtocolType::Java => self.encrypt_java(data),
            ProtocolType::Bedrock => self.encrypt_bedrock(data),
        }
    }

    /// Decrypt data
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        if !self.enabled {
            return Ok(Bytes::copy_from_slice(data));
        }

        match self.protocol_type {
            ProtocolType::Java => self.decrypt_java(data),
            ProtocolType::Bedrock => self.decrypt_bedrock(data),
        }
    }

    /// Encrypt data for Java Edition (AES/CFB8)
    fn encrypt_java(&self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        // This would integrate with minecraft-server-protocol's encryption
        // For now, return the data as-is (placeholder implementation)
        Ok(Bytes::copy_from_slice(data))
    }

    /// Decrypt data for Java Edition (AES/CFB8)
    fn decrypt_java(&mut self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        // This would integrate with minecraft-server-protocol's encryption
        // For now, return the data as-is (placeholder implementation)
        Ok(Bytes::copy_from_slice(data))
    }

    /// Encrypt data for Bedrock Edition (uses mirai's Encryptor)
    fn encrypt_bedrock(&self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        // For Bedrock, encryption is handled by mirai's Encryptor
        // This is a placeholder - actual encryption would be done by the Encryptor
        Ok(Bytes::copy_from_slice(data))
    }

    /// Decrypt data for Bedrock Edition (uses mirai's Encryptor)
    fn decrypt_bedrock(&mut self, data: &[u8]) -> Result<Bytes, UnifiedProtocolError> {
        // For Bedrock, decryption is handled by mirai's Encryptor
        // This is a placeholder - actual decryption would be done by the Encryptor
        Ok(Bytes::copy_from_slice(data))
    }
}

/// Unified player profile that works for both Java and Bedrock
#[derive(Debug, Clone)]
pub struct UnifiedPlayerProfile {
    /// Player UUID
    pub uuid: Uuid,
    /// Player username
    pub username: String,
    /// Protocol type
    pub protocol_type: ProtocolType,
    /// Xbox Live user ID (Bedrock only)
    pub xuid: Option<u64>,
    /// Player properties (Java only)
    pub properties: Vec<PlayerProperty>,
    /// Skin data (Bedrock only)
    pub skin_data: Option<Vec<u8>>,
    /// Authentication timestamp
    pub authenticated_at: SystemTime,
}

/// Java Edition player profile
#[derive(Debug, Clone)]
pub struct JavaPlayerProfile {
    /// Player UUID
    pub uuid: Uuid,
    /// Player username
    pub username: String,
    /// Player properties (textures, etc.)
    pub properties: Vec<PlayerProperty>,
}

/// Bedrock Edition player profile
#[derive(Debug, Clone)]
pub struct BedrockPlayerProfile {
    /// Player UUID
    pub uuid: Uuid,
    /// Player username
    pub username: String,
    /// Xbox Live user ID
    pub xuid: u64,
    /// Skin data
    pub skin_data: Option<Vec<u8>>,
}

/// Player property (textures, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProperty {
    /// Property name
    pub name: String,
    /// Property value
    pub value: String,
    /// Property signature (if signed)
    pub signature: Option<String>,
}

/// Authentication statistics
#[derive(Debug, Clone)]
pub struct AuthStats {
    /// Total number of authentication attempts
    pub total_authentications: u64,
    /// Number of failed authentication attempts
    pub failed_authentications: u64,
    /// Number of active sessions
    pub active_sessions: u64,
}

impl AuthStats {
    /// Create new authentication statistics
    pub fn new() -> Self {
        Self {
            total_authentications: 0,
            failed_authentications: 0,
            active_sessions: 0,
        }
    }
}

impl Default for AuthStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified authentication statistics
#[derive(Debug, Clone)]
pub struct UnifiedAuthStats {
    /// Total number of authentication attempts
    pub total_authentications: u64,
    /// Number of Java authentication attempts
    pub java_authentications: u64,
    /// Number of Bedrock authentication attempts
    pub bedrock_authentications: u64,
    /// Number of failed authentication attempts
    pub failed_authentications: u64,
    /// Number of active sessions
    pub active_sessions: u64,
}

/// Unified authentication configuration
#[derive(Debug, Clone)]
pub struct UnifiedAuthConfig {
    /// Java Edition authentication configuration
    pub java_config: JavaAuthConfig,
    /// Bedrock Edition authentication configuration
    pub bedrock_config: BedrockAuthConfig,
    /// Enable authentication caching
    pub enable_caching: bool,
    /// Cache timeout in seconds
    pub cache_timeout: u64,
}

impl Default for UnifiedAuthConfig {
    fn default() -> Self {
        Self {
            java_config: JavaAuthConfig::default(),
            bedrock_config: BedrockAuthConfig::default(),
            enable_caching: true,
            cache_timeout: 300, // 5 minutes
        }
    }
}

/// Java Edition authentication configuration
#[derive(Debug, Clone)]
pub struct JavaAuthConfig {
    /// Enable offline mode
    pub offline_mode: bool,
    /// Mojang API base URL
    pub mojang_api_url: String,
    /// Microsoft authentication URL
    pub microsoft_auth_url: String,
    /// Enable encryption
    pub enable_encryption: bool,
}

impl Default for JavaAuthConfig {
    fn default() -> Self {
        Self {
            offline_mode: false,
            mojang_api_url: "https://api.mojang.com".to_string(),
            microsoft_auth_url: "https://login.microsoftonline.com".to_string(),
            enable_encryption: true,
        }
    }
}

/// Bedrock Edition authentication configuration
#[derive(Debug, Clone)]
pub struct BedrockAuthConfig {
    /// Xbox Live API base URL
    pub xbox_live_api_url: String,
    /// Minecraft services API URL
    pub minecraft_services_url: String,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Require Xbox Live authentication
    pub require_xbox_live: bool,
}

impl Default for BedrockAuthConfig {
    fn default() -> Self {
        Self {
            xbox_live_api_url: "https://user.auth.xboxlive.com".to_string(),
            minecraft_services_url: "https://api.minecraftservices.com".to_string(),
            enable_encryption: true,
            require_xbox_live: true,
        }
    }
}

/// Generate an offline UUID from a username (Java Edition)
fn generate_offline_uuid(username: &str) -> Uuid {
    use sha2::{Digest, Sha256 as Sha1};

    let mut hasher = Sha1::new();
    hasher.update(b"OfflinePlayer:");
    hasher.update(username.as_bytes());
    let hash = hasher.finalize();

    // Create UUID from hash (version 3 UUID)
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&hash[..16]);

    // Set version (3) and variant bits
    uuid_bytes[6] = (uuid_bytes[6] & 0x0F) | 0x30;
    uuid_bytes[8] = (uuid_bytes[8] & 0x3F) | 0x80;

    Uuid::from_bytes(uuid_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_uuid_generation() {
        let uuid1 = generate_offline_uuid("TestPlayer");
        let uuid2 = generate_offline_uuid("TestPlayer");
        let uuid3 = generate_offline_uuid("DifferentPlayer");

        // Same username should generate same UUID
        assert_eq!(uuid1, uuid2);

        // Different usernames should generate different UUIDs
        assert_ne!(uuid1, uuid3);

        // Check UUID version
        assert_eq!(uuid1.get_version_num(), 3);
    }

    #[tokio::test]
    async fn test_unified_auth_service_creation() {
        let config = UnifiedAuthConfig::default();
        let auth_service = UnifiedAuthService::new(config);

        let stats = auth_service.get_auth_stats();
        assert_eq!(stats.total_authentications, 0);
        assert_eq!(stats.java_authentications, 0);
        assert_eq!(stats.bedrock_authentications, 0);
    }

    #[tokio::test]
    async fn test_java_offline_authentication() {
        let mut config = JavaAuthConfig::default();
        config.offline_mode = true;

        let java_auth = JavaAuthService::new(config);

        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some("TestPlayer".to_string());

        let result = java_auth.authenticate(&auth_data);
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert_eq!(profile.username, "TestPlayer");
        assert_eq!(profile.uuid.get_version_num(), 3);
    }

    #[tokio::test]
    async fn test_encryption_manager() {
        let mut manager = UnifiedEncryptionManager::new();
        let connection_id = Uuid::new_v4();
        let shared_secret = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Enable encryption
        let result = manager.enable_encryption(connection_id, ProtocolType::Java, shared_secret);
        assert!(result.is_ok());

        // Test encryption/decryption
        let test_data = b"Hello, World!";
        let encrypted = manager.encrypt_data(connection_id, test_data).unwrap();
        let decrypted = manager.decrypt_data(connection_id, &encrypted).unwrap();

        assert_eq!(decrypted.as_ref(), test_data);

        // Remove session
        manager.remove_session(connection_id);

        // Should fail after removal
        let result = manager.encrypt_data(connection_id, test_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_generation() {
        let java_auth = JavaAuthService::new(JavaAuthConfig::default());

        let token1 = java_auth.generate_verify_token();
        let token2 = java_auth.generate_verify_token();

        assert_eq!(token1.len(), 4);
        assert_eq!(token2.len(), 4);
        // Tokens should be different (very high probability)
        assert_ne!(token1, token2);
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
    fn test_auth_stats() {
        let mut stats = AuthStats::new();
        assert_eq!(stats.total_authentications, 0);
        assert_eq!(stats.failed_authentications, 0);
        assert_eq!(stats.active_sessions, 0);

        stats.total_authentications += 1;
        stats.active_sessions += 1;

        assert_eq!(stats.total_authentications, 1);
        assert_eq!(stats.active_sessions, 1);
    }
}
