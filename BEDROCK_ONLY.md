# Mirai - Bedrock Edition Only

Mirai is a Minecraft Bedrock Edition server implementation written in Rust. 

## Protocol Support

**Mirai only supports Bedrock Edition protocol (UDP/RakNet).**

- ✅ Bedrock Edition (Pocket Edition, Windows 10, Xbox, PlayStation, Nintendo Switch)
- ❌ Java Edition (not supported)

## Why Bedrock Only?

1. **Simplicity**: Focusing on one protocol reduces complexity and maintenance burden
2. **Cross-platform**: Bedrock Edition is the cross-platform version of Minecraft
3. **Performance**: UDP/RakNet protocol is designed for real-time gaming
4. **Modern**: Bedrock Edition represents the future direction of Minecraft

## Architecture

The protocol layer (`mirai/crates/proto/`) has been refactored to be Bedrock-only:

- `BedrockPacket` trait for all Bedrock packets
- `BedrockConnection` for UDP/RakNet connections
- `BedrockAuthService` for Xbox Live authentication
- `BedrockPacketCodec` for encoding/decoding

All Java Edition support has been removed to keep the codebase focused and maintainable.

## Migration from Unified Protocol

If you were using the previous "unified" protocol system that supported both Java and Bedrock:

- Replace `UnifiedConnection` with `BedrockConnection`
- Replace `UnifiedPacket` with `BedrockPacket`
- Replace `UnifiedAuthService` with `BedrockAuthService`
- Update imports to use Bedrock-specific types

The API is similar but simplified for Bedrock-only use cases.