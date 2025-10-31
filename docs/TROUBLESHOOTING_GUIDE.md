# Troubleshooting Guide: Common Integration Issues

This guide helps you diagnose and resolve common issues when integrating and running the unified Mirai server with ECS, plugins, and advanced features.

## Table of Contents

1. [Quick Diagnostics](#quick-diagnostics)
2. [Configuration Issues](#configuration-issues)
3. [ECS Integration Problems](#ecs-integration-problems)
4. [Plugin System Issues](#plugin-system-issues)
5. [Performance Problems](#performance-problems)
6. [Network and Connection Issues](#network-and-connection-issues)
7. [World Loading Problems](#world-loading-problems)
8. [Memory and Resource Issues](#memory-and-resource-issues)
9. [Build and Compilation Errors](#build-and-compilation-errors)
10. [Advanced Debugging Techniques](#advanced-debugging-techniques)

## Quick Diagnostics

### System Health Check

```bash
# Run comprehensive system diagnostics
mirai doctor --config unified_config.toml --verbose

# Quick health check
mirai health-check --all

# Check specific subsystems
mirai health-check --ecs --plugins --network --world
```

### Common Diagnostic Commands

```bash
# Check configuration validity
mirai validate-config --config unified_config.toml

# Test network connectivity
mirai test-network --port 19132 --timeout 10s

# Verify world integrity
mirai validate-world --world-path worlds/world

# Check plugin compatibility
mirai check-plugins --plugin-dir plugins/

# System resource check
mirai system-info --performance
```

### Log Analysis Quick Start

```bash
# Find recent errors
grep -i "error\|fail\|panic" /var/log/mirai/server.log | tail -20

# Check for performance issues
grep -i "slow\|lag\|timeout" /var/log/mirai/server.log | tail -10

# Plugin-related issues
grep -i "plugin" /var/log/mirai/server.log | grep -i "error\|fail" | tail -10

# ECS system issues
grep -i "ecs\|system\|component" /var/log/mirai/server.log | grep -i "error" | tail -10
```

## Configuration Issues

### Issue: Server Fails to Start with Configuration Error

**Symptoms:**
- Server exits immediately on startup
- Error messages about invalid configuration
- TOML parsing errors

**Diagnosis:**
```bash
# Validate configuration syntax
mirai validate-config --config unified_config.toml --verbose

# Check for common issues
mirai doctor --config unified_config.toml --check-config
```

**Common Causes and Solutions:**

#### 1. Invalid TOML Syntax

```bash
# Error: Expected '=' after key
# Problem: Missing equals sign or quotes

# Incorrect:
server_name = My Server

# Correct:
server_name = "My Server"
```

#### 2. Invalid Data Types

```toml
# Incorrect:
max_players = "50"          # String instead of integer
view_distance = 10.5        # Float instead of integer

# Correct:
max_players = 50
view_distance = 10
```

#### 3. Missing Required Sections

```toml
# Ensure all required sections are present:
[server]
[network]
[world]
[features]
```

#### 4. Invalid File Paths

```bash
# Check if paths exist
ls -la worlds/world         # World path
ls -la plugins/            # Plugin directory

# Fix permissions if needed
chmod -R 755 worlds/
chmod -R 755 plugins/
```

### Issue: Feature Flags Not Working

**Symptoms:**
- Features enabled in config but not working
- Unexpected behavior with feature toggles

**Solution:**
```toml
# Ensure features are properly configured
[features]
vanilla_mobs = true
redstone = true
world_generation = true
ecs_system = true           # Required for ECS features
plugin_system = true        # Required for plugins
performance_monitoring = true

# Check feature dependencies
[ecs]
enabled = true              # Must match features.ecs_system

[plugins]
enabled = true              # Must match features.plugin_system
```

## ECS Integration Problems

### Issue: ECS Systems Not Running

**Symptoms:**
- Systems registered but not executing
- No ECS-related log messages
- Components not being processed

**Diagnosis:**
```bash
# Check ECS system status
mirai debug ecs --list-systems --config unified_config.toml

# Verify ECS is enabled
grep -i "ecs.*enabled" unified_config.toml
```

**Solutions:**

#### 1. ECS Not Enabled

```toml
[features]
ecs_system = true           # Enable ECS feature

[ecs]
enabled = true              # Enable ECS subsystem
system_thread_count = 4     # Set appropriate thread count
```

#### 2. System Dependencies Not Met

```rust
// Check system dependencies
impl System for MySystem {
    fn dependencies(&self) -> Vec<String> {
        vec!["required_system".to_string()]  // Ensure dependencies exist
    }
}
```

#### 3. System Errors Preventing Execution

```bash
# Check for system-specific errors
grep -i "system.*error" /var/log/mirai/server.log

# Enable debug logging for ECS
export RUST_LOG=mirai::ecs=debug
```

### Issue: Component Registration Failures

**Symptoms:**
- Components not found when queried
- Type registration errors
- Component deserialization failures

**Solutions:**

#### 1. Component Not Registered

```rust
// Ensure components are properly registered
impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        // Register component types
        app.register_component::<MyComponent>();
        app.register_component::<AnotherComponent>();
        
        Ok(())
    }
}
```

#### 2. Component Serialization Issues

```rust
// Ensure components implement required traits
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MyComponent {
    pub value: i32,
}
```

### Issue: ECS Performance Problems

**Symptoms:**
- High CPU usage from ECS systems
- Slow system execution
- Memory leaks in ECS components

**Solutions:**

#### 1. Optimize System Queries

```rust
// Inefficient: Broad query
let all_entities: Vec<EntityId> = world.query::<&Component>()
    .iter()
    .map(|(id, _)| id)
    .collect();

// Efficient: Specific query with filters
let active_entities: Vec<EntityId> = world.query::<(&Component, &ActiveFlag)>()
    .iter()
    .filter(|(_, (_, flag))| flag.active)
    .map(|(id, _)| id)
    .collect();
```

#### 2. Batch Processing

```rust
impl System for OptimizedSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let entities: Vec<EntityId> = world.query::<&MyComponent>()
            .iter()
            .map(|(id, _)| id)
            .collect();
        
        // Process in batches for better performance
        for batch in entities.chunks(100) {
            self.process_batch(world, batch)?;
        }
        
        Ok(())
    }
}
```

## Plugin System Issues

### Issue: Plugins Fail to Load

**Symptoms:**
- Plugin loading errors on startup
- Plugins not appearing in plugin list
- Dynamic library loading failures

**Diagnosis:**
```bash
# Check plugin compatibility
mirai check-plugin --plugin plugins/my_plugin.so --verbose

# List loaded plugins
mirai list-plugins --config unified_config.toml

# Check plugin dependencies
ldd plugins/my_plugin.so  # Linux
otool -L plugins/my_plugin.so  # macOS
```

**Solutions:**

#### 1. Plugin Compatibility Issues

```bash
# Check plugin API version
mirai plugin-info --plugin plugins/my_plugin.so

# Update plugin if needed
# Or move to legacy directory
mkdir -p plugins/legacy
mv plugins/incompatible_plugin.so plugins/legacy/
```

#### 2. Missing Dependencies

```bash
# Install missing system libraries
# Ubuntu/Debian:
sudo apt install libssl-dev libpq-dev

# CentOS/RHEL:
sudo yum install openssl-devel postgresql-devel

# Check plugin-specific dependencies
mirai check-plugin-deps --plugin plugins/my_plugin.so
```

#### 3. Plugin Configuration Errors

```toml
[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false          # Disable in production

# Plugin-specific configuration
[plugins.my_plugin]
enabled = true
config_file = "plugins/my_plugin.toml"
```

### Issue: Plugin Performance Problems

**Symptoms:**
- High CPU usage from specific plugins
- Memory leaks in plugin code
- Slow plugin system execution

**Solutions:**

#### 1. Profile Plugin Performance

```bash
# Profile specific plugin
mirai profile plugin --plugin-name my_plugin --duration 60s

# Check plugin resource usage
mirai plugin-stats --plugin-name my_plugin
```

#### 2. Optimize Plugin Configuration

```toml
[plugins.resource_intensive_plugin]
enabled = true
update_interval = "100ms"   # Reduce update frequency
batch_size = 50            # Process in smaller batches
parallel_processing = false # Disable if causing issues
```

## Performance Problems

### Issue: Low TPS (Ticks Per Second)

**Symptoms:**
- TPS consistently below 20
- Laggy gameplay
- Delayed block updates

**Diagnosis:**
```bash
# Monitor TPS in real-time
mirai monitor tps --duration 300s

# Analyze performance bottlenecks
mirai analyze performance --log-file server.log --duration 600s
```

**Solutions:**

#### 1. Reduce Server Load

```toml
[server]
max_players = 30           # Reduce if too high
view_distance = 8          # Lower view distance
simulation_distance = 4    # Lower simulation distance

[world]
spawn_protection = 8       # Reduce protected area
```

#### 2. Optimize ECS Configuration

```toml
[ecs]
system_thread_count = 4    # Match CPU cores
batch_size = 50           # Smaller batches
parallel_systems = true    # Enable parallel processing
```

#### 3. Disable Expensive Features

```toml
[features]
performance_monitoring = false  # Disable if not needed
detailed_profiling = false     # Only enable for debugging

[monitoring]
metrics_interval = "30s"       # Less frequent monitoring
```

### Issue: High Memory Usage

**Symptoms:**
- Constantly increasing memory usage
- Out of memory errors
- System swapping

**Solutions:**

#### 1. Tune Memory Settings

```toml
[performance]
memory_limit = "4GB"       # Set appropriate limit
gc_interval = "30s"        # More frequent garbage collection

[caching]
chunk_cache_size = 500     # Reduce cache sizes
entity_cache_size = 2000
```

#### 2. Check for Memory Leaks

```bash
# Monitor memory usage over time
mirai monitor memory --duration 1800s --output memory.log

# Analyze memory patterns
mirai analyze memory --log-file memory.log
```

## Network and Connection Issues

### Issue: Players Cannot Connect

**Symptoms:**
- Connection timeouts
- "Server not responding" errors
- Network unreachable messages

**Diagnosis:**
```bash
# Test network connectivity
mirai test-network --port 19132 --external

# Check if port is open
netstat -tulpn | grep 19132
ss -tulpn | grep 19132

# Test from external network
nmap -sU -p 19132 your-server-ip
```

**Solutions:**

#### 1. Firewall Configuration

```bash
# Linux (iptables)
sudo iptables -A INPUT -p udp --dport 19132 -j ACCEPT

# Linux (ufw)
sudo ufw allow 19132/udp

# Windows
netsh advfirewall firewall add rule name="Mirai Server" dir=in action=allow protocol=UDP localport=19132
```

#### 2. Network Configuration

```toml
[network]
port = 19132
ipv4_addr = "0.0.0.0"      # Listen on all interfaces
max_clients = 100
timeout_seconds = 30

# If behind NAT/proxy
[network.nat]
external_ip = "your.external.ip"
port_forwarding = true
```

### Issue: High Network Latency

**Symptoms:**
- High ping times
- Delayed block updates
- Rubber-banding movement

**Solutions:**

#### 1. Optimize Network Settings

```toml
[network.optimization]
tcp_nodelay = true
packet_batching = true
batch_size = 20
compression_threshold = 128  # Lower threshold for better compression
```

#### 2. Check Network Infrastructure

```bash
# Test latency to server
ping your-server-ip

# Check network path
traceroute your-server-ip

# Monitor network statistics
mirai monitor network --duration 300s
```

## World Loading Problems

### Issue: World Fails to Load

**Symptoms:**
- Server crashes on world loading
- Corrupted world errors
- Missing chunk data

**Diagnosis:**
```bash
# Validate world integrity
mirai validate-world --world-path worlds/world --verbose

# Check world format
mirai world-info --world-path worlds/world

# Test world loading
mirai test-world-load --world-path worlds/world --timeout 60s
```

**Solutions:**

#### 1. World Corruption

```bash
# Attempt world repair
mirai repair-world --world-path worlds/world --backup

# If repair fails, restore from backup
cp -r worlds/backups/world_20231201_120000 worlds/world
```

#### 2. Insufficient Permissions

```bash
# Fix world directory permissions
chmod -R 755 worlds/
chown -R mirai:mirai worlds/
```

#### 3. Disk Space Issues

```bash
# Check available disk space
df -h

# Clean up old backups if needed
find worlds/backups -name "world_*" -mtime +7 -delete
```

### Issue: Slow World Loading

**Symptoms:**
- Long startup times
- Slow chunk generation
- High disk I/O during world operations

**Solutions:**

#### 1. Optimize I/O Settings

```toml
[io]
async_world_loading = true
world_save_interval = "300s"  # Less frequent saves
batch_world_saves = true
compression_level = 3         # Faster compression
```

#### 2. Use SSD Storage

```bash
# Move world to SSD if available
sudo mkdir /mnt/ssd/mirai
sudo mv worlds /mnt/ssd/mirai/
sudo ln -s /mnt/ssd/mirai/worlds worlds
```

## Memory and Resource Issues

### Issue: Memory Leaks

**Symptoms:**
- Steadily increasing memory usage
- Eventually running out of memory
- System becomes unresponsive

**Diagnosis:**
```bash
# Monitor memory usage over time
mirai monitor memory --duration 3600s --output memory_leak.log

# Check for specific leak patterns
mirai analyze memory-leaks --log-file memory_leak.log
```

**Solutions:**

#### 1. Enable Memory Debugging

```bash
# Run with memory debugging
export RUST_BACKTRACE=1
export RUST_LOG=mirai::memory=debug
mirai --config unified_config.toml
```

#### 2. Tune Garbage Collection

```toml
[performance]
gc_interval = "15s"        # More frequent GC
memory_limit = "6GB"       # Lower memory limit
gc_threshold = 0.7         # Trigger GC earlier
```

### Issue: High CPU Usage

**Symptoms:**
- CPU usage consistently above 80%
- System becomes sluggish
- High load averages

**Solutions:**

#### 1. Reduce Thread Count

```toml
[ecs]
system_thread_count = 2    # Reduce thread count
parallel_systems = false   # Disable parallel processing

[performance]
thread_pool_size = 2
worker_threads = 1
```

#### 2. Optimize System Scheduling

```bash
# Set CPU affinity (Linux)
taskset -c 0,1 mirai --config unified_config.toml

# Lower process priority
nice -n 10 mirai --config unified_config.toml
```

## Build and Compilation Errors

### Issue: Compilation Failures

**Symptoms:**
- Build errors during compilation
- Missing dependency errors
- Linker failures

**Solutions:**

#### 1. Update Rust Toolchain

```bash
# Update Rust
rustup update stable

# Check version
rustc --version
cargo --version
```

#### 2. Install Missing Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# macOS
xcode-select --install
brew install openssl
```

#### 3. Clean Build

```bash
# Clean previous build artifacts
cargo clean

# Rebuild from scratch
cargo build --release --features "unified,ecs,plugins,monitoring"
```

### Issue: Feature Flag Conflicts

**Symptoms:**
- Conflicting feature definitions
- Undefined symbols during linking
- Runtime feature detection failures

**Solutions:**

#### 1. Check Feature Combinations

```bash
# List available features
cargo metadata --format-version 1 | jq '.packages[0].features'

# Build with specific features
cargo build --release --features "unified,ecs" --no-default-features
```

#### 2. Resolve Feature Conflicts

```toml
# Cargo.toml - Ensure compatible feature combinations
[features]
default = ["unified"]
unified = ["ecs", "plugins", "monitoring"]
ecs = []
plugins = ["ecs"]  # Plugins depend on ECS
monitoring = []
```

## Advanced Debugging Techniques

### Debug Logging Configuration

```bash
# Enable comprehensive debug logging
export RUST_LOG=mirai=debug,mirai::ecs=trace,mirai::plugins=debug

# Log to file
export RUST_LOG=debug
mirai --config unified_config.toml 2>&1 | tee debug.log
```

### Performance Profiling

```bash
# CPU profiling with perf (Linux)
perf record -g mirai --config unified_config.toml
perf report

# Memory profiling with valgrind
valgrind --tool=massif mirai --config unified_config.toml

# Custom profiling
mirai profile --all --duration 300s --output profile_report.json
```

### Network Debugging

```bash
# Packet capture
sudo tcpdump -i any -w mirai_packets.pcap port 19132

# Network analysis
mirai analyze network --pcap-file mirai_packets.pcap

# Connection debugging
mirai debug connections --show-all --real-time
```

### ECS System Debugging

```rust
// Add debug information to systems
impl System for DebugSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let start_time = Instant::now();
        
        // Your system logic here
        
        let duration = start_time.elapsed();
        if duration > Duration::from_millis(5) {
            tracing::debug!("System {} took {:?}", self.name(), duration);
        }
        
        Ok(())
    }
}
```

### Plugin Debugging

```bash
# Debug specific plugin
mirai debug plugin --plugin-name my_plugin --verbose

# Plugin system state
mirai debug plugin-system --show-state --show-dependencies

# Plugin performance analysis
mirai analyze plugin-performance --plugin-name my_plugin --duration 300s
```

### Emergency Recovery

```bash
#!/bin/bash
# emergency_recovery.sh

echo "=== Mirai Emergency Recovery ==="

# Stop server
echo "Stopping server..."
pkill -TERM mirai

# Check system resources
echo "System resources:"
free -h
df -h

# Check for core dumps
if ls core.* 1> /dev/null 2>&1; then
    echo "Core dumps found - analyzing..."
    gdb mirai core.* -batch -ex "bt" -ex "quit"
fi

# Restore from backup
echo "Available backups:"
ls -la backups/

echo "To restore: cp -r backups/latest/* ."
echo "To start in safe mode: mirai --config minimal_config.toml --safe-mode"
```

### Getting Help

When reporting issues, include:

1. **System Information:**
   ```bash
   mirai system-info --full > system_info.txt
   ```

2. **Configuration:**
   ```bash
   mirai validate-config --config unified_config.toml --export > config_info.txt
   ```

3. **Logs:**
   ```bash
   tail -1000 /var/log/mirai/server.log > recent_logs.txt
   ```

4. **Performance Data:**
   ```bash
   mirai analyze performance --duration 300s --export > performance_data.json
   ```

This troubleshooting guide covers the most common issues encountered when running the unified Mirai server. For additional help, consult the documentation or reach out to the community with the diagnostic information collected using the commands above.