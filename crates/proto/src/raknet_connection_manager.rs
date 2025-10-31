//! Comprehensive RakNet connection manager that integrates mirai's RakNet with enhanced protocol handling
//! 
//! This module provides a unified connection management system that seamlessly handles
//! RakNet protocol packets and integrates them with the enhanced Bedrock protocol system.

use crate::connection::{
    BedrockConnection, BedrockConnectionManager, BedrockConnectionState,
    BedrockAuthData, ConnectionStats, GlobalConnectionStats
};
use crate::enhanced_connection::{
    EnhancedConnectionManager, EnhancedConnectionConfig, ConnectionInfo,
    RakNetConnectionInfo, EnhancedConnectionStats
};
use crate::raknet_bridge::{
    RakNetBridge, EnhancedRakNetClient, RakNetConfig, RakNetConnectionState,
    RakNetPacketHandler
};
use crate::unified::{BedrockProtocolError, RawBedrockPacket, PacketDirection};
use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{Duration, Instant, interval};
use uuid::Uuid;

/// Comprehensive RakNet connection manager that merges mirai's RakNet functionality
/// with enhanced protocol handling from minecraft-server-protocol
pub struct RakNetConnectionManager {
    /// Enhanced connection manager for unified protocol handling
    enhanced_manager: Arc<EnhancedConnectionManager>,
    /// RakNet-specific packet processing bridge
    raknet_bridge: Arc<RwLock<RakNetBridge>>,
    /// UDP socket for RakNet communication
    udp_socket: Arc<UdpSocket>,
    /// Active RakNet sessions (address -> connection info)
    active_sessions: Arc<RwLock<HashMap<SocketAddr, RakNetSessionInfo>>>,
    /// Connection timeout tracking
    connection_timeouts: Arc<RwLock<HashMap<Uuid, Instant>>>,
    /// Manager configuration
    config: RakNetManagerConfig,
    /// Running state
    is_running: Arc<Mutex<bool>>,
    /// Statistics
    stats: Arc<RwLock<RakNetManagerStats>>,
}

impl RakNetConnectionManager {
    /// Create a new RakNet connection manager
    pub async fn new(
        bind_addr: SocketAddr,
        config: RakNetManagerConfig,
    ) -> Result<Self, BedrockProtocolError> {
        // Create UDP socket for RakNet
        let udp_socket = UdpSocket::bind(bind_addr).await
            .map_err(|e| BedrockProtocolError::Connection(
                format!("Failed to bind RakNet UDP socket to {}: {}", bind_addr, e)
            ))?;
        
        let udp_socket = Arc::new(udp_socket);
        
        // Create enhanced connection manager
        let enhanced_config = EnhancedConnectionConfig {
            max_connections: config.max_connections,
            enable_raknet: true,
            raknet_config: config.raknet_config.clone(),
            cleanup_interval: config.cleanup_interval,
            enable_stats: true,
        };
        
        let mut enhanced_manager = EnhancedConnectionManager::new(enhanced_config);
        enhanced_manager.set_bedrock_socket(udp_socket.clone());
        
        // Create RakNet bridge
        let raknet_bridge = Arc::new(RwLock::new(RakNetBridge::new()));
        
        tracing::info!("RakNet connection manager bound to {}", bind_addr);
        
        Ok(Self {
            enhanced_manager: Arc::new(enhanced_manager),
            raknet_bridge,
            udp_socket,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_timeouts: Arc::new(RwLock::new(HashMap::new())),
            config,
            is_running: Arc::new(Mutex::new(false)),
            stats: Arc::new(RwLock::new(RakNetManagerStats::new())),
        })
    }
    
    /// Start the RakNet connection manager
    pub async fn start(&self) -> Result<(), BedrockProtocolError> {
        {
            let mut running = self.is_running.lock().await;
            if *running {
                return Err(BedrockProtocolError::Connection(
                    "RakNet manager already running".to_string()
                ));
            }
            *running = true;
        }
        
        tracing::info!("Starting RakNet connection manager");
        
        // Start the main packet processing loop
        let socket = self.udp_socket.clone();
        let enhanced_manager = self.enhanced_manager.clone();
        let bridge = self.raknet_bridge.clone();
        let sessions = self.active_sessions.clone();
        let timeouts = self.connection_timeouts.clone();
        let stats = self.stats.clone();
        let running = self.is_running.clone();
        let config = self.config.clone();
        
        // Spawn packet processing task
        let _packet_task = {
            let socket = socket.clone();
            let sessions = sessions.clone();
            let timeouts = timeouts.clone();
            let stats = stats.clone();
            let running = running.clone();
            
            tokio::spawn(async move {
                Self::packet_processing_loop(
                    socket,
                    enhanced_manager,
                    bridge,
                    sessions,
                    timeouts,
                    stats,
                    running,
                    config,
                ).await;
            })
        };
        
        // Spawn cleanup task
        let _cleanup_task = {
            let sessions = sessions.clone();
            let timeouts = timeouts.clone();
            let running = running.clone();
            let cleanup_interval = Duration::from_secs(self.config.cleanup_interval);
            let connection_timeout = Duration::from_millis(self.config.raknet_config.connection_timeout);
            
            tokio::spawn(async move {
                let mut interval = interval(cleanup_interval);
                
                while *running.lock().await {
                    interval.tick().await;
                    Self::cleanup_expired_connections(
                        &sessions,
                        &timeouts,
                        connection_timeout,
                    ).await;
                }
            })
        };
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.start_time = Some(Instant::now());
        }
        
        tracing::info!("RakNet connection manager started successfully");
        Ok(())
    }
    
    /// Stop the RakNet connection manager
    pub async fn stop(&self) -> Result<(), BedrockProtocolError> {
        {
            let mut running = self.is_running.lock().await;
            if !*running {
                return Ok(());
            }
            *running = false;
        }
        
        tracing::info!("Stopping RakNet connection manager");
        
        // Shutdown enhanced manager
        self.enhanced_manager.shutdown().await;
        
        // Clear active sessions
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.clear();
        }
        
        // Clear timeouts
        {
            let mut timeouts = self.connection_timeouts.write().await;
            timeouts.clear();
        }
        
        tracing::info!("RakNet connection manager stopped");
        Ok(())
    }
    
    /// Main packet processing loop
    async fn packet_processing_loop(
        socket: Arc<UdpSocket>,
        enhanced_manager: Arc<EnhancedConnectionManager>,
        bridge: Arc<RwLock<RakNetBridge>>,
        sessions: Arc<RwLock<HashMap<SocketAddr, RakNetSessionInfo>>>,
        timeouts: Arc<RwLock<HashMap<Uuid, Instant>>>,
        stats: Arc<RwLock<RakNetManagerStats>>,
        running: Arc<Mutex<bool>>,
        config: RakNetManagerConfig,
    ) {
        let mut buffer = vec![0u8; config.max_packet_size];
        
        while *running.lock().await {
            // Receive packet with timeout
            match tokio::time::timeout(
                Duration::from_millis(100),
                socket.recv_from(&mut buffer)
            ).await {
                Ok(Ok((size, source_addr))) => {
                    // Update stats
                    {
                        let mut stats = stats.write().await;
                        stats.total_packets_received += 1;
                        stats.total_bytes_received += size as u64;
                    }
                    
                    // Process the packet
                    if let Err(e) = Self::handle_incoming_packet(
                        &buffer[..size],
                        source_addr,
                        enhanced_manager.clone(),
                        &bridge,
                        &sessions,
                        &timeouts,
                        &stats,
                    ).await {
                        tracing::error!("Failed to handle packet from {}: {}", source_addr, e);
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("UDP receive error: {}", e);
                    // Update error stats
                    {
                        let mut stats = stats.write().await;
                        stats.total_errors += 1;
                    }
                }
                Err(_) => {
                    // Timeout - continue loop
                }
            }
        }
        
        tracing::debug!("RakNet packet processing loop ended");
    }
    
    /// Handle an incoming RakNet packet
    async fn handle_incoming_packet(
        data: &[u8],
        source_addr: SocketAddr,
        enhanced_manager: Arc<EnhancedConnectionManager>,
        bridge: &Arc<RwLock<RakNetBridge>>,
        sessions: &Arc<RwLock<HashMap<SocketAddr, RakNetSessionInfo>>>,
        timeouts: &Arc<RwLock<HashMap<Uuid, Instant>>>,
        stats: &Arc<RwLock<RakNetManagerStats>>,
    ) -> Result<(), BedrockProtocolError> {
        // Update session activity
        let connection_id = {
            let mut sessions_guard = sessions.write().await;
            if let Some(session) = sessions_guard.get_mut(&source_addr) {
                session.last_activity = Instant::now();
                session.packets_received += 1;
                Some(session.connection_id)
            } else {
                None
            }
        };
        
        // Process through enhanced connection manager
        if let Some((conn_id, _packet)) = enhanced_manager.process_bedrock_data(data, source_addr).await? {
            // Update timeout tracking
            {
                let mut timeouts_guard = timeouts.write().await;
                timeouts_guard.insert(conn_id, Instant::now());
            }
            
            // Create session if it doesn't exist
            if connection_id.is_none() {
                let mut sessions_guard = sessions.write().await;
                sessions_guard.insert(source_addr, RakNetSessionInfo {
                    connection_id: conn_id,
                    address: source_addr,
                    created_at: Instant::now(),
                    last_activity: Instant::now(),
                    packets_sent: 0,
                    packets_received: 1,
                });
                
                // Update stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_connections += 1;
                    stats_guard.total_connections += 1;
                }
                
                tracing::debug!("Created new RakNet session for {} (connection: {})", source_addr, conn_id);
            }
            
            // Process the packet through RakNet bridge for any additional handling
            {
                let mut bridge_guard = bridge.write().await;
                if let Some(_processed_packet) = bridge_guard.process_raknet_packet(data, source_addr)? {
                    // Additional RakNet-specific processing could go here
                    tracing::trace!("Processed RakNet packet from {}", source_addr);
                }
            }
        }
        
        Ok(())
    }
    
    /// Send a packet to a specific connection
    pub async fn send_packet(
        &self,
        connection_id: Uuid,
        packet: RawBedrockPacket,
    ) -> Result<(), BedrockProtocolError> {
        // Send through enhanced manager
        self.enhanced_manager.send_packet(connection_id, packet).await?;
        
        // Update session stats
        if let Some(session_addr) = self.find_session_address(connection_id).await {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_addr) {
                session.packets_sent += 1;
                session.last_activity = Instant::now();
            }
        }
        
        // Update global stats
        {
            let mut stats = self.stats.write().await;
            stats.total_packets_sent += 1;
        }
        
        Ok(())
    }
    
    /// Broadcast a packet to all active connections
    pub async fn broadcast_packet(
        &self,
        packet: RawBedrockPacket,
    ) -> Result<usize, BedrockProtocolError> {
        let sent_count = self.enhanced_manager.broadcast_to_all(packet).await?;
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_packets_sent += sent_count as u64;
        }
        
        Ok(sent_count)
    }
    
    /// Get connection information
    pub async fn get_connection_info(&self, connection_id: Uuid) -> Option<ConnectionInfo> {
        self.enhanced_manager.get_connection_info(connection_id).await
    }
    
    /// Get all active connections
    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        self.enhanced_manager.get_all_connections().await
    }
    
    /// Get RakNet-specific statistics
    pub async fn get_raknet_stats(&self) -> RakNetManagerStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        
        // Add current active connections count
        let sessions = self.active_sessions.read().await;
        result.active_connections = sessions.len() as u64;
        
        result
    }
    
    /// Get enhanced connection statistics
    pub async fn get_enhanced_stats(&self) -> EnhancedConnectionStats {
        self.enhanced_manager.get_connection_stats().await
    }
    
    /// Update RakNet configuration
    pub async fn update_raknet_config(&self, config: RakNetConfig) {
        self.enhanced_manager.update_raknet_config(config).await;
    }
    
    /// Get current RakNet configuration
    pub async fn get_raknet_config(&self) -> RakNetConfig {
        self.enhanced_manager.get_raknet_config().await
    }
    
    /// Find the session address for a connection ID
    async fn find_session_address(&self, connection_id: Uuid) -> Option<SocketAddr> {
        let sessions = self.active_sessions.read().await;
        sessions.iter()
            .find(|(_, session)| session.connection_id == connection_id)
            .map(|(addr, _)| *addr)
    }
    
    /// Cleanup expired connections
    async fn cleanup_expired_connections(
        sessions: &Arc<RwLock<HashMap<SocketAddr, RakNetSessionInfo>>>,
        timeouts: &Arc<RwLock<HashMap<Uuid, Instant>>>,
        timeout_duration: Duration,
    ) {
        let now = Instant::now();
        let mut expired_sessions = Vec::new();
        let mut expired_timeouts = Vec::new();
        
        // Find expired sessions
        {
            let sessions_guard = sessions.read().await;
            for (addr, session) in sessions_guard.iter() {
                if now.duration_since(session.last_activity) > timeout_duration {
                    expired_sessions.push((*addr, session.connection_id));
                }
            }
        }
        
        // Find expired timeouts
        {
            let timeouts_guard = timeouts.read().await;
            for (conn_id, last_activity) in timeouts_guard.iter() {
                if now.duration_since(*last_activity) > timeout_duration {
                    expired_timeouts.push(*conn_id);
                }
            }
        }
        
        // Remove expired sessions
        if !expired_sessions.is_empty() {
            let mut sessions_guard = sessions.write().await;
            for (addr, _) in &expired_sessions {
                sessions_guard.remove(addr);
            }
        }
        
        // Remove expired timeouts
        if !expired_timeouts.is_empty() {
            let mut timeouts_guard = timeouts.write().await;
            for conn_id in &expired_timeouts {
                timeouts_guard.remove(conn_id);
            }
        }
        
        if !expired_sessions.is_empty() {
            tracing::debug!("Cleaned up {} expired RakNet sessions", expired_sessions.len());
        }
    }
    
    /// Check if the manager is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }
    
    /// Get the local address the manager is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, BedrockProtocolError> {
        self.udp_socket.local_addr()
            .map_err(|e| BedrockProtocolError::Connection(
                format!("Failed to get local address: {}", e)
            ))
    }
}

/// Configuration for RakNet connection manager
#[derive(Debug, Clone)]
pub struct RakNetManagerConfig {
    /// Maximum number of connections
    pub max_connections: usize,
    /// RakNet-specific configuration
    pub raknet_config: RakNetConfig,
    /// Connection cleanup interval in seconds
    pub cleanup_interval: u64,
    /// Maximum packet size for UDP
    pub max_packet_size: usize,
    /// Enable detailed logging
    pub enable_debug_logging: bool,
}

impl Default for RakNetManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            raknet_config: RakNetConfig::default(),
            cleanup_interval: 30, // 30 seconds
            max_packet_size: 65536, // Maximum UDP packet size
            enable_debug_logging: false,
        }
    }
}

/// RakNet session information
#[derive(Debug, Clone)]
pub struct RakNetSessionInfo {
    /// Associated connection ID
    pub connection_id: Uuid,
    /// Client address
    pub address: SocketAddr,
    /// Session creation time
    pub created_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
    /// Number of packets sent to this session
    pub packets_sent: u64,
    /// Number of packets received from this session
    pub packets_received: u64,
}

impl RakNetSessionInfo {
    /// Get session duration
    pub fn session_duration(&self) -> Duration {
        self.last_activity.duration_since(self.created_at)
    }
    
    /// Check if session is active within timeout
    pub fn is_active(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() < timeout
    }
}

/// RakNet manager statistics
#[derive(Debug, Clone)]
pub struct RakNetManagerStats {
    /// Manager start time
    pub start_time: Option<Instant>,
    /// Total connections created
    pub total_connections: u64,
    /// Currently active connections
    pub active_connections: u64,
    /// Total packets sent
    pub total_packets_sent: u64,
    /// Total packets received
    pub total_packets_received: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Total errors encountered
    pub total_errors: u64,
}

impl RakNetManagerStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            start_time: None,
            total_connections: 0,
            active_connections: 0,
            total_packets_sent: 0,
            total_packets_received: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_errors: 0,
        }
    }
    
    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        if let Some(start_time) = self.start_time {
            start_time.elapsed()
        } else {
            Duration::from_secs(0)
        }
    }
    
    /// Get packets per second (sent)
    pub fn packets_per_second_sent(&self) -> f64 {
        let uptime_secs = self.uptime().as_secs_f64();
        if uptime_secs > 0.0 {
            self.total_packets_sent as f64 / uptime_secs
        } else {
            0.0
        }
    }
    
    /// Get packets per second (received)
    pub fn packets_per_second_received(&self) -> f64 {
        let uptime_secs = self.uptime().as_secs_f64();
        if uptime_secs > 0.0 {
            self.total_packets_received as f64 / uptime_secs
        } else {
            0.0
        }
    }
}

impl Default for RakNetManagerStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    #[tokio::test]
    async fn test_raknet_manager_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let config = RakNetManagerConfig::default();
        
        let manager = RakNetConnectionManager::new(addr, config).await;
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        assert!(!manager.is_running().await);
        assert!(manager.local_addr().is_ok());
    }
    
    #[tokio::test]
    async fn test_raknet_manager_start_stop() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let config = RakNetManagerConfig::default();
        
        let manager = RakNetConnectionManager::new(addr, config).await.unwrap();
        
        // Start manager
        assert!(manager.start().await.is_ok());
        assert!(manager.is_running().await);
        
        // Try to start again - should fail
        assert!(manager.start().await.is_err());
        
        // Stop manager
        assert!(manager.stop().await.is_ok());
        assert!(!manager.is_running().await);
        
        // Stop again - should be ok
        assert!(manager.stop().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_raknet_manager_stats() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let config = RakNetManagerConfig::default();
        
        let manager = RakNetConnectionManager::new(addr, config).await.unwrap();
        
        let stats = manager.get_raknet_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_packets_sent, 0);
        assert_eq!(stats.total_packets_received, 0);
        
        let enhanced_stats = manager.get_enhanced_stats().await;
        assert_eq!(enhanced_stats.bedrock_connections, 0);
    }
    
    #[tokio::test]
    async fn test_raknet_config_update() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let config = RakNetManagerConfig::default();
        
        let manager = RakNetConnectionManager::new(addr, config).await.unwrap();
        
        let original_config = manager.get_raknet_config().await;
        assert_eq!(original_config.mtu, 1400);
        
        let mut new_config = original_config.clone();
        new_config.mtu = 1200;
        new_config.max_connections = 50;
        
        manager.update_raknet_config(new_config.clone()).await;
        
        let updated_config = manager.get_raknet_config().await;
        assert_eq!(updated_config.mtu, 1200);
        assert_eq!(updated_config.max_connections, 50);
    }
    
    #[test]
    fn test_raknet_session_info() {
        let now = Instant::now();
        let session = RakNetSessionInfo {
            connection_id: Uuid::new_v4(),
            address: "127.0.0.1:19132".parse().unwrap(),
            created_at: now,
            last_activity: now,
            packets_sent: 0,
            packets_received: 0,
        };
        
        assert!(session.is_active(Duration::from_secs(1)));
        assert!(session.session_duration().as_millis() < 100);
    }
    
    #[test]
    fn test_raknet_manager_config_default() {
        let config = RakNetManagerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.cleanup_interval, 30);
        assert_eq!(config.max_packet_size, 65536);
        assert!(!config.enable_debug_logging);
    }
    
    #[test]
    fn test_raknet_manager_stats_calculations() {
        let mut stats = RakNetManagerStats::new();
        stats.start_time = Some(Instant::now() - Duration::from_secs(10));
        stats.total_packets_sent = 100;
        stats.total_packets_received = 200;
        
        assert!(stats.uptime().as_secs() >= 10);
        assert!(stats.packets_per_second_sent() > 0.0);
        assert!(stats.packets_per_second_received() > 0.0);
        assert!(stats.packets_per_second_received() > stats.packets_per_second_sent());
    }
}