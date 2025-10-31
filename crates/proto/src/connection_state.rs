//! Unified connection state management with integrated authentication and encryption
//! 
//! This module provides comprehensive connection state management that handles
//! authentication, encryption, and protocol-specific state transitions for both
//! Java and Bedrock connections.

use crate::unified_auth::{
    UnifiedAuthService, UnifiedPlayerProfile, UnifiedAuthConfig, 
    ProtocolType, UnifiedProtocolError, UnifiedAuthData
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, Duration};
use uuid::Uuid;

/// Unified connection state that works for both Java and Bedrock protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedConnectionState {
    /// Connection is being established
    Connecting,
    /// Handshaking phase (Java) or initial connection (Bedrock)
    Handshaking,
    /// Status request phase (Java only)
    Status,
    /// Login/authentication phase
    Login,
    /// Configuration phase (Java 1.20.2+)
    Configuration,
    /// Resource pack negotiation (Bedrock)
    ResourcePacks,
    /// Main gameplay phase
    Play,
    /// Connection is being closed
    Disconnecting,
    /// Connection is closed
    Disconnected,
}

impl UnifiedConnectionState {
    /// Check if the connection is active
    pub fn is_active(&self) -> bool {
        matches!(
            *self,
            Self::Connecting | Self::Handshaking | Self::Status | Self::Login | 
            Self::Configuration | Self::ResourcePacks | Self::Play
        )
    }
    
    /// Check if the connection is closed
    pub fn is_closed(self) -> bool {
        matches!(self, Self::Disconnected)
    }
}

/// Unified connection state manager that handles authentication and encryption
pub struct UnifiedConnectionStateManager {
    /// Authentication service
    auth_service: Arc<UnifiedAuthService>,
    /// Connection states
    connection_states: Arc<RwLock<HashMap<Uuid, ConnectionStateInfo>>>,
    /// Configuration
    config: ConnectionStateConfig,
}

impl UnifiedConnectionStateManager {
    /// Create a new unified connection state manager
    pub fn new(auth_config: UnifiedAuthConfig, state_config: ConnectionStateConfig) -> Self {
        Self {
            auth_service: Arc::new(UnifiedAuthService::new(auth_config)),
            connection_states: Arc::new(RwLock::new(HashMap::new())),
            config: state_config,
        }
    }
    
    /// Initialize a new connection
    pub async fn initialize_connection(
        &self,
        connection_id: Uuid,
        protocol_type: ProtocolType,
    ) -> Result<(), UnifiedProtocolError> {
        let state_info = ConnectionStateInfo::new(protocol_type);
        
        let mut states = self.connection_states.write().await;
        states.insert(connection_id, state_info);
        
        tracing::debug!("Initialized connection state for {} ({})", connection_id, 
            match protocol_type {
                ProtocolType::Java => "Java",
                ProtocolType::Bedrock => "Bedrock",
            }
        );
        
        Ok(())
    }
    
    /// Update connection state
    pub async fn update_connection_state(
        &self,
        connection_id: Uuid,
        new_state: UnifiedConnectionState,
    ) -> Result<(), UnifiedProtocolError> {
        let mut states = self.connection_states.write().await;
        
        if let Some(state_info) = states.get_mut(&connection_id) {
            let old_state = state_info.current_state;
            
            // Validate state transition
            if !self.is_valid_state_transition(old_state, new_state, state_info.protocol_type) {
                return Err(UnifiedProtocolError::InvalidPacket(
                    format!("Invalid state transition from {:?} to {:?}", old_state, new_state)
                ));
            }
            
            state_info.current_state = new_state;
            state_info.last_state_change = SystemTime::now();
            
            tracing::debug!("Connection {} state: {:?} -> {:?}", connection_id, old_state, new_state);
            
            // Handle state-specific actions
            self.handle_state_change(connection_id, old_state, new_state, state_info).await?;
        } else {
            return Err(UnifiedProtocolError::Connection(
                "Connection state not found".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Authenticate a connection
    pub async fn authenticate_connection(
        &self,
        connection_id: Uuid,
        auth_data: UnifiedAuthData,
    ) -> Result<UnifiedPlayerProfile, UnifiedProtocolError> {
        let protocol_type = {
            let states = self.connection_states.read().await;
            if let Some(state_info) = states.get(&connection_id) {
                state_info.protocol_type
            } else {
                return Err(UnifiedProtocolError::Connection(
                    "Connection state not found".to_string()
                ));
            }
        };
        
        // Authenticate the player
        let profile = self.auth_service.authenticate_player(protocol_type, &auth_data)?;
        
        // Update connection state with authentication info
        {
            let mut states = self.connection_states.write().await;
            if let Some(state_info) = states.get_mut(&connection_id) {
                state_info.player_profile = Some(profile.clone());
                state_info.authenticated_at = Some(SystemTime::now());
                state_info.auth_data = Some(auth_data);
            }
        }
        
        tracing::info!("Authenticated connection {} as {} ({})", 
            connection_id, profile.username, profile.uuid);
        
        Ok(profile)
    }
    
    /// Enable encryption for a connection
    pub async fn enable_encryption(
        &self,
        connection_id: Uuid,
        shared_secret: Vec<u8>,
    ) -> Result<(), UnifiedProtocolError> {
        let protocol_type = {
            let states = self.connection_states.read().await;
            if let Some(state_info) = states.get(&connection_id) {
                state_info.protocol_type
            } else {
                return Err(UnifiedProtocolError::Connection(
                    "Connection state not found".to_string()
                ));
            }
        };
        
        // Enable encryption through auth service
        self.auth_service.enable_encryption(connection_id, protocol_type, shared_secret)?;
        
        // Update connection state
        {
            let mut states = self.connection_states.write().await;
            if let Some(state_info) = states.get_mut(&connection_id) {
                state_info.encryption_enabled = true;
                state_info.encryption_enabled_at = Some(SystemTime::now());
            }
        }
        
        tracing::debug!("Enabled encryption for connection {}", connection_id);
        Ok(())
    }
    
    /// Get connection state information
    pub async fn get_connection_state(&self, connection_id: Uuid) -> Option<ConnectionStateInfo> {
        let states = self.connection_states.read().await;
        states.get(&connection_id).cloned()
    }
    
    /// Get all connection states
    pub async fn get_all_connection_states(&self) -> HashMap<Uuid, ConnectionStateInfo> {
        let states = self.connection_states.read().await;
        states.clone()
    }
    
    /// Remove connection state
    pub async fn remove_connection_state(&self, connection_id: Uuid) -> Option<ConnectionStateInfo> {
        let mut states = self.connection_states.write().await;
        let state_info = states.remove(&connection_id);
        
        if state_info.is_some() {
            tracing::debug!("Removed connection state for {}", connection_id);
        }
        
        state_info
    }
    
    /// Check if a connection is authenticated
    pub async fn is_authenticated(&self, connection_id: Uuid) -> bool {
        let states = self.connection_states.read().await;
        if let Some(state_info) = states.get(&connection_id) {
            state_info.player_profile.is_some()
        } else {
            false
        }
    }
    
    /// Check if a connection has encryption enabled
    pub async fn is_encryption_enabled(&self, connection_id: Uuid) -> bool {
        let states = self.connection_states.read().await;
        if let Some(state_info) = states.get(&connection_id) {
            state_info.encryption_enabled
        } else {
            false
        }
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStateStats {
        let states = self.connection_states.read().await;
        
        let mut stats = ConnectionStateStats::new();
        
        for state_info in states.values() {
            stats.total_connections += 1;
            
            match state_info.protocol_type {
                ProtocolType::Java => stats.java_connections += 1,
                ProtocolType::Bedrock => stats.bedrock_connections += 1,
            }
            
            match state_info.current_state {
                UnifiedConnectionState::Connecting => stats.connecting_connections += 1,
                UnifiedConnectionState::Handshaking => stats.handshaking_connections += 1,
                UnifiedConnectionState::Login => stats.login_connections += 1,
                UnifiedConnectionState::Play => stats.play_connections += 1,
                _ => {}
            }
            
            if state_info.player_profile.is_some() {
                stats.authenticated_connections += 1;
            }
            
            if state_info.encryption_enabled {
                stats.encrypted_connections += 1;
            }
        }
        
        stats
    }
    
    /// Cleanup expired connections
    pub async fn cleanup_expired_connections(&self) -> usize {
        let mut states = self.connection_states.write().await;
        let mut to_remove = Vec::new();
        
        let now = SystemTime::now();
        let timeout = Duration::from_secs(self.config.connection_timeout);
        
        for (connection_id, state_info) in states.iter() {
            if let Ok(elapsed) = now.duration_since(state_info.last_activity) {
                if elapsed > timeout {
                    to_remove.push(*connection_id);
                }
            }
        }
        
        for connection_id in &to_remove {
            states.remove(connection_id);
        }
        
        let cleaned_up = to_remove.len();
        if cleaned_up > 0 {
            tracing::info!("Cleaned up {} expired connection states", cleaned_up);
        }
        
        cleaned_up
    }
    
    /// Validate state transition
    fn is_valid_state_transition(
        &self,
        from: UnifiedConnectionState,
        to: UnifiedConnectionState,
        protocol_type: ProtocolType,
    ) -> bool {
        use UnifiedConnectionState::*;
        
        match protocol_type {
            ProtocolType::Java => {
                match (from, to) {
                    (Connecting, Handshaking) => true,
                    (Handshaking, Status) => true,
                    (Handshaking, Login) => true,
                    (Status, Handshaking) => true,
                    (Login, Configuration) => true,
                    (Login, Play) => true,
                    (Configuration, Play) => true,
                    (_, Disconnecting) => true,
                    (Disconnecting, Disconnected) => true,
                    _ => false,
                }
            }
            ProtocolType::Bedrock => {
                match (from, to) {
                    (Connecting, Handshaking) => true,
                    (Handshaking, Login) => true,
                    (Login, Play) => true,
                    (_, Disconnecting) => true,
                    (Disconnecting, Disconnected) => true,
                    _ => false,
                }
            }
        }
    }
    
    /// Handle state change actions
    async fn handle_state_change(
        &self,
        connection_id: Uuid,
        _old_state: UnifiedConnectionState,
        new_state: UnifiedConnectionState,
        state_info: &mut ConnectionStateInfo,
    ) -> Result<(), UnifiedProtocolError> {
        // Update last activity
        state_info.last_activity = SystemTime::now();
        
        // Handle specific state transitions
        match new_state {
            UnifiedConnectionState::Login => {
                // Prepare for authentication
                state_info.login_started_at = Some(SystemTime::now());
            }
            UnifiedConnectionState::Play => {
                // Connection is now fully established
                state_info.play_started_at = Some(SystemTime::now());
                tracing::info!("Connection {} entered play state", connection_id);
            }
            UnifiedConnectionState::Disconnected => {
                // Connection is closed
                state_info.disconnected_at = Some(SystemTime::now());
                tracing::info!("Connection {} disconnected", connection_id);
            }
            _ => {}
        }
        
        Ok(())
    }
}

/// Connection state information
#[derive(Debug, Clone)]
pub struct ConnectionStateInfo {
    /// Protocol type
    pub protocol_type: ProtocolType,
    /// Current connection state
    pub current_state: UnifiedConnectionState,
    /// Player profile (if authenticated)
    pub player_profile: Option<UnifiedPlayerProfile>,
    /// Authentication data
    pub auth_data: Option<UnifiedAuthData>,
    /// Whether encryption is enabled
    pub encryption_enabled: bool,
    /// Connection creation time
    pub created_at: SystemTime,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Last state change time
    pub last_state_change: SystemTime,
    /// Authentication time
    pub authenticated_at: Option<SystemTime>,
    /// Encryption enabled time
    pub encryption_enabled_at: Option<SystemTime>,
    /// Login started time
    pub login_started_at: Option<SystemTime>,
    /// Play state started time
    pub play_started_at: Option<SystemTime>,
    /// Disconnection time
    pub disconnected_at: Option<SystemTime>,
}

impl ConnectionStateInfo {
    /// Create new connection state info
    pub fn new(protocol_type: ProtocolType) -> Self {
        let now = SystemTime::now();
        Self {
            protocol_type,
            current_state: UnifiedConnectionState::Connecting,
            player_profile: None,
            auth_data: None,
            encryption_enabled: false,
            created_at: now,
            last_activity: now,
            last_state_change: now,
            authenticated_at: None,
            encryption_enabled_at: None,
            login_started_at: None,
            play_started_at: None,
            disconnected_at: None,
        }
    }
    
    /// Get connection duration
    pub fn connection_duration(&self) -> Duration {
        self.last_activity.duration_since(self.created_at)
            .unwrap_or_default()
    }
    
    /// Get time since last activity
    pub fn time_since_last_activity(&self) -> Duration {
        SystemTime::now().duration_since(self.last_activity)
            .unwrap_or_default()
    }
    
    /// Check if connection is active
    pub fn is_active(&self) -> bool {
        self.current_state.is_active()
    }
    
    /// Check if connection is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.player_profile.is_some()
    }
}

/// Connection state statistics
#[derive(Debug, Clone)]
pub struct ConnectionStateStats {
    /// Total number of connections
    pub total_connections: u64,
    /// Number of Java connections
    pub java_connections: u64,
    /// Number of Bedrock connections
    pub bedrock_connections: u64,
    /// Number of connecting connections
    pub connecting_connections: u64,
    /// Number of handshaking connections
    pub handshaking_connections: u64,
    /// Number of login connections
    pub login_connections: u64,
    /// Number of play connections
    pub play_connections: u64,
    /// Number of authenticated connections
    pub authenticated_connections: u64,
    /// Number of encrypted connections
    pub encrypted_connections: u64,
}

impl ConnectionStateStats {
    /// Create new connection state statistics
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            java_connections: 0,
            bedrock_connections: 0,
            connecting_connections: 0,
            handshaking_connections: 0,
            login_connections: 0,
            play_connections: 0,
            authenticated_connections: 0,
            encrypted_connections: 0,
        }
    }
}

impl Default for ConnectionStateStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection state configuration
#[derive(Debug, Clone)]
pub struct ConnectionStateConfig {
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Authentication timeout in seconds
    pub auth_timeout: u64,
    /// Enable state validation
    pub enable_state_validation: bool,
    /// Enable automatic cleanup
    pub enable_auto_cleanup: bool,
    /// Cleanup interval in seconds
    pub cleanup_interval: u64,
}

impl Default for ConnectionStateConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 300, // 5 minutes
            auth_timeout: 60,        // 1 minute
            enable_state_validation: true,
            enable_auto_cleanup: true,
            cleanup_interval: 60,    // 1 minute
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified_auth::UnifiedAuthConfig;
    
    #[tokio::test]
    async fn test_connection_state_manager_creation() {
        let auth_config = UnifiedAuthConfig::default();
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.total_connections, 0);
    }
    
    #[tokio::test]
    async fn test_connection_initialization() {
        let auth_config = UnifiedAuthConfig::default();
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let connection_id = Uuid::new_v4();
        let result = manager.initialize_connection(connection_id, ProtocolType::Java).await;
        assert!(result.is_ok());
        
        let state_info = manager.get_connection_state(connection_id).await.unwrap();
        assert_eq!(state_info.protocol_type, ProtocolType::Java);
        assert_eq!(state_info.current_state, UnifiedConnectionState::Connecting);
        assert!(!state_info.is_authenticated());
        assert!(!state_info.encryption_enabled);
    }
    
    #[tokio::test]
    async fn test_state_transitions() {
        let auth_config = UnifiedAuthConfig::default();
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let connection_id = Uuid::new_v4();
        manager.initialize_connection(connection_id, ProtocolType::Java).await.unwrap();
        
        // Valid transition: Connecting -> Handshaking
        let result = manager.update_connection_state(connection_id, UnifiedConnectionState::Handshaking).await;
        assert!(result.is_ok());
        
        // Valid transition: Handshaking -> Login
        let result = manager.update_connection_state(connection_id, UnifiedConnectionState::Login).await;
        assert!(result.is_ok());
        
        // Invalid transition: Login -> Status (not valid for this flow)
        let result = manager.update_connection_state(connection_id, UnifiedConnectionState::Status).await;
        assert!(result.is_err());
        
        // Valid transition: Login -> Play
        let result = manager.update_connection_state(connection_id, UnifiedConnectionState::Play).await;
        assert!(result.is_ok());
        
        let state_info = manager.get_connection_state(connection_id).await.unwrap();
        assert_eq!(state_info.current_state, UnifiedConnectionState::Play);
        assert!(state_info.is_active());
    }
    
    #[tokio::test]
    async fn test_authentication() {
        let mut auth_config = UnifiedAuthConfig::default();
        auth_config.java_config.offline_mode = true;
        
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let connection_id = Uuid::new_v4();
        manager.initialize_connection(connection_id, ProtocolType::Java).await.unwrap();
        
        let mut auth_data = UnifiedAuthData::default();
        auth_data.username = Some("TestPlayer".to_string());
        
        let result = manager.authenticate_connection(connection_id, auth_data).await;
        assert!(result.is_ok());
        
        let profile = result.unwrap();
        assert_eq!(profile.username, "TestPlayer");
        assert_eq!(profile.protocol_type, ProtocolType::Java);
        
        // Check that connection is now authenticated
        assert!(manager.is_authenticated(connection_id).await);
        
        let state_info = manager.get_connection_state(connection_id).await.unwrap();
        assert!(state_info.is_authenticated());
        assert!(state_info.authenticated_at.is_some());
    }
    
    #[tokio::test]
    async fn test_encryption() {
        let auth_config = UnifiedAuthConfig::default();
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let connection_id = Uuid::new_v4();
        manager.initialize_connection(connection_id, ProtocolType::Java).await.unwrap();
        
        let shared_secret = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let result = manager.enable_encryption(connection_id, shared_secret).await;
        assert!(result.is_ok());
        
        // Check that encryption is enabled
        assert!(manager.is_encryption_enabled(connection_id).await);
        
        let state_info = manager.get_connection_state(connection_id).await.unwrap();
        assert!(state_info.encryption_enabled);
        assert!(state_info.encryption_enabled_at.is_some());
    }
    
    #[tokio::test]
    async fn test_connection_statistics() {
        let auth_config = UnifiedAuthConfig::default();
        let state_config = ConnectionStateConfig::default();
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        // Create multiple connections
        let java_id = Uuid::new_v4();
        let bedrock_id = Uuid::new_v4();
        
        manager.initialize_connection(java_id, ProtocolType::Java).await.unwrap();
        manager.initialize_connection(bedrock_id, ProtocolType::Bedrock).await.unwrap();
        
        // Update states
        manager.update_connection_state(java_id, UnifiedConnectionState::Handshaking).await.unwrap();
        manager.update_connection_state(bedrock_id, UnifiedConnectionState::Handshaking).await.unwrap();
        manager.update_connection_state(bedrock_id, UnifiedConnectionState::Login).await.unwrap();
        
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.total_connections, 2);
        assert_eq!(stats.java_connections, 1);
        assert_eq!(stats.bedrock_connections, 1);
        assert_eq!(stats.handshaking_connections, 1);
        assert_eq!(stats.login_connections, 1);
    }
    
    #[tokio::test]
    async fn test_connection_cleanup() {
        let auth_config = UnifiedAuthConfig::default();
        let mut state_config = ConnectionStateConfig::default();
        state_config.connection_timeout = 1; // 1 second timeout
        
        let manager = UnifiedConnectionStateManager::new(auth_config, state_config);
        
        let connection_id = Uuid::new_v4();
        manager.initialize_connection(connection_id, ProtocolType::Java).await.unwrap();
        
        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        let cleaned_up = manager.cleanup_expired_connections().await;
        assert_eq!(cleaned_up, 1);
        
        // Connection should be removed
        assert!(manager.get_connection_state(connection_id).await.is_none());
    }
    
    #[test]
    fn test_connection_state_info() {
        let state_info = ConnectionStateInfo::new(ProtocolType::Java);
        
        assert_eq!(state_info.protocol_type, ProtocolType::Java);
        assert_eq!(state_info.current_state, UnifiedConnectionState::Connecting);
        assert!(!state_info.is_authenticated());
        assert!(!state_info.encryption_enabled);
        assert!(state_info.is_active());
        
        // Duration should be very small for new state
        assert!(state_info.connection_duration().as_millis() < 100);
        assert!(state_info.time_since_last_activity().as_millis() < 100);
    }
}