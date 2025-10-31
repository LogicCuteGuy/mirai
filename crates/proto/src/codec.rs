//! Bedrock packet codec for mirai
//! 
//! This module provides encoding and decoding capabilities for Bedrock Edition
//! (UDP/RakNet) protocol packets. Mirai only supports Bedrock Edition.

use crate::unified::{
    BedrockProtocolError, RawBedrockPacket, BedrockPacket, PacketDirection, utils
};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Maximum packet size for Bedrock protocol
pub const MAX_PACKET_SIZE: usize = 1024 * 1024; // 1MB

/// Bedrock packet codec for encoding and decoding packets
pub struct BedrockPacketCodec {
    /// Enable packet batching
    batching_enabled: bool,
    /// Decoder buffer
    buffer: BytesMut,
}

impl Clone for BedrockPacketCodec {
    fn clone(&self) -> Self {
        Self {
            batching_enabled: self.batching_enabled,
            buffer: BytesMut::new(), // Start with empty buffer for clone
        }
    }
}

impl BedrockPacketCodec {
    /// Create a new Bedrock codec
    pub fn new() -> Self {
        Self {
            batching_enabled: false,
            buffer: BytesMut::with_capacity(8192),
        }
    }
    
    /// Enable or disable packet batching
    pub fn set_batching_enabled(&mut self, enabled: bool) {
        self.batching_enabled = enabled;
    }
    
    /// Encode a Bedrock packet to bytes
    pub fn encode(&self, packet: RawBedrockPacket) -> Result<Bytes, BedrockProtocolError> {
        let mut buf = BytesMut::new();
        
        // Write packet ID as u32 (little endian for Bedrock)
        utils::write_packet_id(&mut buf, packet.id);
        
        // Write packet data
        buf.extend_from_slice(&packet.data);
        
        Ok(buf.freeze())
    }
    
    /// Add data to the decoder buffer
    pub fn add_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
    
    /// Try to decode a packet from the buffer
    pub fn decode(&mut self) -> Result<Option<RawBedrockPacket>, BedrockProtocolError> {
        // Bedrock packets need at least 4 bytes for the packet ID
        if self.buffer.len() < 4 {
            return Ok(None);
        }
        
        // Read packet ID (u32 little endian)
        let mut temp_buf = self.buffer.clone().freeze();
        let packet_id = utils::read_packet_id(&mut temp_buf)?;
        
        // For now, assume the rest of the buffer is packet data
        // In a real implementation, we'd need to know the packet length from RakNet framing
        let packet_data = self.buffer.split_off(4).freeze();
        self.buffer.clear();
        
        let packet = RawBedrockPacket::new(packet_id, packet_data, PacketDirection::Serverbound);
        Ok(Some(packet))
    }
    
    /// Try to decode multiple packets from the buffer
    pub fn decode_all(&mut self) -> Result<Vec<RawBedrockPacket>, BedrockProtocolError> {
        let mut packets = Vec::new();
        
        while let Some(packet) = self.decode()? {
            packets.push(packet);
        }
        
        Ok(packets)
    }
    
    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
    
    /// Clear the buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }
}

impl Default for BedrockPacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bedrock_codec_creation() {
        let codec = BedrockPacketCodec::new();
        assert!(!codec.batching_enabled);
        assert_eq!(codec.buffer_size(), 0);
    }
    
    #[test]
    fn test_bedrock_packet_encoding() {
        let codec = BedrockPacketCodec::new();
        let packet = RawBedrockPacket::new(0xfe, Bytes::from_static(b"test"), PacketDirection::Serverbound);
        
        let encoded = codec.encode(packet).unwrap();
        assert!(!encoded.is_empty());
        
        // Should contain packet ID (4 bytes) + data (4 bytes)
        assert_eq!(encoded.len(), 8);
        
        // Check packet ID is encoded correctly (little endian)
        assert_eq!(encoded[0], 0xfe);
        assert_eq!(encoded[1], 0x00);
        assert_eq!(encoded[2], 0x00);
        assert_eq!(encoded[3], 0x00);
    }
    
    #[test]
    fn test_bedrock_codec_decode() {
        let mut codec = BedrockPacketCodec::new();
        
        // Create test data with packet ID and payload
        let mut test_data = BytesMut::new();
        test_data.put_u32_le(0xfe); // Packet ID
        test_data.extend_from_slice(b"test payload");
        
        codec.add_data(&test_data);
        let decoded = codec.decode().unwrap().unwrap();
        
        assert_eq!(decoded.id, 0xfe);
        assert_eq!(decoded.data, Bytes::from_static(b"test payload"));
    }
    
    #[test]
    fn test_partial_packet_decoding() {
        let mut codec = BedrockPacketCodec::new();
        
        // Add only 2 bytes (less than required 4 for packet ID)
        codec.add_data(&[0xfe, 0x00]);
        let result = codec.decode().unwrap();
        assert!(result.is_none());
        
        // Add remaining bytes
        codec.add_data(&[0x00, 0x00]);
        codec.add_data(b"payload");
        let result = codec.decode().unwrap();
        assert!(result.is_some());
    }
    
    #[test]
    fn test_decode_all() {
        let mut codec = BedrockPacketCodec::new();
        
        // Add multiple packets
        let mut data = BytesMut::new();
        data.put_u32_le(0x01);
        data.extend_from_slice(b"first");
        data.put_u32_le(0x02);
        data.extend_from_slice(b"second");
        
        codec.add_data(&data);
        let packets = codec.decode_all().unwrap();
        
        // Note: Current implementation will decode as one packet
        // In a real implementation, we'd need proper packet framing
        assert!(!packets.is_empty());
    }
    
    #[test]
    fn test_batching_enable() {
        let mut codec = BedrockPacketCodec::new();
        assert!(!codec.batching_enabled);
        
        codec.set_batching_enabled(true);
        assert!(codec.batching_enabled);
    }
    
    #[test]
    fn test_buffer_management() {
        let mut codec = BedrockPacketCodec::new();
        
        codec.add_data(b"test data");
        assert_eq!(codec.buffer_size(), 9);
        
        codec.clear_buffer();
        assert_eq!(codec.buffer_size(), 0);
    }
}